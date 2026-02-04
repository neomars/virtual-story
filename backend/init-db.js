
const mysql = require('mysql2/promise');
const { dbConfig } = require('./db'); // Importe la configuration centralisée

const createPartsTableSQL = `
  CREATE TABLE IF NOT EXISTS parts (
    id INT AUTO_INCREMENT PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    first_scene_id INT,
    \`order\` INT DEFAULT 0,
    loop_video_path VARCHAR(255)
  );
`;

const createScenesTableSQL = `
  CREATE TABLE IF NOT EXISTS scenes (
    id INT AUTO_INCREMENT PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    video_path VARCHAR(255) NOT NULL,
    thumbnail_path VARCHAR(255) NOT NULL,
    part_id INT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (part_id) REFERENCES parts(id) ON DELETE SET NULL
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

const createSettingsTableSQL = `
  CREATE TABLE IF NOT EXISTS settings (
    setting_key VARCHAR(50) PRIMARY KEY,
    setting_value VARCHAR(255)
  );
`;

const insertDefaultSettingsSQL = `
  INSERT INTO settings (setting_key, setting_value)
  VALUES ('player_background', NULL)
  ON DUPLICATE KEY UPDATE setting_key=setting_key;
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
    await connection.query(createPartsTableSQL);
    await connection.query(createScenesTableSQL);
    await connection.query(createChoicesTableSQL);
    await connection.query(createSettingsTableSQL);

    // Assurer le lien SQL de parts vers scenes (first_scene_id)
    try {
      console.log('Vérification du lien SQL parts -> scenes...');
      await connection.query("ALTER TABLE parts ADD CONSTRAINT fk_parts_first_scene FOREIGN KEY (first_scene_id) REFERENCES scenes(id) ON DELETE SET NULL;");
    } catch (e) {
      // Le lien existe probablement déjà ou les tables sont vides
    }

    // Migration pour les installations existantes : ajout de la colonne part_id si absente
    const [columns] = await connection.query("SHOW COLUMNS FROM scenes LIKE 'part_id'");
    if (columns.length === 0) {
      console.log('Migration : Ajout de la colonne "part_id" à la table "scenes"...');
      await connection.query("ALTER TABLE scenes ADD COLUMN part_id INT, ADD CONSTRAINT fk_part_id FOREIGN KEY (part_id) REFERENCES parts(id) ON DELETE SET NULL;");
    }

    console.log('Tables "parts", "scenes", "choices" et "settings" prêtes.');

    // Insère les paramètres par défaut
    await connection.query(insertDefaultSettingsSQL);
    console.log('Paramètres par défaut assurés.');

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
