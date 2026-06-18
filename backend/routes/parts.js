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

router.get('/uploads', isAuthenticated, async (req, res) => {
  try {
    const fs = require('fs').promises;
    const files = await fs.readdir(partsDir);
    const videoFiles = files.filter(file => {
      const ext = path.extname(file).toLowerCase();
      return ['.mp4', '.mov', '.avi', '.mkv', '.webm'].includes(ext);
    });
    res.send(videoFiles);
  } catch (error) {
    console.error('Failed to list part uploads:', error);
    res.status(500).send({ message: 'Failed to list uploaded videos for chapters.' });
  }
});

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
  const { title, first_scene_id, order, existing_video_filename } = req.body;
  let loop_video_path = null;
  if (req.file) {
    loop_video_path = `/parts/${req.file.filename}`;
  } else if (existing_video_filename) {
    loop_video_path = `/parts/${existing_video_filename}`;
  }

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
  const { title, first_scene_id, order, existing_video_filename } = req.body;
  try {
    if (req.file) {
      const loop_video_path = `/parts/${req.file.filename}`;
      await dbPool.query(
        'UPDATE parts SET title = ?, first_scene_id = ?, `order` = ?, loop_video_path = ? WHERE id = ?',
        [title, first_scene_id, order, loop_video_path, req.params.id]
      );
    } else if (existing_video_filename) {
      const loop_video_path = `/parts/${existing_video_filename}`;
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
