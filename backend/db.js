const alasql = require('alasql');
const fs = require('fs');
const path = require('path');

const DB_FILE = path.resolve(__dirname, 'db.json');

// Helper to save to JSON
function save() {
    const data = {};
    Object.keys(alasql.tables).forEach(table => {
        data[table] = alasql.tables[table].data;
    });
    fs.writeFileSync(DB_FILE, JSON.stringify(data, null, 2));
}

// Initialize tables
const initSql = `
CREATE TABLE IF NOT EXISTS users (id INT AUTO_INCREMENT PRIMARY KEY, username STRING, password_hash STRING);
CREATE TABLE IF NOT EXISTS parts (id INT AUTO_INCREMENT PRIMARY KEY, title STRING, first_scene_id INT, \`order\` INT, loop_video_path STRING);
CREATE TABLE IF NOT EXISTS scenes (id INT AUTO_INCREMENT PRIMARY KEY, title STRING, video_path STRING, thumbnail_path STRING, part_id INT, created_at TIMESTAMP);
CREATE TABLE IF NOT EXISTS choices (id INT AUTO_INCREMENT PRIMARY KEY, source_scene_id INT, destination_scene_id INT, choice_text STRING);
CREATE TABLE IF NOT EXISTS settings (setting_key STRING PRIMARY KEY, setting_value STRING);
`;

initSql.split(';').filter(s => s.trim()).forEach(sql => {
    try {
        alasql(sql);
    } catch (e) {
        console.error("Error creating table:", e.message, "SQL:", sql);
    }
});

// Load data if exists
if (fs.existsSync(DB_FILE)) {
    try {
        const data = JSON.parse(fs.readFileSync(DB_FILE, 'utf8'));
        Object.keys(data).forEach(table => {
            if (alasql.tables[table]) {
                alasql.tables[table].data = data[table];
                // Update AUTO_INCREMENT counter to avoid ID collisions
                if (data[table].length > 0 && alasql.tables[table].identities && alasql.tables[table].identities.id) {
                    const maxId = Math.max(...data[table].map(r => parseInt(r.id) || 0));
                    alasql.tables[table].identities.id.value = maxId + 1;
                }
            }
        });
    } catch (e) {
        console.error("Error loading db.json:", e);
    }
}

const pool = {
    async query(sql, params) {
        const trimmedSql = sql.trim();
        const upperSql = trimmedSql.toUpperCase();

        // Handle MySQL-specific SHOW COLUMNS
        if (upperSql.startsWith('SHOW COLUMNS')) {
            const columnNameMatch = sql.match(/LIKE\s+'([^']+)'/i);
            if (columnNameMatch) {
                const colName = columnNameMatch[1];
                return [[{ Field: colName }], []];
            }
            return [[], []];
        }

        // Handle ON DUPLICATE KEY UPDATE for settings specifically
        // Improved to be slightly less brittle
        if (upperSql.includes('ON DUPLICATE KEY UPDATE') && upperSql.includes('SETTINGS')) {
             const key = params[0];
             const val = params[1];
             const exists = alasql('SELECT * FROM settings WHERE setting_key = ?', [key]);
             if (exists.length > 0) {
                 alasql('UPDATE settings SET setting_value = ? WHERE setting_key = ?', [val, key]);
             } else {
                 alasql('INSERT INTO settings (setting_key, setting_value) VALUES (?, ?)', [key, val]);
             }
             save();
             return [{}, []];
        }

        try {
            // Ignore Runtime Migration/Constraint queries
            if (upperSql.includes('FOREIGN KEY') || upperSql.includes('CONSTRAINT')) {
                return [[], []];
            }

            const result = alasql(trimmedSql, params);

            // Save on writes
            if (trimmedSql.match(/^(INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)/i)) {
                save();
            }

            // Mock mysql2 return format for INSERT
            if (upperSql.startsWith('INSERT')) {
                const tableNameMatch = trimmedSql.match(/INSERT INTO\s+([^\s(]+)/i);
                const tableName = tableNameMatch ? tableNameMatch[1].replace(/`/g, '') : null;
                let insertId = 0;
                if (tableName && alasql.tables[tableName] && alasql.tables[tableName].identities && alasql.tables[tableName].identities.id) {
                    // In Alasql, identities.id.value is the NEXT value to be used.
                    insertId = alasql.tables[tableName].identities.id.value - (alasql.tables[tableName].identities.id.step || 1);
                }
                return [{ insertId }, []];
            }

            // SELECT and other queries return result as rows
            return [result, []];
        } catch (err) {
            console.error('Alasql error:', err.message, 'SQL:', sql);
            // Don't throw for some common migration queries
            if (upperSql.startsWith('ALTER TABLE') || upperSql.startsWith('CREATE TABLE')) {
                return [[], []];
            }
            throw err;
        }
    },
    async getConnection() {
        return {
            query: this.query.bind(this),
            release: () => {}
        };
    },
    async end() {
        save();
    }
};

module.exports = {
    pool,
    dbConfig: {
        database: 'json_db'
    }
};
