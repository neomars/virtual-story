const express = require('express');
const router = express.Router();
const path = require('path');
const fs = require('fs').promises;
const bcrypt = require('bcrypt');
const multer = require('multer');
const { pool: dbPool } = require('../db');
const { isAuthenticated } = require('../middleware/auth');
const { backgroundsDir } = require('../utils/config');

const backgroundStorage = multer.diskStorage({
    destination: (req, file, cb) => cb(null, backgroundsDir),
    filename: (req, file, cb) => {
        const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
        cb(null, 'background-' + uniqueSuffix + path.extname(file.originalname));
    }
});
const backgroundUpload = multer({ storage: backgroundStorage });

router.get('/users', isAuthenticated, async (req, res) => {
  try {
    const [rows] = await dbPool.query('SELECT id, username FROM users');
    res.send(rows);
  } catch (err) {
    console.error('Fetch users error:', err);
    res.status(500).send({ message: 'Erreur lors de la récupération des utilisateurs.' });
  }
});

router.post('/users', isAuthenticated, async (req, res) => {
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

router.put('/users/:id', isAuthenticated, async (req, res) => {
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

router.delete('/users/:id', isAuthenticated, async (req, res) => {
  try {
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

router.post('/change-password', isAuthenticated, async (req, res) => {
  const { oldPassword, newPassword } = req.body;
  try {
    const [rows] = await dbPool.query('SELECT * FROM users WHERE id = ?', [req.session.userId]);
    const user = rows[0];
    const match = await bcrypt.compare(oldPassword, user.password_hash);
    if (!match) return res.status(401).send({ message: 'Ancien mot de passe incorrect.' });
    const hash = await bcrypt.hash(newPassword, 10);
    await dbPool.query('UPDATE users SET password_hash = ? WHERE id = ?', [hash, req.session.userId]);
    res.send({ message: 'Mot de passe mis à jour avec succès.' });
  } catch (err) {
    console.error('Change password error:', err);
    res.status(500).send({ message: 'Erreur lors du changement de mot de passe.' });
  }
});

router.post('/db-sync', async (req, res) => {
  try {
    console.log('[DB-SYNC] Starting synchronization...');
    await dbPool.query("CREATE TABLE IF NOT EXISTS users (id INT AUTO_INCREMENT PRIMARY KEY, username VARCHAR(255) NOT NULL UNIQUE, password_hash VARCHAR(255) NOT NULL)");
    await dbPool.query("CREATE TABLE IF NOT EXISTS parts (id INT AUTO_INCREMENT PRIMARY KEY, title VARCHAR(255) NOT NULL, first_scene_id INT, `order` INT DEFAULT 0, loop_video_path VARCHAR(255))");
    await dbPool.query("CREATE TABLE IF NOT EXISTS scenes (id INT AUTO_INCREMENT PRIMARY KEY, title VARCHAR(255) NOT NULL, video_path VARCHAR(255) NOT NULL, thumbnail_path VARCHAR(255) NOT NULL, part_id INT, created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)");
    await dbPool.query("CREATE TABLE IF NOT EXISTS choices (id INT AUTO_INCREMENT PRIMARY KEY, source_scene_id INT NOT NULL, destination_scene_id INT NOT NULL, choice_text VARCHAR(255) NOT NULL)");
    await dbPool.query("CREATE TABLE IF NOT EXISTS settings (setting_key VARCHAR(50) PRIMARY KEY, setting_value VARCHAR(255))");

    const [userRows] = await dbPool.query("SELECT * FROM users");
    if (userRows.length > 0 && !(req.session && req.session.userId)) {
      return res.status(401).send({ message: 'Authentification requise pour synchroniser la base de données.' });
    }

    const [adminRows] = await dbPool.query("SELECT * FROM users WHERE username = 'admin'");
    if (adminRows.length === 0) {
      const hash = await bcrypt.hash('admin', 10);
      await dbPool.query("INSERT INTO users (username, password_hash) VALUES (?, ?)", ['admin', hash]);
    }

    const [columns] = await dbPool.query("SHOW COLUMNS FROM scenes LIKE 'part_id'");
    if (columns.length === 0) await dbPool.query("ALTER TABLE scenes ADD COLUMN part_id INT");

    const [partColumns] = await dbPool.query("SHOW COLUMNS FROM parts LIKE 'loop_video_path'");
    if (partColumns.length === 0) await dbPool.query("ALTER TABLE parts ADD COLUMN loop_video_path VARCHAR(255)");

    const addFK = async (sql) => { try { await dbPool.query(sql); } catch (e) { } };
    await addFK("ALTER TABLE scenes ADD CONSTRAINT fk_part_id FOREIGN KEY (part_id) REFERENCES parts(id) ON DELETE SET NULL");
    await addFK("ALTER TABLE parts ADD CONSTRAINT fk_parts_first_scene FOREIGN KEY (first_scene_id) REFERENCES scenes(id) ON DELETE SET NULL");
    await addFK("ALTER TABLE choices ADD CONSTRAINT fk_choices_source FOREIGN KEY (source_scene_id) REFERENCES scenes(id) ON DELETE CASCADE");
    await addFK("ALTER TABLE choices ADD CONSTRAINT fk_choices_dest FOREIGN KEY (destination_scene_id) REFERENCES scenes(id) ON DELETE CASCADE");

    await dbPool.query("INSERT INTO settings (setting_key, setting_value) VALUES ('player_background', NULL) ON DUPLICATE KEY UPDATE setting_key=setting_key");

    console.log('[DB-SYNC] Synchronization completed successfully.');
    res.send({ message: 'Base de données synchronisée avec succès !' });
  } catch (err) {
    console.error('[DB-SYNC] Error:', err);
    res.status(500).send({ message: 'Échec de la synchronisation : ' + err.message });
  }
});

router.post('/background', isAuthenticated, backgroundUpload.single('background'), async (req, res) => {
    if (!req.file) return res.status(400).send({ message: 'No background image was uploaded.' });
    const newBackgroundUrl = `/backgrounds/${req.file.filename}`;
    try {
        const [rows] = await dbPool.query("SELECT setting_value FROM settings WHERE setting_key = 'player_background'");
        const oldBackgroundUrl = rows.length > 0 ? rows[0].setting_value : null;
        await dbPool.query("INSERT INTO settings (setting_key, setting_value) VALUES ('player_background', ?) ON DUPLICATE KEY UPDATE setting_value = ?", [newBackgroundUrl, newBackgroundUrl]);
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

router.get('/scenes/:id/relations', isAuthenticated, async (req, res) => {
  try {
    const [sceneRows] = await dbPool.query('SELECT * FROM scenes WHERE id = ?', [req.params.id]);
    if (sceneRows.length === 0) return res.status(404).send({ message: 'Scene not found.' });
    const [parentScenesRows] = await dbPool.query(`SELECT s.id, s.title, c.choice_text, c.id AS choice_id FROM scenes s JOIN choices c ON s.id = c.source_scene_id WHERE c.destination_scene_id = ?`, [req.params.id]);
    const [childScenesRows] = await dbPool.query(`SELECT s.id, s.title, c.choice_text, c.id AS choice_id FROM scenes s JOIN choices c ON s.id = c.destination_scene_id WHERE c.source_scene_id = ?`, [req.params.id]);
    res.send({ current_scene: sceneRows[0], parent_scenes: parentScenesRows, child_scenes: childScenesRows });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene relations.' });
  }
});

router.get('/story-graph', isAuthenticated, async (req, res) => {
  try {
    const [scenes] = await dbPool.query('SELECT s.*, p.title AS part_title FROM scenes s LEFT JOIN parts p ON s.part_id = p.id');
    const [choices] = await dbPool.query('SELECT * FROM choices');
    const sceneMap = new Map(scenes.map(s => [s.id, { ...s, children: [] }]));
    const choiceMap = new Map();
    for (const choice of choices) {
      const sourceScene = sceneMap.get(choice.source_scene_id);
      const destinationScene = sceneMap.get(choice.destination_scene_id);
      if (sourceScene && destinationScene) {
        sourceScene.children.push({ ...destinationScene, choice_text: choice.choice_text });
      }
      choiceMap.set(choice.destination_scene_id, true);
    }
    const rootScenes = [];
    for (const scene of sceneMap.values()) {
      if (!choiceMap.has(scene.id)) rootScenes.push(scene);
    }
    res.send(rootScenes);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to build the story graph.' });
  }
});

module.exports = router;
