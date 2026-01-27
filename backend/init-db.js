
require('dotenv').config();
const mysql = require('mysql2/promise');

// --- Database Configuration ---
// Loaded from the .env file
const dbConfig = {
  host: process.env.DB_HOST || 'localhost',
  user: process.env.DB_USER || 'root',
  password: process.env.DB_PASSWORD || 'password',
  database: process.env.DB_NAME || 'virtual_story_db'
};

// --- SQL Statements for Table Creation ---
const createScenesTableSQL = `
  CREATE TABLE IF NOT EXISTS scenes (
    id INT AUTO_INCREMENT PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    video_path VARCHAR(255) NOT NULL,
    thumbnail_path VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
  );
`;

const createChoicesTableSQL = `
  CREATE TABLE IF NOT EXISTS choices (
    id INT AUTO_INCREMENT PRIMARY KEY,
    source_scene_id INT NOT NULL,
    destination_scene_id INT NOT NULL,
    choice_text VARCHAR(255) NOT NULL,
    FOREIGN KEY (source_scene_id) REFERENCES scenes(id) ON DELETE CASCADE,
    FOREIGN KEY (destination_scene_id) REFERENCES scenes(id) ON DELETE CASCADE
  );
`;

/**
 * Connects to the database, creates it if it doesn't exist,
 * and creates the necessary tables.
 */
async function initializeDatabase() {
  let connection;
  try {
    // Connect to the MySQL server (without specifying a database)
    connection = await mysql.createConnection({
      host: dbConfig.host,
      user: dbConfig.user,
      password: dbConfig.password,
    });

    // Create the database if it doesn't exist
    await connection.query(`CREATE DATABASE IF NOT EXISTS \`${dbConfig.database}\`;`);
    console.log(`Database "${dbConfig.database}" is ready.`);

    // Close the initial connection and reconnect to the specific database
    await connection.end();
    connection = await mysql.createConnection(dbConfig);

    // Create the tables
    await connection.query(createScenesTableSQL);
    await connection.query(createChoicesTableSQL);
    console.log('Tables "scenes" and "choices" have been created or already exist.');

  } catch (error) {
    console.error('Error initializing database:', error);
    process.exit(1); // Exit with an error code
  } finally {
    if (connection) {
      await connection.end();
    }
  }
}

// Run the initialization function
initializeDatabase();
