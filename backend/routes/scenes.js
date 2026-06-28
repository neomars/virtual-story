const express = require('express');
const router = express.Router();
const path = require('path');
const fs = require('fs').promises;
const multer = require('multer');
const { pool: dbPool } = require('../db');
const { isAuthenticated } = require('../middleware/auth');
const { deleteSceneInternal, processVideoAndThumbnail, getAbsPath } = require('../utils/videoUtils');
const { videosDir, thumbnailsDir, videoFileFilter } = require('../utils/config');

const storage = multer.diskStorage({
  destination: (req, file, cb) => cb(null, videosDir),
  filename: (req, file, cb) => {
    const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
    cb(null, file.fieldname + '-' + uniqueSuffix + path.extname(file.originalname));
  }
});

const upload = multer({ storage: storage, fileFilter: videoFileFilter });

router.get('/uploads', isAuthenticated, async (req, res) => {
  try {
    const [scenes] = await dbPool.query('SELECT video_path FROM scenes');
    const usedVideos = scenes.map(s => path.basename(s.video_path));

    const files = await fs.readdir(videosDir);
    const result = [];
    for (const file of files) {
      const ext = path.extname(file).toLowerCase();
      if (['.mp4', '.mov', '.avi', '.mkv', '.webm'].includes(ext)) {
        if (usedVideos.includes(file)) continue;

        const basename = path.parse(file).name;
        const thumbnailName = `thumb-${basename}.png`;
        const thumbnailPath = path.join(thumbnailsDir, thumbnailName);
        let hasThumbnail = false;
        try {
          await fs.access(thumbnailPath);
          hasThumbnail = true;
        } catch (e) {}

        result.push({
          video: file,
          thumbnail: hasThumbnail ? `/thumbnails/${thumbnailName}` : null
        });
      }
    }
    res.send(result);
  } catch (error) {
    console.error('Failed to list uploads:', error);
    res.status(500).send({ message: 'Failed to list uploaded videos.' });
  }
});

router.get('/', isAuthenticated, async (req, res) => {
  try {
    const [rows] = await dbPool.query('SELECT * FROM scenes ORDER BY created_at DESC');
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scenes.' });
  }
});

router.post('/bulk-import', isAuthenticated, async (req, res) => {
  const { filenames, part_id } = req.body;
  if (!filenames || !Array.isArray(filenames) || filenames.length === 0) {
    return res.status(400).send({ message: 'No filenames provided for bulk import.' });
  }

  const results = [];
  const errors = [];

  for (const videoFilename of filenames) {
    try {
      const videoFilePath = path.join(videosDir, videoFilename);
      await fs.access(videoFilePath);

      const basename = path.parse(videoFilename).name;
      const title = basename.replace(/_/g, ' ').replace(/-/g, ' ');
      const thumbnailFilename = `thumb-${basename}.png`;
      const thumbnailFullPath = path.join(thumbnailsDir, thumbnailFilename);

      // Process video (generate thumbnail if missing)
      await processVideoAndThumbnail(videoFilePath, null, null, thumbnailFullPath);

      const videoUrl = `/videos/${videoFilename}`;
      const thumbnailUrl = `/thumbnails/${thumbnailFilename}`;
      const actualPartId = (!part_id || part_id === 'null' || part_id === 'undefined') ? null : part_id;

      const [result] = await dbPool.query(
        'INSERT INTO scenes (title, video_path, thumbnail_path, part_id) VALUES (?, ?, ?, ?)',
        [title, videoUrl, thumbnailUrl, actualPartId]
      );

      results.push({ id: result.insertId, title });
    } catch (error) {
      console.error(`[BULK-IMPORT] Error processing ${videoFilename}:`, error);
      errors.push({ filename: videoFilename, message: error.message });
    }
  }

  res.status(207).send({
    message: `Bulk import completed with ${results.length} successes and ${errors.length} errors.`,
    results,
    errors
  });
});

router.post('/', isAuthenticated, (req, res, next) => {
  upload.single('video')(req, res, (err) => {
    if (err) return res.status(400).send({ message: `Upload error: ${err.message}` });
    next();
  });
}, async (req, res) => {
  const { title, prepend_scene_id, append_scene_id, existing_video_filename } = req.body;
  let videoFilePath, videoFilename;

  if (req.file) {
    videoFilePath = req.file.path;
    videoFilename = req.file.filename;
  } else if (existing_video_filename) {
    videoFilename = existing_video_filename;
    videoFilePath = path.join(videosDir, videoFilename);
    // Check if file exists
    try {
      await fs.access(videoFilePath);
    } catch (e) {
      return res.status(400).send({ message: 'Selected existing video file not found.' });
    }
  } else {
    return res.status(400).send({ message: 'No video file was uploaded or selected.' });
  }

  const thumbnailFilename = `thumb-${path.parse(videoFilename).name}.png`;
  const thumbnailFullPath = path.join(thumbnailsDir, thumbnailFilename);
  try {
    await processVideoAndThumbnail(videoFilePath, prepend_scene_id, append_scene_id, thumbnailFullPath);
    const videoUrl = `/videos/${videoFilename}`;
    const thumbnailUrl = `/thumbnails/${thumbnailFilename}`;
    let { part_id } = req.body;
    if (!part_id || part_id === 'null' || part_id === 'undefined') part_id = null;
    const [result] = await dbPool.query('INSERT INTO scenes (title, video_path, thumbnail_path, part_id) VALUES (?, ?, ?, ?)', [title, videoUrl, thumbnailUrl, part_id]);
    const newId = result.insertId;
    let summary = { mergedVideos: [title], brokenLinks: [] };
    if (prepend_scene_id && prepend_scene_id !== 'null' && prepend_scene_id != newId) {
      const report = await deleteSceneInternal(prepend_scene_id);
      if (report) {
        summary.mergedVideos.unshift(report.title);
        summary.brokenLinks.push(...report.brokenLinks.map(l => ({ ...l, scene: report.title })));
      }
    }
    if (append_scene_id && append_scene_id !== 'null' && append_scene_id != newId) {
      const report = await deleteSceneInternal(append_scene_id);
      if (report) {
        summary.mergedVideos.push(report.title);
        summary.brokenLinks.push(...report.brokenLinks.map(l => ({ ...l, scene: report.title })));
      }
    }
    res.status(201).send({ id: newId, title, video_path: videoUrl, thumbnail_path: thumbnailUrl, concatenationSummary: summary });
  } catch (error) {
    console.error('[UPLOAD] Error processing upload:', error);
    if (req.file) await fs.unlink(req.file.path).catch(() => {});
    await fs.unlink(thumbnailFullPath).catch(() => {});
    res.status(500).send({ message: 'Failed to process video: ' + error.message });
  }
});

