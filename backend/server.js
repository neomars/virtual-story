const express = require('express');
const session = require('express-session');
const helmet = require('helmet');
const cors = require('cors');
const path = require('path');
const fs = require('fs').promises;

const { apiLimiter } = require('./middleware/rateLimiter');

const app = express();
const PORT = process.env.PORT || 3000;
const isProd = process.env.NODE_ENV === 'production';

// Basic security headers
app.use(helmet({
  contentSecurityPolicy: false,
  crossOriginResourcePolicy: { policy: "cross-origin" }
}));

app.use(cors({
  origin: true,
  credentials: true
}));
app.use(express.json());

// Session configuration
app.use(session({
  secret: process.env.SESSION_SECRET || 'virtual-story-default-secret',
  resave: false,
  saveUninitialized: false,
  name: 'vs.sid',
  cookie: {
    secure: isProd,
    httpOnly: true,
    sameSite: 'lax',
    maxAge: 24 * 60 * 60 * 1000
  }
}));

// Global API Rate Limiter
app.use('/api/', apiLimiter);

// Upload directories setup
const videosDir = path.join(__dirname, 'uploads/videos');
const thumbnailsDir = path.join(__dirname, 'uploads/thumbnails');
const partsDir = path.join(__dirname, 'uploads/parts');
const backgroundsDir = path.join(__dirname, 'uploads');

(async () => {
    try {
        await fs.mkdir(videosDir, { recursive: true });
        await fs.mkdir(thumbnailsDir, { recursive: true });
        await fs.mkdir(partsDir, { recursive: true });
    } catch (error) {
        console.error("Error creating upload directories:", error);
    }
})();

// Static assets
app.use('/videos', express.static(videosDir));
app.use('/thumbnails', express.static(thumbnailsDir));
app.use('/parts', express.static(partsDir));
app.use('/backgrounds', express.static(backgroundsDir));

// Routes
const authRoutes = require('./routes/auth');
const sceneRoutes = require('./routes/scenes');
const partRoutes = require('./routes/parts');
const adminRoutes = require('./routes/admin');
const playerRoutes = require('./routes/player');
const settingRoutes = require('./routes/settings');
const choiceRoutes = require('./routes/choices');

app.use('/api/auth', authRoutes);
app.use('/api/scenes', sceneRoutes);
app.use('/api/parts', partRoutes);
app.use('/api/admin', adminRoutes);
app.use('/api/player', playerRoutes);
app.use('/api/settings', settingRoutes);
app.use('/api/choices', choiceRoutes);

app.get('/', (req, res) => {
  res.send('Backend server is running!');
});

// Global error handler
app.use((err, req, res, next) => {
  console.error('Unhandled error:', err);
  const message = isProd ? 'An unexpected error occurred on the server.' : err.message;
  res.status(err.status || 500).send({ message });
});

app.listen(PORT, '0.0.0.0', () => {
  console.log(`Server is running on http://localhost:${PORT} and listening on all interfaces.`);
});
