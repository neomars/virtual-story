// Fichier : backend/db.js

// Charge les variables d'environnement depuis le fichier .env
require('dotenv').config();
const mysql = require('mysql2/promise');

// Objet de configuration centralisé, lu depuis les variables d'environnement
const dbConfig = {
  host: process.env.DB_HOST || 'localhost',
  user: process.env.DB_USER || 'root',
  password: process.env.DB_PASSWORD || 'password',
  database: process.env.DB_NAME || 'virtual_story_db',
  waitForConnections: true,
  connectionLimit: 10, // Nombre maximum de connexions dans le pool
  queueLimit: 0
};

// Création du pool de connexions qui sera partagé dans toute l'application
const dbPool = mysql.createPool(dbConfig);

// Test de la connexion au démarrage
dbPool.getConnection()
  .then(conn => {
    console.log('Connexion à la base de données établie avec succès via le pool.');
    conn.release(); // Libère la connexion pour qu'elle retourne dans le pool
  })
  .catch(err => {
    console.error('Erreur de connexion à la base de données:', err.message);
    // Dans un vrai cas, on pourrait vouloir arrêter l'application si la DB n'est pas dispo
  });

// Exporte le pool de connexions pour être utilisé par server.js
// et la configuration pour être utilisée par le script d'initialisation
module.exports = {
    dbPool,
    dbConfig
};