router.get('/:id', async (req, res) => {
  try {
    const [rows] = await dbPool.query('SELECT * FROM scenes WHERE id = ?', [req.params.id]);
    if (rows.length === 0) return res.status(404).send({ message: 'Scene not found.' });
    res.send(rows[0]);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve scene.' });
  }
});

router.put('/:id', isAuthenticated, (req, res, next) => {
  upload.single('video')(req, res, (err) => {
    if (err) return res.status(400).send({ message: `Upload error: ${err.message}` });
    next();
  });
}, async (req, res) => {
  try {
    let { title, part_id, prepend_scene_id, append_scene_id, existing_video_filename } = req.body;
    if (!part_id || part_id === 'null' || part_id === 'undefined') part_id = null;
    let summary = null;
    const hasConcatRequest = (prepend_scene_id && prepend_scene_id !== 'null') || (append_scene_id && append_scene_id !== 'null');
    if (req.file || existing_video_filename || hasConcatRequest) {
      const [currRows] = await dbPool.query('SELECT video_path, thumbnail_path, title FROM scenes WHERE id = ?', [req.params.id]);
      if (currRows.length === 0) return res.status(404).send({ message: 'Scene not found.' });
      let workVideoPath, finalVideoFilename;
      if (req.file) {
        workVideoPath = req.file.path;
        finalVideoFilename = req.file.filename;
      } else if (existing_video_filename) {
        finalVideoFilename = existing_video_filename;
        workVideoPath = path.join(videosDir, finalVideoFilename);
      } else {
        const ext = path.extname(currRows[0].video_path);
        finalVideoFilename = `video-${Date.now()}-${Math.round(Math.random() * 1E9)}${ext}`;
        workVideoPath = path.join(videosDir, finalVideoFilename);
        await fs.copyFile(getAbsPath(currRows[0].video_path), workVideoPath);
      }
      const thumbnailFilename = `thumb-${path.parse(finalVideoFilename).name}.png`;
      const thumbnailFullPath = path.join(thumbnailsDir, thumbnailFilename);
      await processVideoAndThumbnail(workVideoPath, prepend_scene_id, append_scene_id, thumbnailFullPath);
      const videoUrl = `/videos/${finalVideoFilename}`;
      const thumbnailUrl = `/thumbnails/${thumbnailFilename}`;
      await fs.unlink(getAbsPath(currRows[0].video_path)).catch(() => {});
      await fs.unlink(getAbsPath(currRows[0].thumbnail_path)).catch(() => {});
      await dbPool.query('UPDATE scenes SET title = ?, part_id = ?, video_path = ?, thumbnail_path = ? WHERE id = ?', [title, part_id, videoUrl, thumbnailUrl, req.params.id]);
      summary = { mergedVideos: [title], brokenLinks: [] };
      if (prepend_scene_id && prepend_scene_id !== 'null' && prepend_scene_id != req.params.id) {
        const report = await deleteSceneInternal(prepend_scene_id);
        if (report) {
          summary.mergedVideos.unshift(report.title);
          summary.brokenLinks.push(...report.brokenLinks.map(l => ({ ...l, scene: report.title })));
        }
      }
      if (append_scene_id && append_scene_id !== 'null' && append_scene_id != req.params.id) {
        const report = await deleteSceneInternal(append_scene_id);
        if (report) {
          summary.mergedVideos.push(report.title);
          summary.brokenLinks.push(...report.brokenLinks.map(l => ({ ...l, scene: report.title })));
        }
      }
    } else {
      await dbPool.query('UPDATE scenes SET title = ?, part_id = ? WHERE id = ?', [title, part_id, req.params.id]);
    }
    res.send({ message: 'Scene updated successfully.', concatenationSummary: summary });
  } catch (error) {
    console.error('Update error:', error);
    res.status(500).send({ message: 'Failed to update scene: ' + error.message });
  }
});

router.delete('/:id', isAuthenticated, async (req, res) => {
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

router.get('/:id/choices', async (req, res) => {
  try {
    const [rows] = await dbPool.query(`SELECT c.id, c.choice_text, s.id as destination_id, s.title as destination_title FROM choices c JOIN scenes s ON c.destination_scene_id = s.id WHERE c.source_scene_id = ?`, [req.params.id]);
    res.send(rows);
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to retrieve choices.' });
  }
});

router.post('/:id/choices', isAuthenticated, async (req, res) => {
  const { destination_scene_id, choice_text } = req.body;
  try {
    const [result] = await dbPool.query('INSERT INTO choices (source_scene_id, destination_scene_id, choice_text) VALUES (?, ?, ?)', [req.params.id, destination_scene_id, choice_text]);
    res.status(201).send({ id: result.insertId });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to add choice.' });
  }
});

module.exports = router;
