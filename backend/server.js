
const express = require('express');
const cors = require('cors');
const path = require('path');
const fs = require('fs').promises;
const multer = require('multer');
const { exec } = require('child_process');
const { pool: dbPool } = require('./db');

const app = express();
const PORT = 3000;

app.use(cors());
app.use(express.json());

const videosDir = path.join(__dirname, 'uploads/videos');
const thumbnailsDir = path.join(__dirname, 'uploads/thumbnails');
const partsDir = path.join(__dirname, 'uploads/parts');
const backgroundsDir = path.join(__dirname, 'uploads');

(async () => {
    try {
        await fs.mkdir(videosDir, { recursive: true });
        await fs.mkdir(thumbnailsDir, { recursive: true });
        await fs.mkdir(partsDir, { recursive: true });
    } catch (error) {
        console.error("Error creating upload directories:", error);
    }
})();

app.use('/videos', express.static(videosDir));
app.use('/thumbnails', express.static(thumbnailsDir));
app.use('/parts', express.static(partsDir));
app.use('/backgrounds', express.static(backgroundsDir));

const storage = multer.diskStorage({
  destination: (req, file, cb) => cb(null, videosDir),
  filename: (req, file, cb) => {
    const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
    cb(null, file.fieldname + '-' + uniqueSuffix + path.extname(file.originalname));
  }
});

const videoFileFilter = (req, file, cb) => {
  if (file.mimetype === 'video/mp4') {
    cb(null, true);
  } else {
    cb(new Error('Invalid file type. Only MP4 videos are allowed.'), false);
  }
};

const upload = multer({ storage: storage, fileFilter: videoFileFilter });

const partStorage = multer.diskStorage({
  destination: (req, file, cb) => cb(null, partsDir),
  filename: (req, file, cb) => {
    const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
    cb(null, 'part-' + uniqueSuffix + path.extname(file.originalname));
  }
});
const partUpload = multer({ storage: partStorage, fileFilter: videoFileFilter });

const backgroundStorage = multer.diskStorage({
    destination: (req, file, cb) => cb(null, backgroundsDir),
    filename: (req, file, cb) => {
        const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
        cb(null, 'background-' + uniqueSuffix + path.extname(file.originalname));
    }
});
const backgroundUpload = multer({ storage: backgroundStorage });

// --- All routes are now prefixed with /api ---

// --- Parts (Chapters) Endpoints ---

app.post('/api/admin/db-sync', async (req, res) => {
  try {
    console.log('[DB-SYNC] Starting migration...');
    // Ensure parts table exists
    await dbPool.query(`
      CREATE TABLE IF NOT EXISTS parts (
        id INT AUTO_INCREMENT PRIMARY KEY,
        title VARCHAR(255) NOT NULL,
        first_scene_id INT,
        \`order\` INT DEFAULT 0,
        loop_video_path VARCHAR(255)
      )
    `);

    // Ensure part_id column exists in scenes
    const [columns] = await dbPool.query("SHOW COLUMNS FROM scenes LIKE 'part_id'");
    if (columns.length === 0) {
      console.log('[DB-SYNC] Adding part_id column to scenes...');
      await dbPool.query("ALTER TABLE scenes ADD COLUMN part_id INT");
    }

    // Try to add constraints independently to be robust
    try {
      await dbPool.query("ALTER TABLE scenes ADD CONSTRAINT fk_part_id FOREIGN KEY (part_id) REFERENCES parts(id) ON DELETE SET NULL");
    } catch (e) { /* Already exists */ }

    try {
      await dbPool.query("ALTER TABLE parts ADD CONSTRAINT fk_parts_first_scene FOREIGN KEY (first_scene_id) REFERENCES scenes(id) ON DELETE SET NULL");
    } catch (e) { /* Already exists */ }

    // Ensure loop_video_path column exists in parts
    const [partColumns] = await dbPool.query("SHOW COLUMNS FROM parts LIKE 'loop_video_path'");
    if (partColumns.length === 0) {
      console.log('[DB-SYNC] Adding loop_video_path column to parts...');
      await dbPool.query("ALTER TABLE parts ADD COLUMN loop_video_path VARCHAR(255)");
    }

    console.log('[DB-SYNC] Migration completed successfully.');
    res.send({ message: 'Base de données synchronisée avec succès !' });
  } catch (err) {
    console.error('[DB-SYNC] Error:', err);
    res.status(500).send({ message: 'Échec de la synchronisation : ' + err.message });
  }
});

