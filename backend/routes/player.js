const express = require('express');
const router = express.Router();
const { pool: dbPool } = require('../db');

router.get('/scenes/:id', async (req, res) => {
  const sceneId = req.params.id;
  try {
    const [sceneRows] = await dbPool.query(`
      SELECT s.*,
             COALESCE(p1.loop_video_path, p2.loop_video_path) as part_loop_video_path,
             COALESCE(s.part_id, p2.id) as effective_part_id
      FROM scenes s
      LEFT JOIN parts p1 ON s.part_id = p1.id
      LEFT JOIN parts p2 ON s.id = p2.first_scene_id
      WHERE s.id = ?`, [sceneId]);

    if (sceneRows.length === 0) return res.status(404).send({ message: 'Scene not found.' });

    let currentScene = sceneRows[0];
    if (currentScene.effective_part_id) {
        currentScene.part_id = currentScene.effective_part_id;
    }

    let inheritedPartId = null;
    let inheritedLoopVideo = null;

    if (!currentScene.part_loop_video_path) {
      let currentId = sceneId;
      const visited = new Set([currentId]);
      for (let depth = 0; depth < 10; depth++) {
        const [parents] = await dbPool.query("SELECT source_scene_id FROM choices WHERE destination_scene_id = ?", [currentId]);
        if (parents.length === 0) break;
        const parentIds = parents.map(p => p.source_scene_id);
        const [parentScenes] = await dbPool.query(`
          SELECT s.id, s.part_id, p.id as starting_part_id, p.loop_video_path
          FROM scenes s
          LEFT JOIN parts p ON s.id = p.first_scene_id
          WHERE s.id IN (?)`, [parentIds]);

        const partParent = parentScenes.find(p => p.part_id !== null || p.starting_part_id !== null);
        if (partParent) {
          inheritedPartId = partParent.part_id || partParent.starting_part_id;
          if (partParent.loop_video_path) {
            inheritedLoopVideo = partParent.loop_video_path;
          } else {
            const [partData] = await dbPool.query("SELECT loop_video_path FROM parts WHERE id = ?", [inheritedPartId]);
            if (partData.length > 0) inheritedLoopVideo = partData[0].loop_video_path;
          }
          break;
        }
        currentId = parentIds[0];
        if (visited.has(currentId)) break;
        visited.add(currentId);
      }
    }

    if (inheritedPartId) {
      currentScene.part_id = inheritedPartId;
      currentScene.part_loop_video_path = inheritedLoopVideo;
    }

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

module.exports = router;
