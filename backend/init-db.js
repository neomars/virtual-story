
const mysql = require('mysql2/promise');
const { dbConfig } = require('./db'); // Importe la configuration centralisée

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

async function initializeDatabase() {
  let connection;
  try {
    // Crée une connexion SANS spécifier de base de données pour créer la DB elle-même
    connection = await mysql.createConnection({
      host: dbConfig.host,
      user: dbConfig.user,
      password: dbConfig.password,
    });

    // Crée la base de données si elle n'existe pas
    await connection.query(`CREATE DATABASE IF NOT EXISTS \`${dbConfig.database}\`;`);
    console.log(`Base de données "${dbConfig.database}" prête.`);
    await connection.end(); // Ferme la connexion initiale

    // Se reconnecte, mais cette fois à la base de données spécifique
    connection = await mysql.createConnection(dbConfig);

    // Crée les tables
    await connection.query(createScenesTableSQL);
    await connection.query(createChoicesTableSQL);
    console.log('Tables "scenes" et "choices" créées ou déjà existantes.');

  } catch (error) {
    console.error("Erreur lors de l'initialisation de la base de données:", error.message);
    process.exit(1);
  } finally {
    if (connection) {
      await connection.end();
      console.log('Connexion fermée.');
    }
  }
}

initializeDatabase();
