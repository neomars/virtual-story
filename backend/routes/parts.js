const express = require('express');
const router = express.Router();
const path = require('path');
const multer = require('multer');
const { pool: dbPool } = require('../db');
const { isAuthenticated } = require('../middleware/auth');
const { partsDir, videoFileFilter } = require('../utils/config');

const partStorage = multer.diskStorage({
  destination: (req, file, cb) => cb(null, partsDir),
  filename: (req, file, cb) => {
    const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
    cb(null, 'part-' + uniqueSuffix + path.extname(file.originalname));
  }
});

const partUpload = multer({ storage: partStorage, fileFilter: videoFileFilter });

router.get('/', async (req, res) => {
  try {
    const [rows] = await dbPool.query('SELECT * FROM parts ORDER BY `order` ASC');
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve parts.' });
  }
});

router.post('/', isAuthenticated, partUpload.single('loop_video'), async (req, res) => {
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

router.put('/:id', isAuthenticated, partUpload.single('loop_video'), async (req, res) => {
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

router.delete('/:id', isAuthenticated, async (req, res) => {
  try {
    await dbPool.query('DELETE FROM parts WHERE id = ?', [req.params.id]);
    res.send({ message: 'Part deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete part.' });
  }
});

module.exports = router;