app.get('/api/parts', async (req, res) => {
  try {
    const [rows] = await dbPool.execute('SELECT * FROM parts ORDER BY `order` ASC');
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve parts.' });
  }
});

app.post('/api/parts', partUpload.single('loop_video'), async (req, res) => {
  const { title, first_scene_id, order } = req.body;
  const loop_video_path = req.file ? `/parts/${req.file.filename}` : null;
  try {
    const [result] = await dbPool.execute(
      'INSERT INTO parts (title, first_scene_id, `order`, loop_video_path) VALUES (?, ?, ?, ?)',
      [title, first_scene_id, order || 0, loop_video_path]
    );
    res.status(201).send({ id: result.insertId, title, first_scene_id, order: order || 0, loop_video_path });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to create part.' });
  }
});

app.put('/api/parts/:id', partUpload.single('loop_video'), async (req, res) => {
  const { title, first_scene_id, order } = req.body;
  try {
    if (req.file) {
      const loop_video_path = `/parts/${req.file.filename}`;
      await dbPool.execute(
        'UPDATE parts SET title = ?, first_scene_id = ?, `order` = ?, loop_video_path = ? WHERE id = ?',
        [title, first_scene_id, order, loop_video_path, req.params.id]
      );
    } else {
      await dbPool.execute(
        'UPDATE parts SET title = ?, first_scene_id = ?, `order` = ? WHERE id = ?',
        [title, first_scene_id, order, req.params.id]
      );
    }
    res.send({ message: 'Part updated successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to update part.' });
  }
});

app.delete('/api/parts/:id', async (req, res) => {
  try {
    await dbPool.execute('DELETE FROM parts WHERE id = ?', [req.params.id]);
    res.send({ message: 'Part deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete part.' });
  }
});

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
  if (!req.file) {
    console.log('[UPLOAD] Upload failed: No file received.');
    return res.status(400).send({ message: 'No video file was uploaded.' });
  }

  console.log('[UPLOAD] Step 1: Video file received successfully.', req.file);

  const { title } = req.body;
  const videoFilename = req.file.filename;
  const thumbnailFilename = `thumb-${path.parse(videoFilename).name}.png`;
  const thumbnailFullPath = path.join(thumbnailsDir, thumbnailFilename);

  console.log(`[FFMPEG] Step 2: Starting thumbnail generation for '${req.file.path}'.`);

  // Define the direct ffmpeg command
  const ffmpegCommand = `unset LD_LIBRARY_PATH; /usr/bin/ffmpeg -i "${req.file.path}" -ss 00:00:05.000 -vframes 1 -s 320x240 "${thumbnailFullPath}"`;

  exec(ffmpegCommand, async (error, stdout, stderr) => {
    if (error) {
      console.error('[FFMPEG] Error during thumbnail generation:', error.message);
      console.error('[FFMPEG] stderr:', stderr);
      // Clean up the uploaded video file if thumbnail generation fails
      await fs.unlink(req.file.path).catch(err => console.error('Error cleaning up video file:', err.message));
      return res.status(500).send({ message: 'Failed to generate thumbnail.' });
    }

    console.log('[FFMPEG] Step 3: Thumbnail generation finished successfully.');

    try {
      const videoUrl = `/videos/${videoFilename}`;
      const thumbnailUrl = `/thumbnails/${thumbnailFilename}`;

      console.log(`[DB] Step 4: Saving scene to database with title: '${title}', video_path: '${videoUrl}', thumbnail_path: '${thumbnailUrl}'`);

      const { part_id } = req.body;
      const [result] = await dbPool.execute('INSERT INTO scenes (title, video_path, thumbnail_path, part_id) VALUES (?, ?, ?, ?)', [title, videoUrl, thumbnailUrl, part_id || null]);

      console.log('[DB] Step 5: Scene saved successfully. Insert ID:', result.insertId);
      res.status(201).send({ id: result.insertId, title, video_path: videoUrl, thumbnail_path: thumbnailUrl });
    } catch (dbError) {
      console.error('[DB] Error saving scene to database:', dbError);
      // Clean up uploaded files if DB insertion fails
      await fs.unlink(req.file.path).catch(err => console.error('Error cleaning up video file:', err.message));
      await fs.unlink(thumbnailFullPath).catch(err => console.error('Error cleaning up thumbnail file:', err.message));
      res.status(500).send({ message: 'Failed to save scene to database.' });
    }
  });
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
    const { title, part_id } = req.body;
    await dbPool.execute('UPDATE scenes SET title = ?, part_id = ? WHERE id = ?', [title, part_id || null, req.params.id]);
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
    const [sceneRows] = await dbPool.execute(
      `SELECT s.*, p.loop_video_path as part_loop_video_path
       FROM scenes s LEFT JOIN parts p ON s.part_id = p.id
       WHERE s.id = ?`, [req.params.id]
    );
    if (sceneRows.length === 0) return res.status(404).send({ message: 'Scene not found.' });

    const [parentScenesRows] = await dbPool.execute(`SELECT s.id, s.title FROM scenes s JOIN choices c ON s.id = c.source_scene_id WHERE c.destination_scene_id = ?`, [req.params.id]);
    const [choicesRows] = await dbPool.execute(`SELECT c.id, c.choice_text, s.id as destination_scene_id, s.title as destination_scene_title FROM choices c JOIN scenes s ON c.destination_scene_id = s.id WHERE c.source_scene_id = ?`, [req.params.id]);

    // Fetch siblings (scenes that share the same parent)
    const [siblingsRows] = await dbPool.execute(
      `SELECT DISTINCT s.id, s.title
       FROM scenes s
       JOIN choices c ON s.id = c.destination_scene_id
       WHERE c.source_scene_id IN (
         SELECT source_scene_id FROM choices WHERE destination_scene_id = ?
       ) AND s.id != ?`, [req.params.id, req.params.id]
    );

    res.send({
      current_scene: sceneRows[0],
      parent_scenes: parentScenesRows,
      next_choices: choicesRows,
      sibling_scenes: siblingsRows
    });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene data for the player.' });
  }
});

