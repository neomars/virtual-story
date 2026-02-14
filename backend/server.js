
const express = require('express');
const session = require('express-session');
const bcrypt = require('bcrypt');
const rateLimit = require('express-rate-limit');
const cors = require('cors');
const path = require('path');
const fs = require('fs').promises;
const multer = require('multer');
const ffmpegPath = require('ffmpeg-static');
const { exec } = require('child_process');
const { pool: dbPool } = require('./db');

const app = express();
const PORT = process.env.PORT || 3000;

app.use(cors({
  origin: true,
  credentials: true
}));
app.use(express.json());

// Session configuration
app.use(session({
  secret: process.env.SESSION_SECRET || 'virtual-story-default-secret',
  resave: false,
  saveUninitialized: false,
  cookie: {
    secure: false, // Set to true if using HTTPS
    maxAge: 24 * 60 * 60 * 1000 // 24 hours
  }
}));

// Login rate limiter
const loginLimiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 10, // Limit each IP to 10 login attempts per window
  message: { message: 'Trop de tentatives de connexion. Réessayez dans 15 minutes.' }
});

// Middleware to check authentication
const isAuthenticated = (req, res, next) => {
  if (req.session && req.session.userId) {
    return next();
  }
  res.status(401).send({ message: 'Non autorisé. Veuillez vous connecter.' });
};

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

const getAbsPath = (vPath) => path.join(__dirname, 'uploads', vPath.startsWith('/') ? vPath.slice(1) : vPath);

const deleteSceneInternal = async (sceneId) => {
  try {
    const [rows] = await dbPool.query('SELECT video_path, thumbnail_path FROM scenes WHERE id = ?', [sceneId]);
    if (rows.length === 0) return;
    const scene = rows[0];

    await fs.unlink(getAbsPath(scene.video_path)).catch(err => console.error(`[CLEANUP] Error deleting video ${scene.video_path}:`, err.message));
    await fs.unlink(getAbsPath(scene.thumbnail_path)).catch(err => console.error(`[CLEANUP] Error deleting thumbnail ${scene.thumbnail_path}:`, err.message));

    await dbPool.query('DELETE FROM scenes WHERE id = ?', [sceneId]);
    console.log(`[CLEANUP] Scene ${sceneId} and its files removed after concatenation.`);
  } catch (err) {
    console.error(`[CLEANUP] Failed to delete scene ${sceneId}:`, err);
  }
};

const processVideoAndThumbnail = async (videoPath, prependId, appendId, thumbnailPath) => {
  let inputs = [];
  if (prependId && prependId !== 'null' && prependId !== 'undefined') {
    const [rows] = await dbPool.query('SELECT video_path FROM scenes WHERE id = ?', [prependId]);
    if (rows.length > 0) inputs.push(getAbsPath(rows[0].video_path));
  }
  inputs.push(videoPath);
  if (appendId && appendId !== 'null' && appendId !== 'undefined') {
    const [rows] = await dbPool.query('SELECT video_path FROM scenes WHERE id = ?', [appendId]);
    if (rows.length > 0) inputs.push(getAbsPath(rows[0].video_path));
  }

  if (inputs.length > 1) {
    console.log(`[FFMPEG] Concatenating ${inputs.length} videos...`);
    const tempOutput = videoPath + '.concat.mp4';
    let filter = '';
    for (let i = 0; i < inputs.length; i++) {
      filter += `[${i}:v]scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2,setsar=1[v${i}];`;
    }
    for (let i = 0; i < inputs.length; i++) {
      filter += `[v${i}][${i}:a]`;
    }
    filter += `concat=n=${inputs.length}:v=1:a=1[outv][outa]`;

    const inputArgs = inputs.map(p => `-i "${p}"`).join(' ');
    const concatCmd = `unset LD_LIBRARY_PATH; "${ffmpegPath}" ${inputArgs} -filter_complex "${filter}" -map "[outv]" -map "[outa]" -c:v libx264 -preset fast -crf 23 -c:a aac -b:a 128k -y "${tempOutput}"`;

    await new Promise((resolve, reject) => {
      exec(concatCmd, (err, stdout, stderr) => {
        if (err) { reject(new Error(stderr)); } else { resolve(); }
      });
    });

    await fs.unlink(videoPath);
    await fs.rename(tempOutput, videoPath);
    console.log('[FFMPEG] Concatenation successful.');
  }

  console.log(`[FFMPEG] Generating thumbnail for '${videoPath}'.`);
  const thumbCmd = `unset LD_LIBRARY_PATH; "${ffmpegPath}" -i "${videoPath}" -ss 00:00:05.000 -vframes 1 -s 320x240 -y "${thumbnailPath}"`;
  await new Promise((resolve, reject) => {
    exec(thumbCmd, (err, stdout, stderr) => {
      if (err) { reject(new Error(stderr)); } else { resolve(); }
    });
  });
};

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

