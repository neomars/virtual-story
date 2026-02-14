
const { pool } = require('./db');

async function inspect() {
  try {
    const [parts] = await pool.query("SELECT * FROM parts");
    console.log("PARTS:");
    console.table(parts);

    const [scenes] = await pool.query("SELECT id, title, part_id FROM scenes");
    console.log("SCENES:");
    console.table(scenes);

    const [choices] = await pool.query("SELECT * FROM choices");
    console.log("CHOICES:");
    console.table(choices);
  } catch (e) {
    console.error(e);
  } finally {
    process.exit();
  }
}

inspect();
