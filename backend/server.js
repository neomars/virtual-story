
const express = require('express');
const cors = require('cors');
const path = require('path');
const fs = require('fs').promises; // Use the promise-based version of fs
const multer = require('multer');
const ffmpeg = require('fluent-ffmpeg');
const { dbPool } = require('./db'); // Importation du pool de connexions

const app = express();
const PORT = 3000;

// --- Middleware ---
app.use(cors());
app.use(express.json());

// --- Static File Serving ---
const videosDir = path.join(__dirname, 'uploads/videos');
const thumbnailsDir = path.join(__dirname, 'uploads/thumbnails');

(async () => {
    try {
        await fs.mkdir(videosDir, { recursive: true });
        await fs.mkdir(thumbnailsDir, { recursive: true });
    } catch (error) {
        console.error("Error creating upload directories:", error);
    }
})();

app.use('/videos', express.static(videosDir));
app.use('/thumbnails', express.static(thumbnailsDir));

// --- Multer Configuration for Video Uploads ---
const storage = multer.diskStorage({
  destination: (req, file, cb) => {
    cb(null, videosDir);
  },
  filename: (req, file, cb) => {
    const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
    cb(null, file.fieldname + '-' + uniqueSuffix + path.extname(file.originalname));
  }
});

const upload = multer({ storage: storage });

// --- API Endpoints ---

// Upload a new scene (video)
app.post('/api/scenes', upload.single('video'), async (req, res) => {
  if (!req.file) {
    return res.status(400).send({ message: 'No video file was uploaded.' });
  }

  const { title } = req.body;
  const videoPath = req.file.path;
  const videoFilename = req.file.filename;
  const thumbnailFilename = `thumb-${path.parse(videoFilename).name}.png`;
  const thumbnailPath = path.join(thumbnailsDir, thumbnailFilename);

  // Generate thumbnail
  ffmpeg(videoPath)
    .on('end', async () => {
      try {
        const videoUrl = `/videos/${videoFilename}`;
        const thumbnailUrl = `/thumbnails/${thumbnailFilename}`;

        const [result] = await dbPool.execute(
          'INSERT INTO scenes (title, video_path, thumbnail_path) VALUES (?, ?, ?)',
          [title, videoUrl, thumbnailUrl]
        );

        res.status(201).send({
          id: result.insertId,
          title,
          video_path: videoUrl,
          thumbnail_path: thumbnailUrl
        });
      } catch (dbError) {
        console.error('Database error:', dbError);
        res.status(500).send({ message: 'Failed to save scene to database.' });
      }
    })
    .on('error', (err) => {
      console.error('FFmpeg error:', err);
      res.status(500).send({ message: 'Failed to generate thumbnail.' });
    })
    .screenshots({
      timestamps: ['00:00:05.000'],
      filename: thumbnailFilename,
      folder: thumbnailsDir,
      size: '320x240'
    });
});

// Get all scenes
app.get('/api/scenes', async (req, res) => {
  try {
    const [rows] = await dbPool.execute('SELECT * FROM scenes ORDER BY created_at DESC');
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scenes.' });
  }
});

// Get a single scene by ID
app.get('/api/scenes/:id', async (req, res) => {
  const { id } = req.params;
  console.log(`[Debug] Requête reçue pour charger la scène ID: ${id}`); // Log de débogage
  try {
    const [rows] = await dbPool.execute('SELECT * FROM scenes WHERE id = ?', [id]);
    if (rows.length === 0) {
      return res.status(404).send({ message: 'Scene not found.' });
    }
    res.send(rows[0]);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene.' });
  }
});

// Update a scene's title
app.put('/api/scenes/:id', async (req, res) => {
  const { id } = req.params;
  const { title } = req.body;

  try {
    await dbPool.execute('UPDATE scenes SET title = ? WHERE id = ?', [title, id]);
    res.send({ message: 'Scene updated successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to update scene.' });
  }
});

// Delete a scene
app.delete('/api/scenes/:id', async (req, res) => {
  const { id } = req.params;

  try {
    // First, retrieve the scene to get file paths
    const [rows] = await dbPool.execute('SELECT * FROM scenes WHERE id = ?', [id]);
    if (rows.length === 0) {
      return res.status(404).send({ message: 'Scene not found.' });
    }
    const scene = rows[0];

    // Delete video and thumbnail files
    const videoPath = path.join(__dirname, 'uploads', scene.video_path);
    const thumbnailPath = path.join(__dirname, 'uploads', scene.thumbnail_path);

    await fs.unlink(videoPath).catch(err => console.error("Error deleting video file:", err));
    await fs.unlink(thumbnailPath).catch(err => console.error("Error deleting thumbnail file:", err));


    // Delete the scene from the database
    await dbPool.execute('DELETE FROM scenes WHERE id = ?', [id]);

    res.send({ message: 'Scene deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete scene.' });
  }
});