// --- Auth Endpoints ---

app.post('/api/auth/login', loginLimiter, async (req, res) => {
  const { username, password } = req.body;
  try {
    const [rows] = await dbPool.query('SELECT * FROM users WHERE username = ?', [username]);
    if (rows.length === 0) {
      return res.status(401).send({ message: 'Identifiants invalides.' });
    }
    const user = rows[0];
    const match = await bcrypt.compare(password, user.password_hash);
    if (!match) {
      return res.status(401).send({ message: 'Identifiants invalides.' });
    }
    req.session.userId = user.id;
    req.session.username = user.username;
    res.send({ id: user.id, username: user.username });
  } catch (err) {
    console.error('Login error:', err);
    res.status(500).send({ message: 'Erreur lors de la connexion.' });
  }
});

app.post('/api/auth/logout', (req, res) => {
  req.session.destroy(err => {
    if (err) {
      return res.status(500).send({ message: 'Erreur lors de la déconnexion.' });
    }
    res.clearCookie('connect.sid');
    res.send({ message: 'Déconnecté avec succès.' });
  });
});

app.get('/api/auth/me', (req, res) => {
  if (req.session && req.session.userId) {
    res.send({ id: req.session.userId, username: req.session.username });
  } else {
    res.status(401).send({ message: 'Non authentifié.' });
  }
});

// --- Admin User Management Endpoints ---

app.get('/api/admin/users', isAuthenticated, async (req, res) => {
  try {
    const [rows] = await dbPool.query('SELECT id, username FROM users');
    res.send(rows);
  } catch (err) {
    console.error('Fetch users error:', err);
    res.status(500).send({ message: 'Erreur lors de la récupération des utilisateurs.' });
  }
});

app.post('/api/admin/users', isAuthenticated, async (req, res) => {
  const { username, password } = req.body;
  try {
    const hash = await bcrypt.hash(password, 10);
    const [result] = await dbPool.query('INSERT INTO users (username, password_hash) VALUES (?, ?)', [username, hash]);
    res.status(201).send({ id: result.insertId, username });
  } catch (err) {
    console.error('Create user error:', err);
    res.status(500).send({ message: 'Erreur lors de la création de l\'utilisateur.' });
  }
});

app.put('/api/admin/users/:id', isAuthenticated, async (req, res) => {
  const { username, password } = req.body;
  try {
    if (password) {
      const hash = await bcrypt.hash(password, 10);
      await dbPool.query('UPDATE users SET username = ?, password_hash = ? WHERE id = ?', [username, hash, req.params.id]);
    } else {
      await dbPool.query('UPDATE users SET username = ? WHERE id = ?', [username, req.params.id]);
    }
    res.send({ message: 'Utilisateur mis à jour.' });
  } catch (err) {
    console.error('Update user error:', err);
    res.status(500).send({ message: 'Erreur lors de la mise à jour de l\'utilisateur.' });
  }
});

app.delete('/api/admin/users/:id', isAuthenticated, async (req, res) => {
  try {
    // Prevent deleting self
    if (parseInt(req.params.id) === req.session.userId) {
      return res.status(400).send({ message: 'Vous ne pouvez pas supprimer votre propre compte.' });
    }
    await dbPool.query('DELETE FROM users WHERE id = ?', [req.params.id]);
    res.send({ message: 'Utilisateur supprimé.' });
  } catch (err) {
    console.error('Delete user error:', err);
    res.status(500).send({ message: 'Erreur lors de la suppression de l\'utilisateur.' });
  }
});

app.post('/api/admin/change-password', isAuthenticated, async (req, res) => {
  const { oldPassword, newPassword } = req.body;
  try {
    const [rows] = await dbPool.query('SELECT * FROM users WHERE id = ?', [req.session.userId]);
    const user = rows[0];
    const match = await bcrypt.compare(oldPassword, user.password_hash);
    if (!match) {
      return res.status(401).send({ message: 'Ancien mot de passe incorrect.' });
    }
    const hash = await bcrypt.hash(newPassword, 10);
    await dbPool.query('UPDATE users SET password_hash = ? WHERE id = ?', [hash, req.session.userId]);
    res.send({ message: 'Mot de passe mis à jour avec succès.' });
  } catch (err) {
    console.error('Change password error:', err);
    res.status(500).send({ message: 'Erreur lors du changement de mot de passe.' });
  }
});

