const path = require('path');
const fs = require('fs').promises;
const { exec } = require('child_process');
const ffmpegPath = require('ffmpeg-static');
const { pool: dbPool } = require('../db');

const uploadsDir = path.join(__dirname, '../uploads');
const getAbsPath = (vPath) => path.join(uploadsDir, vPath.startsWith('/') ? vPath.slice(1) : vPath);

const deleteSceneInternal = async (sceneId) => {
  try {
    const [rows] = await dbPool.query('SELECT title, video_path, thumbnail_path FROM scenes WHERE id = ?', [sceneId]);
    if (rows.length === 0) return null;
    const scene = rows[0];

    // Identify links to be broken
    const [brokenLinks] = await dbPool.query(`
      SELECT 'incoming' as type, s.title as other_scene, c.choice_text
      FROM choices c JOIN scenes s ON c.source_scene_id = s.id
      WHERE c.destination_scene_id = ?
      UNION ALL
      SELECT 'outgoing' as type, s.title as other_scene, c.choice_text
      FROM choices c JOIN scenes s ON c.destination_scene_id = s.id
      WHERE c.source_scene_id = ?
    `, [sceneId, sceneId]);

    await fs.unlink(getAbsPath(scene.video_path)).catch(err => console.error(`[CLEANUP] Error deleting video ${scene.video_path}:`, err.message));
    await fs.unlink(getAbsPath(scene.thumbnail_path)).catch(err => console.error(`[CLEANUP] Error deleting thumbnail ${scene.thumbnail_path}:`, err.message));

    await dbPool.query('DELETE FROM scenes WHERE id = ?', [sceneId]);
    console.log(`[CLEANUP] Scene ${sceneId} ("${scene.title}") and its files removed.`);

    return {
      id: sceneId,
      title: scene.title,
      brokenLinks
    };
  } catch (err) {
    console.error(`[CLEANUP] Failed to delete scene ${sceneId}:`, err);
    return null;
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
    console.log(`[FFMPEG] Concatenating ${inputs.length} videos using filter_complex for better synchronization...`);
    const tempOutput = videoPath + '.concat.mp4';

    // Build the filter_complex command
    // We scale everything to 720p 30fps to ensure compatibility
    let filterComplex = '';
    let inputArgs = '';
    inputs.forEach((p, i) => {
      inputArgs += `-i "${p}" `;
      filterComplex += `[${i}:v]scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30[v${i}]; `;
    });

    const concatInputs = inputs.map((_, i) => `[v${i}][${i}:a]`).join('');
    filterComplex += `${concatInputs}concat=n=${inputs.length}:v=1:a=1[v][a]`;

    const concatCmd = `unset LD_LIBRARY_PATH; "${ffmpegPath}" ${inputArgs} -filter_complex "${filterComplex}" -map "[v]" -map "[a]" -c:v libx264 -preset fast -crf 23 -c:a aac -b:a 128k -y "${tempOutput}"`;

    console.log(`[FFMPEG] Executing: ${concatCmd}`);

    try {
      await new Promise((resolve, reject) => {
        exec(concatCmd, (err, stdout, stderr) => {
          if (err) {
            console.error('[FFMPEG] Concat error output:', stderr);
            reject(new Error(stderr));
          } else {
            resolve();
          }
        });
      });

      await fs.unlink(videoPath).catch(() => {});
      await fs.rename(tempOutput, videoPath);
      console.log('[FFMPEG] Concatenation successful.');
    } catch (err) {
      await fs.unlink(tempOutput).catch(() => {});
      throw err;
    }
  }

  console.log(`[FFMPEG] Generating thumbnail for '${videoPath}'.`);
  const thumbCmd = `unset LD_LIBRARY_PATH; "${ffmpegPath}" -i "${videoPath}" -ss 00:00:05.000 -vframes 1 -s 320x240 -y "${thumbnailPath}"`;
  await new Promise((resolve, reject) => {
    exec(thumbCmd, (err, stdout, stderr) => {
      if (err) { reject(new Error(stderr)); } else { resolve(); }
    });
  });
};

module.exports = { deleteSceneInternal, processVideoAndThumbnail, getAbsPath };
