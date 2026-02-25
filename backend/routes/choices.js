const express = require('express');
const router = express.Router();
const { pool: dbPool } = require('../db');
const { isAuthenticated } = require('../middleware/auth');

router.delete('/:id', isAuthenticated, async (req, res) => {
  try {
    await dbPool.query('DELETE FROM choices WHERE id = ?', [req.params.id]);
    res.send({ message: 'Choice deleted successfully.' });
  } catch (dbError) {
    console.error('Database error:', dbError);
    res.status(500).send({ message: 'Failed to delete choice.' });
  }
});

module.exports = router;
