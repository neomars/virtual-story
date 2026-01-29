
const { pool } = require('./backend/db');

async function seedDatabase() {
  const conn = await pool.getConnection();
  try {
    console.log("Seeding database with sample data...");

    // Clear existing data
    await conn.query("DELETE FROM choices;");
    await conn.query("DELETE FROM scenes;");
    await conn.query("ALTER TABLE scenes AUTO_INCREMENT = 1;");
    await conn.query("ALTER TABLE choices AUTO_INCREMENT = 1;");


    // Insert Scenes
    await conn.query(
      "INSERT INTO scenes (title, video_path, thumbnail_path) VALUES (?, ?, ?)",
      ["Introduction", "/uploads/video1.mp4", "/uploads/thumb1.png"]
    );
    await conn.query(
      "INSERT INTO scenes (title, video_path, thumbnail_path) VALUES (?, ?, ?)",
      ["The Left Path", "/uploads/video2.mp4", "/uploads/thumb2.png"]
    );
    await conn.query(
      "INSERT INTO scenes (title, video_path, thumbnail_path) VALUES (?, ?, ?)",
      ["The Right Path", "/uploads/video3.mp4", "/uploads/thumb3.png"]
    );

    // Insert Choices
    await conn.query(
      "INSERT INTO choices (source_scene_id, destination_scene_id, choice_text) VALUES (?, ?, ?)",
      [1, 2, "Go Left"]
    );
    await conn.query(
      "INSERT INTO choices (source_scene_id, destination_scene_id, choice_text) VALUES (?, ?, ?)",
      [1, 3, "Go Right"]
    );

    console.log("Database seeded successfully.");
  } catch (error) {
    console.error("Error seeding database:", error);
  } finally {
    conn.release();
    pool.end();
  }
}

seedDatabase();
