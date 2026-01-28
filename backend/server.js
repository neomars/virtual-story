
const express = require('express');
const cors = require('cors');
const path = require('path');
const fs = require('fs').promises;
const multer = require('multer');
const ffmpeg = require('fluent-ffmpeg');
const { dbPool } = require('./db');

const app = express();
const PORT = 3000;

app.use(cors());
app.use(express.json());

const videosDir = path.join(__dirname, 'uploads/videos');
const thumbnailsDir = path.join(__dirname, 'uploads/thumbnails');
const backgroundsDir = path.join(__dirname, 'uploads');

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
app.use('/backgrounds', express.static(backgroundsDir));

const storage = multer.diskStorage({
  destination: (req, file, cb) => cb(null, videosDir),
  filename: (req, file, cb) => {
    const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
    cb(null, file.fieldname + '-' + uniqueSuffix + path.extname(file.originalname));
  }
});
const upload = multer({ storage: storage });

const backgroundStorage = multer.diskStorage({
    destination: (req, file, cb) => cb(null, backgroundsDir),
    filename: (req, file, cb) => {
        const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
        cb(null, 'background-' + uniqueSuffix + path.extname(file.originalname));
    }
});
const backgroundUpload = multer({ storage: backgroundStorage });

// --- All routes are now prefixed with /api ---

app.get('/api/settings/background', async (req, res) => {
    try {
        const [rows] = await dbPool.execute("SELECT setting_value FROM settings WHERE setting_key = 'player_background'");
        res.send({ backgroundUrl: rows.length > 0 ? rows[0].setting_value : null });
    } catch (dbError) {
        console.error('Database error:', dbError);
        res.status(500).send({ message: 'Failed to retrieve background setting.' });
    }
});

app.post('/api/admin/background', backgroundUpload.single('background'), async (req, res) => {
    if (!req.file) {
        return res.status(400).send({ message: 'No background image was uploaded.' });
    }
    const newBackgroundUrl = `/backgrounds/${req.file.filename}`;
    try {
        const [rows] = await dbPool.execute("SELECT setting_value FROM settings WHERE setting_key = 'player_background'");
        const oldBackgroundUrl = rows.length > 0 ? rows[0].setting_value : null;

        await dbPool.execute(
            "INSERT INTO settings (setting_key, setting_value) VALUES ('player_background', ?) ON DUPLICATE KEY UPDATE setting_value = ?",
            [newBackgroundUrl, newBackgroundUrl]
        );

        if (oldBackgroundUrl) {
            const oldFilePath = path.join(backgroundsDir, path.basename(oldBackgroundUrl));
            await fs.unlink(oldFilePath).catch(err => console.error("Error deleting old background file:", err.message));
        }
        res.send({ message: 'Background updated successfully.', backgroundUrl: newBackgroundUrl });
    } catch (dbError) {
        console.error('Database error:', dbError);
        res.status(500).send({ message: 'Failed to update background setting.' });
    }
});

app.post('/api/scenes', upload.single('video'), (req, res) => {
  if (!req.file) return res.status(400).send({ message: 'No video file was uploaded.' });

  const { title } = req.body;
  const videoFilename = req.file.filename;
  const thumbnailFilename = `thumb-${path.parse(videoFilename).name}.png`;

  ffmpeg(req.file.path)
    .on('end', async () => {
      try {
        const videoUrl = `/videos/${videoFilename}`;
        const thumbnailUrl = `/thumbnails/${thumbnailFilename}`;
        const [result] = await dbPool.execute('INSERT INTO scenes (title, video_path, thumbnail_path) VALUES (?, ?, ?)', [title, videoUrl, thumbnailUrl]);
        res.status(201).send({ id: result.insertId, title, video_path: videoUrl, thumbnail_path: thumbnailUrl });
      } catch (dbError) {
        console.error('Database error:', dbError);
        res.status(500).send({ message: 'Failed to save scene to database.' });
      }
    })
    .on('error', (err) => {
      console.error('FFmpeg error:', err);
      res.status(500).send({ message: 'Failed to generate thumbnail.' });
    })
    .screenshots({ timestamps: ['00:00:05.000'], filename: thumbnailFilename, folder: thumbnailsDir, size: '320x240' });
});

app.get('/api/scenes', async (req, res) => {
  try {
    const [rows] = await dbPool.execute('SELECT * FROM scenes ORDER BY created_at DESC');
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scenes.' });
  }
});

