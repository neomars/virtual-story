const express = require('express');
const router = express.Router();
const { pool: dbPool } = require('../db');

router.get('/background', async (req, res) => {
    try {
        const [rows] = await dbPool.query("SELECT setting_value FROM settings WHERE setting_key = 'player_background'");
        res.send({ backgroundUrl: rows.length > 0 ? rows[0].setting_value : null });
    } catch (dbError) {
        console.error('Database error:', dbError);
        res.status(500).send({ message: 'Failed to retrieve background setting.' });
    }
});

module.exports = router;
