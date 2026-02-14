
const { pool } = require('./db');

async function main() {
  try {
    const [parts] = await pool.query("SELECT id, title, first_scene_id, loop_video_path FROM parts");
    console.log("--- PARTS ---");
    console.table(parts);

    const [scenes] = await pool.query("SELECT id, title, part_id FROM scenes");
    console.log("--- SCENES ---");
    console.table(scenes);

    const [choices] = await pool.query("SELECT source_scene_id, destination_scene_id, choice_text FROM choices");
    console.log("--- CHOICES ---");
    console.table(choices);

  } catch (err) {
    console.error(err);
  } finally {
    process.exit();
  }
}
main();