app.get('/api/scenes/:id', async (req, res) => {
  try {
    const [rows] = await dbPool.execute('SELECT * FROM scenes WHERE id = ?', [req.params.id]);
    if (rows.length === 0) return res.status(404).send({ message: 'Scene not found.' });
    res.send(rows[0]);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene.' });
  }
});

app.put('/api/scenes/:id', async (req, res) => {
  try {
    await dbPool.execute('UPDATE scenes SET title = ? WHERE id = ?', [req.body.title, req.params.id]);
    res.send({ message: 'Scene updated successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to update scene.' });
  }
});

app.delete('/api/scenes/:id', async (req, res) => {
  try {
    const [rows] = await dbPool.execute('SELECT * FROM scenes WHERE id = ?', [req.params.id]);
    if (rows.length === 0) return res.status(404).send({ message: 'Scene not found.' });
    const scene = rows[0];

    const videoPath = path.join(videosDir, path.basename(scene.video_path));
    const thumbnailPath = path.join(thumbnailsDir, path.basename(scene.thumbnail_path));

    await fs.unlink(videoPath).catch(err => console.error("Error deleting video file:", err.message));
    await fs.unlink(thumbnailPath).catch(err => console.error("Error deleting thumbnail file:", err.message));

    await dbPool.execute('DELETE FROM scenes WHERE id = ?', [req.params.id]);
    res.send({ message: 'Scene deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete scene.' });
  }
});

app.get('/api/scenes/:id/choices', async (req, res) => {
  try {
    const [rows] = await dbPool.execute(
      `SELECT c.id, c.choice_text, s.id as destination_id, s.title as destination_title
       FROM choices c JOIN scenes s ON c.destination_scene_id = s.id
       WHERE c.source_scene_id = ?`, [req.params.id]
    );
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve choices.' });
  }
});

app.post('/api/scenes/:id/choices', async (req, res) => {
  const { destination_scene_id, choice_text } = req.body;
  try {
    const [result] = await dbPool.execute('INSERT INTO choices (source_scene_id, destination_scene_id, choice_text) VALUES (?, ?, ?)', [req.params.id, destination_scene_id, choice_text]);
    res.status(201).send({ id: result.insertId });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to add choice.' });
  }
});

app.delete('/api/choices/:id', async (req, res) => {
  try {
    await dbPool.execute('DELETE FROM choices WHERE id = ?', [req.params.id]);
    res.send({ message: 'Choice deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete choice.' });
  }
});

app.get('/api/player/scenes/:id', async (req, res) => {
  try {
    const [sceneRows] = await dbPool.execute('SELECT * FROM scenes WHERE id = ?', [req.params.id]);
    if (sceneRows.length === 0) return res.status(404).send({ message: 'Scene not found.' });

    const [parentScenesRows] = await dbPool.execute(`SELECT s.id, s.title FROM scenes s JOIN choices c ON s.id = c.source_scene_id WHERE c.destination_scene_id = ?`, [req.params.id]);
    const [choicesRows] = await dbPool.execute(`SELECT c.id, c.choice_text, s.id as destination_scene_id, s.title as destination_scene_title FROM choices c JOIN scenes s ON c.destination_scene_id = s.id WHERE c.source_scene_id = ?`, [req.params.id]);

    res.send({ current_scene: sceneRows[0], parent_scenes: parentScenesRows, next_choices: choicesRows });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene data for the player.' });
  }
});

app.get('/api/admin/scenes/:id/relations', async (req, res) => {
  try {
    const [sceneRows] = await dbPool.execute('SELECT * FROM scenes WHERE id = ?', [req.params.id]);
    if (sceneRows.length === 0) return res.status(404).send({ message: 'Scene not found.' });

    const [parentScenesRows] = await dbPool.execute(`SELECT s.id, s.title FROM scenes s JOIN choices c ON s.id = c.source_scene_id WHERE c.destination_scene_id = ?`, [req.params.id]);
    const [childScenesRows] = await dbPool.execute(`SELECT s.id, s.title, c.choice_text FROM scenes s JOIN choices c ON s.id = c.destination_scene_id WHERE c.source_scene_id = ?`, [req.params.id]);

    res.send({ current_scene: sceneRows[0], parent_scenes: parentScenesRows, child_scenes: childScenesRows });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene relations.' });
  }
});

app.get('/', (req, res) => {
  res.send('Backend server is running!');
});

app.listen(PORT, '0.0.0.0', () => {
  console.log(`Server is running on http://localhost:${PORT} and listening on all interfaces.`);
});