// --- All routes are now prefixed with /api ---

app.post('/api/admin/db-sync', async (req, res) => {
  try {
    console.log('[DB-SYNC] Starting synchronization...');

    // 1. Create tables without constraints first to avoid dependency issues
    await dbPool.query("CREATE TABLE IF NOT EXISTS users (id INT AUTO_INCREMENT PRIMARY KEY, username VARCHAR(255) NOT NULL UNIQUE, password_hash VARCHAR(255) NOT NULL)");
    await dbPool.query("CREATE TABLE IF NOT EXISTS parts (id INT AUTO_INCREMENT PRIMARY KEY, title VARCHAR(255) NOT NULL, first_scene_id INT, `order` INT DEFAULT 0, loop_video_path VARCHAR(255))");
    await dbPool.query("CREATE TABLE IF NOT EXISTS scenes (id INT AUTO_INCREMENT PRIMARY KEY, title VARCHAR(255) NOT NULL, video_path VARCHAR(255) NOT NULL, thumbnail_path VARCHAR(255) NOT NULL, part_id INT, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)");
    await dbPool.query("CREATE TABLE IF NOT EXISTS choices (id INT AUTO_INCREMENT PRIMARY KEY, source_scene_id INT NOT NULL, destination_scene_id INT NOT NULL, choice_text VARCHAR(255) NOT NULL)");
    await dbPool.query("CREATE TABLE IF NOT EXISTS settings (setting_key VARCHAR(50) PRIMARY KEY, setting_value VARCHAR(255))");

    // 2. Auth check (only after users table exists)
    const [userRows] = await dbPool.query("SELECT * FROM users");
    if (userRows.length > 0 && !(req.session && req.session.userId)) {
      return res.status(401).send({ message: 'Authentification requise pour synchroniser la base de données.' });
    }

    // 3. Ensure admin user exists
    const [adminRows] = await dbPool.query("SELECT * FROM users WHERE username = 'admin'");
    if (adminRows.length === 0) {
      const hash = await bcrypt.hash('admin', 10);
      await dbPool.query("INSERT INTO users (username, password_hash) VALUES (?, ?)", ['admin', hash]);
    }

    // 4. Migrations (Columns)
    const [columns] = await dbPool.query("SHOW COLUMNS FROM scenes LIKE 'part_id'");
    if (columns.length === 0) {
      await dbPool.query("ALTER TABLE scenes ADD COLUMN part_id INT");
    }

    const [partColumns] = await dbPool.query("SHOW COLUMNS FROM parts LIKE 'loop_video_path'");
    if (partColumns.length === 0) {
      await dbPool.query("ALTER TABLE parts ADD COLUMN loop_video_path VARCHAR(255)");
    }

    // 5. Constraints (Defensive)
    const addFK = async (sql) => { try { await dbPool.query(sql); } catch (e) { /* ignore existing */ } };
    await addFK("ALTER TABLE scenes ADD CONSTRAINT fk_part_id FOREIGN KEY (part_id) REFERENCES parts(id) ON DELETE SET NULL");
    await addFK("ALTER TABLE parts ADD CONSTRAINT fk_parts_first_scene FOREIGN KEY (first_scene_id) REFERENCES scenes(id) ON DELETE SET NULL");
    await addFK("ALTER TABLE choices ADD CONSTRAINT fk_choices_source FOREIGN KEY (source_scene_id) REFERENCES scenes(id) ON DELETE CASCADE");
    await addFK("ALTER TABLE choices ADD CONSTRAINT fk_choices_dest FOREIGN KEY (destination_scene_id) REFERENCES scenes(id) ON DELETE CASCADE");

    // 6. Default settings
    await dbPool.query("INSERT INTO settings (setting_key, setting_value) VALUES ('player_background', NULL) ON DUPLICATE KEY UPDATE setting_key=setting_key");

    console.log('[DB-SYNC] Synchronization completed successfully.');
    res.send({ message: 'Base de données synchronisée avec succès !' });
  } catch (err) {
    console.error('[DB-SYNC] Error:', err);
    res.status(500).send({ message: 'Échec de la synchronisation : ' + err.message });
  }
});

app.get('/api/parts', async (req, res) => {
  try {
    const [rows] = await dbPool.query('SELECT * FROM parts ORDER BY `order` ASC');
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve parts.' });
  }
});

