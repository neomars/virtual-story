const bcrypt = require('bcrypt');
const { pool } = require('./db');

/**
 * Premier lancement : crée l'utilisateur admin par défaut (admin / admin — à changer dans Admin → Users & Profile)
 * UNIQUEMENT si la base ne contient aucun utilisateur, puis les paramètres par défaut.
 * (Si l'admin a été renommé ou supprimé volontairement, on ne le recrée pas.)
 */
async function ensureDefaults() {
  const [users] = await pool.query('SELECT * FROM users');
  if (users.length === 0) {
    console.log("Première utilisation : création de l'utilisateur admin par défaut (admin / admin).");
    const hash = await bcrypt.hash('admin', 10);
    await pool.query('INSERT INTO users (username, password_hash) VALUES (?, ?)', ['admin', hash]);
  }
  const [bg] = await pool.query("SELECT * FROM settings WHERE setting_key = 'player_background'");
  if (bg.length === 0) {
    await pool.query(`
      INSERT INTO settings (setting_key, setting_value)
      VALUES ('player_background', NULL)
      ON DUPLICATE KEY UPDATE setting_key=setting_key;
    `, ['player_background', null]);
  }
}

module.exports = { ensureDefaults };