app.get('/api/admin/scenes/:id/relations', async (req, res) => {
  try {
    const [sceneRows] = await dbPool.execute('SELECT * FROM scenes WHERE id = ?', [req.params.id]);
    if (sceneRows.length === 0) return res.status(404).send({ message: 'Scene not found.' });

    const [parentScenesRows] = await dbPool.execute(
      `SELECT s.id, s.title, c.choice_text, c.id AS choice_id
       FROM scenes s JOIN choices c ON s.id = c.source_scene_id
       WHERE c.destination_scene_id = ?`,
      [req.params.id]
    );
    const [childScenesRows] = await dbPool.execute(
      `SELECT s.id, s.title, c.choice_text, c.id AS choice_id
       FROM scenes s JOIN choices c ON s.id = c.destination_scene_id
       WHERE c.source_scene_id = ?`,
      [req.params.id]
    );

    res.send({ current_scene: sceneRows[0], parent_scenes: parentScenesRows, child_scenes: childScenesRows });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene relations.' });
  }
});

app.get('/api/admin/story-graph', async (req, res) => {
  try {
    const [scenes] = await dbPool.query('SELECT s.*, p.title AS part_title FROM scenes s LEFT JOIN parts p ON s.part_id = p.id');
    const [choices] = await dbPool.query('SELECT * FROM choices');

    const sceneMap = new Map(scenes.map(s => [s.id, { ...s, children: [] }]));
    const choiceMap = new Map();

    for (const choice of choices) {
      const sourceScene = sceneMap.get(choice.source_scene_id);
      const destinationScene = sceneMap.get(choice.destination_scene_id);
      if (sourceScene && destinationScene) {
        sourceScene.children.push({
          ...destinationScene,
          choice_text: choice.choice_text
        });
      }
      choiceMap.set(choice.destination_scene_id, true);
    }

    const rootScenes = [];
    for (const scene of sceneMap.values()) {
      if (!choiceMap.has(scene.id)) {
        rootScenes.push(scene);
      }
    }

    res.send(rootScenes);

  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to build the story graph.' });
  }
});

app.get('/', (req, res) => {
  res.send('Backend server is running!');
});

app.listen(PORT, '0.0.0.0', () => {
  console.log(`Server is running on http://localhost:${PORT} and listening on all interfaces.`);
});