app.post('/api/parts', isAuthenticated, partUpload.single('loop_video'), async (req, res) => {
  const { title, first_scene_id, order } = req.body;
  const loop_video_path = req.file ? `/parts/${req.file.filename}` : null;
  try {
    const [result] = await dbPool.query(
      'INSERT INTO parts (title, first_scene_id, `order`, loop_video_path) VALUES (?, ?, ?, ?)',
      [title, first_scene_id, order || 0, loop_video_path]
    );
    res.status(201).send({ id: result.insertId, title, first_scene_id, order: order || 0, loop_video_path });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to create part.' });
  }
});

app.put('/api/parts/:id', isAuthenticated, partUpload.single('loop_video'), async (req, res) => {
  const { title, first_scene_id, order } = req.body;
  try {
    if (req.file) {
      const loop_video_path = `/parts/${req.file.filename}`;
      await dbPool.query(
        'UPDATE parts SET title = ?, first_scene_id = ?, `order` = ?, loop_video_path = ? WHERE id = ?',
        [title, first_scene_id, order, loop_video_path, req.params.id]
      );
    } else {
      await dbPool.query(
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

app.delete('/api/parts/:id', isAuthenticated, async (req, res) => {
  try {
    await dbPool.query('DELETE FROM parts WHERE id = ?', [req.params.id]);
    res.send({ message: 'Part deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete part.' });
  }
});

app.get('/api/settings/background', async (req, res) => {
    try {
        const [rows] = await dbPool.query("SELECT setting_value FROM settings WHERE setting_key = 'player_background'");
        res.send({ backgroundUrl: rows.length > 0 ? rows[0].setting_value : null });
    } catch (dbError) {
        console.error('Database error:', dbError);
        res.status(500).send({ message: 'Failed to retrieve background setting.' });
    }
});

app.post('/api/admin/background', isAuthenticated, backgroundUpload.single('background'), async (req, res) => {
    if (!req.file) {
        return res.status(400).send({ message: 'No background image was uploaded.' });
    }
    const newBackgroundUrl = `/backgrounds/${req.file.filename}`;
    try {
        const [rows] = await dbPool.query("SELECT setting_value FROM settings WHERE setting_key = 'player_background'");
        const oldBackgroundUrl = rows.length > 0 ? rows[0].setting_value : null;

        await dbPool.query(
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

app.post('/api/scenes', isAuthenticated, (req, res, next) => {
  console.log('[UPLOAD] Received POST request to /api/scenes');
  upload.single('video')(req, res, (err) => {
    if (err instanceof multer.MulterError) {
      console.error('[UPLOAD] Multer error:', err);
      return res.status(400).send({ message: `Upload error: ${err.message}` });
    } else if (err) {
      console.error('[UPLOAD] Unknown error during multer processing:', err);
      return res.status(500).send({ message: `Unknown upload error: ${err.message}` });
    }
    next();
  });
}, async (req, res) => {
  if (!req.file) {
    console.log('[UPLOAD] Upload failed: No file received in req.file after multer.');
    return res.status(400).send({ message: 'No video file was uploaded.' });
  }

  const { title, prepend_scene_id, append_scene_id } = req.body;
  const videoFilename = req.file.filename;
  const thumbnailFilename = `thumb-${path.parse(videoFilename).name}.png`;
  const thumbnailFullPath = path.join(thumbnailsDir, thumbnailFilename);

  try {
    await processVideoAndThumbnail(req.file.path, prepend_scene_id, append_scene_id, thumbnailFullPath);

    const videoUrl = `/videos/${videoFilename}`;
    const thumbnailUrl = `/thumbnails/${thumbnailFilename}`;

    let { part_id } = req.body;
    if (part_id === undefined || part_id === '' || part_id === 'null' || part_id === 'undefined') {
      part_id = null;
    }
    const [result] = await dbPool.query('INSERT INTO scenes (title, video_path, thumbnail_path, part_id) VALUES (?, ?, ?, ?)', [title, videoUrl, thumbnailUrl, part_id]);
    const newId = result.insertId;

    // Remove source scenes after successful creation of the concatenated scene
    if (prepend_scene_id && prepend_scene_id !== 'null' && prepend_scene_id != newId) await deleteSceneInternal(prepend_scene_id);
    if (append_scene_id && append_scene_id !== 'null' && append_scene_id != newId) await deleteSceneInternal(append_scene_id);

    res.status(201).send({ id: newId, title, video_path: videoUrl, thumbnail_path: thumbnailUrl });
  } catch (error) {
    console.error('[UPLOAD] Error processing upload:', error);
    await fs.unlink(req.file.path).catch(() => {});
    await fs.unlink(thumbnailFullPath).catch(() => {});
    res.status(500).send({ message: 'Failed to process video: ' + error.message });
  }
});

app.get('/api/scenes', isAuthenticated, async (req, res) => {
  try {
    const [rows] = await dbPool.query('SELECT * FROM scenes ORDER BY created_at DESC');
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scenes.' });
  }
});

app.get('/api/scenes/:id', async (req, res) => {
  try {
    const [rows] = await dbPool.query('SELECT * FROM scenes WHERE id = ?', [req.params.id]);
    if (rows.length === 0) return res.status(404).send({ message: 'Scene not found.' });
    res.send(rows[0]);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene.' });
  }
});

app.put('/api/scenes/:id', isAuthenticated, (req, res, next) => {
  upload.single('video')(req, res, (err) => {
    if (err) return res.status(400).send({ message: `Upload error: ${err.message}` });
    next();
  });
}, async (req, res) => {
  try {
    let { title, part_id, prepend_scene_id, append_scene_id } = req.body;
    if (part_id === undefined || part_id === '' || part_id === 'null' || part_id === 'undefined') {
      part_id = null;
    }

    if (req.file) {
      const videoFilename = req.file.filename;
      const thumbnailFilename = `thumb-${path.parse(videoFilename).name}.png`;
      const thumbnailFullPath = path.join(thumbnailsDir, thumbnailFilename);

      await processVideoAndThumbnail(req.file.path, prepend_scene_id, append_scene_id, thumbnailFullPath);

      const videoUrl = `/videos/${videoFilename}`;
      const thumbnailUrl = `/thumbnails/${thumbnailFilename}`;

      // Delete old files
      const [oldRows] = await dbPool.query('SELECT video_path, thumbnail_path FROM scenes WHERE id = ?', [req.params.id]);
      if (oldRows.length > 0) {
        await fs.unlink(getAbsPath(oldRows[0].video_path)).catch(() => {});
        await fs.unlink(getAbsPath(oldRows[0].thumbnail_path)).catch(() => {});
      }

      await dbPool.query(
        'UPDATE scenes SET title = ?, part_id = ?, video_path = ?, thumbnail_path = ? WHERE id = ?',
        [title, part_id, videoUrl, thumbnailUrl, req.params.id]
      );

      // Remove source scenes after successful update
      if (prepend_scene_id && prepend_scene_id !== 'null' && prepend_scene_id != req.params.id) await deleteSceneInternal(prepend_scene_id);
      if (append_scene_id && append_scene_id !== 'null' && append_scene_id != req.params.id) await deleteSceneInternal(append_scene_id);
    } else {
      await dbPool.query('UPDATE scenes SET title = ?, part_id = ? WHERE id = ?', [title, part_id, req.params.id]);
    }

    res.send({ message: 'Scene updated successfully.' });
  } catch (error) {
    console.error('Update error:', error);
    res.status(500).send({ message: 'Failed to update scene: ' + error.message });
  }
});

app.delete('/api/scenes/:id', isAuthenticated, async (req, res) => {
  try {
    const [rows] = await dbPool.query('SELECT id FROM scenes WHERE id = ?', [req.params.id]);
    if (rows.length === 0) return res.status(404).send({ message: 'Scene not found.' });

    await deleteSceneInternal(req.params.id);
    res.send({ message: 'Scene deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete scene.' });
  }
});

app.get('/api/scenes/:id/choices', async (req, res) => {
  try {
    const [rows] = await dbPool.query(
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

app.post('/api/scenes/:id/choices', isAuthenticated, async (req, res) => {
  const { destination_scene_id, choice_text } = req.body;
  try {
    const [result] = await dbPool.query('INSERT INTO choices (source_scene_id, destination_scene_id, choice_text) VALUES (?, ?, ?)', [req.params.id, destination_scene_id, choice_text]);
    res.status(201).send({ id: result.insertId });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to add choice.' });
  }
});

app.delete('/api/choices/:id', isAuthenticated, async (req, res) => {
  try {
    await dbPool.query('DELETE FROM choices WHERE id = ?', [req.params.id]);
    res.send({ message: 'Choice deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete choice.' });
  }
});

app.get('/api/player/scenes/:id', async (req, res) => {
  const sceneId = req.params.id;
  try {
    // 1. Fetch the scene directly
    const [sceneRows] = await dbPool.query("SELECT s.*, p.loop_video_path as part_loop_video_path FROM scenes s LEFT JOIN parts p ON s.part_id = p.id WHERE s.id = ?", [sceneId]);
    if (sceneRows.length === 0) return res.status(404).send({ message: 'Scene not found.' });

    let currentScene = sceneRows[0];

    // 2. Inheritance Logic for part loop video
    // If the scene itself has no part_id, we look at ancestors up to 10 levels
    let inheritedPartId = null;
    let inheritedLoopVideo = null;

    if (currentScene.part_id === null) {
      let currentId = sceneId;
      const visited = new Set([currentId]);

      for (let depth = 0; depth < 10; depth++) {
        // Find parents
        const [parents] = await dbPool.query("SELECT source_scene_id FROM choices WHERE destination_scene_id = ?", [currentId]);
        if (parents.length === 0) break;

        // Check if any parent belongs to a part
        const parentIds = parents.map(p => p.source_scene_id);
        const [parentScenes] = await dbPool.query("SELECT id, part_id FROM scenes WHERE id IN (?)", [parentIds]);

        const partParent = parentScenes.find(p => p.part_id !== null);
        if (partParent) {
          inheritedPartId = partParent.part_id;
          const [partData] = await dbPool.query("SELECT loop_video_path FROM parts WHERE id = ?", [inheritedPartId]);
          if (partData.length > 0) inheritedLoopVideo = partData[0].loop_video_path;
          break;
        }

        // Move up one level (just pick first parent to keep it simple and avoid explosion)
        currentId = parentIds[0];
        if (visited.has(currentId)) break;
        visited.add(currentId);
      }
    }

    if (inheritedPartId) {
      currentScene.part_id = inheritedPartId;
      currentScene.part_loop_video_path = inheritedLoopVideo;
    }

    // 3. Parents, Choices, Siblings
    const [parentScenesRows] = await dbPool.query("SELECT s.id, s.title FROM scenes s JOIN choices c ON s.id = c.source_scene_id WHERE c.destination_scene_id = ?", [sceneId]);
    const [choicesRows] = await dbPool.query("SELECT c.id, c.choice_text, s.id as destination_scene_id, s.title as destination_scene_title FROM choices c JOIN scenes s ON c.destination_scene_id = s.id WHERE c.source_scene_id = ?", [sceneId]);

    const prevId = req.query.previous_scene_id;
    let siblingsQuery, queryParams;

    if (prevId) {
      siblingsQuery = "SELECT s.id, s.title, c.choice_text FROM scenes s JOIN choices c ON s.id = c.destination_scene_id WHERE c.source_scene_id = ? AND s.id != ?";
      queryParams = [prevId, sceneId];
    } else {
      siblingsQuery = "SELECT DISTINCT s.id, s.title, c.choice_text FROM scenes s JOIN choices c ON s.id = c.destination_scene_id WHERE c.source_scene_id IN (SELECT source_scene_id FROM choices WHERE destination_scene_id = ?) AND s.id != ?";
      queryParams = [sceneId, sceneId];
    }

    const [siblingsRows] = await dbPool.query(siblingsQuery, queryParams);

    res.send({
      current_scene: currentScene,
      parent_scenes: parentScenesRows,
      next_choices: choicesRows,
      sibling_scenes: siblingsRows
    });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene data.' });
  }
});

app.get('/api/admin/scenes/:id/relations', isAuthenticated, async (req, res) => {
  try {
    const [sceneRows] = await dbPool.query('SELECT * FROM scenes WHERE id = ?', [req.params.id]);
    if (sceneRows.length === 0) return res.status(404).send({ message: 'Scene not found.' });

    const [parentScenesRows] = await dbPool.query(
      `SELECT s.id, s.title, c.choice_text, c.id AS choice_id
       FROM scenes s JOIN choices c ON s.id = c.source_scene_id
       WHERE c.destination_scene_id = ?`,
      [req.params.id]
    );
    const [childScenesRows] = await dbPool.query(
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

app.get('/api/admin/story-graph', isAuthenticated, async (req, res) => {
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

// Global error handler
app.use((err, req, res, next) => {
  console.error('Unhandled error:', err);
  res.status(500).send({ message: 'An unexpected error occurred on the server.' });
});

app.listen(PORT, '0.0.0.0', () => {
  console.log(`Server is running on http://localhost:${PORT} and listening on all interfaces.`);
});