// Get choices for a scene
app.get('/api/scenes/:id/choices', async (req, res) => {
  const { id } = req.params;

  try {
    const [rows] = await dbPool.execute(
      `SELECT c.id, c.choice_text, s.id as destination_id, s.title as destination_title
       FROM choices c
       JOIN scenes s ON c.destination_scene_id = s.id
       WHERE c.source_scene_id = ?`,
      [id]
    );
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve choices.' });
  }
});

// Add a choice to a scene
app.post('/api/scenes/:id/choices', async (req, res) => {
  const { id: source_scene_id } = req.params;
  const { destination_scene_id, choice_text } = req.body;

  try {
    const [result] = await dbPool.execute(
      'INSERT INTO choices (source_scene_id, destination_scene_id, choice_text) VALUES (?, ?, ?)',
      [source_scene_id, destination_scene_id, choice_text]
    );
    res.status(201).send({ id: result.insertId });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to add choice.' });
  }
});

// Delete a choice
app.delete('/api/choices/:id', async (req, res) => {
  const { id } = req.params;

  try {
    await dbPool.execute('DELETE FROM choices WHERE id = ?', [id]);
    res.send({ message: 'Choice deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete choice.' });
  }
});


// --- Player API Endpoint ---
app.get('/api/player/scenes/:id', async (req, res) => {
  const { id } = req.params;
  const { previous_scene_id } = req.query;

  try {
    // 1. Get current scene
    const [sceneRows] = await dbPool.execute('SELECT * FROM scenes WHERE id = ?', [id]);
    if (sceneRows.length === 0) {
      return res.status(404).send({ message: 'Scene not found.' });
    }
    const currentScene = sceneRows[0];

    // 2. Get all parent scenes (scenes that can lead to this one)
    const [parentScenesRows] = await dbPool.execute(
      `SELECT s.id, s.title
       FROM scenes s
       JOIN choices c ON s.id = c.source_scene_id
       WHERE c.destination_scene_id = ?`,
      [id]
    );

    // 3. Get next choices
    const [choicesRows] = await dbPool.execute(
      `SELECT c.id, c.choice_text, s.id as destination_scene_id, s.title as destination_scene_title
       FROM choices c
       JOIN scenes s ON c.destination_scene_id = s.id
       WHERE c.source_scene_id = ?`,
      [id]
    );

    // 4. Combine and send the response
    const response = {
      current_scene: {
        id: currentScene.id,
        title: currentScene.title,
        video_path: currentScene.video_path,
        thumbnail_path: currentScene.thumbnail_path,
      },
      parent_scenes: parentScenesRows,
      next_choices: choicesRows,
    };

    res.send(response);

  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene data for the player.' });
  }
});


// --- Admin Relations API Endpoint ---
app.get('/api/admin/scenes/:id/relations', async (req, res) => {
  const { id } = req.params;

  try {
    // 1. Get current scene
    const [sceneRows] = await dbPool.execute('SELECT * FROM scenes WHERE id = ?', [id]);
    if (sceneRows.length === 0) {
      return res.status(404).send({ message: 'Scene not found.' });
    }
    const currentScene = sceneRows[0];

    // 2. Get parent scenes
    const [parentScenesRows] = await dbPool.execute(
      `SELECT s.id, s.title
       FROM scenes s
       JOIN choices c ON s.id = c.source_scene_id
       WHERE c.destination_scene_id = ?`,
      [id]
    );

    // 3. Get child scenes (next choices)
    const [childScenesRows] = await dbPool.execute(
      `SELECT s.id, s.title, c.choice_text
       FROM scenes s
       JOIN choices c ON s.id = c.destination_scene_id
       WHERE c.source_scene_id = ?`,
      [id]
    );

    res.send({
      current_scene: currentScene,
      parent_scenes: parentScenesRows,
      child_scenes: childScenesRows,
    });

  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene relations.' });
  }
});


// --- Basic Test Route ---
app.get('/', (req, res) => {
  res.send('Backend server is running!');
});


// --- Server Startup ---
app.listen(PORT, () => {
  console.log(`Server is running on http://localhost:${PORT}`);
});
