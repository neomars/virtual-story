const express = require('express');
const session = require('express-session');
const helmet = require('helmet');
const cors = require('cors');
const path = require('path');
const fs = require('fs').promises;

const { apiLimiter } = require('./middleware/rateLimiter');

const liveProxy = require('./liveProxy');
const { uploadsDir } = require('./utils/paths');

const app = express();
const PORT = process.env.PORT || 3000;
const HOST = process.env.HOST || '0.0.0.0';
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

// Session configuration
app.use(session({
  secret: process.env.SESSION_SECRET || 'virtual-story-default-secret',
  resave: false,
  saveUninitialized: false,
  name: 'vs.sid',
  cookie: {
    // L'app Electron sert du HTTP local : un cookie « secure » n'y serait jamais posé (connexion impossible).
    secure: isProd && process.env.INSECURE_COOKIES !== '1',
    httpOnly: true,
    sameSite: 'lax',
    maxAge: 24 * 60 * 60 * 1000
  }
}));

// Moteur IA live (Rust) : proxy REST + WebSocket. Placé après la session (pour exiger une connexion sur les
// routes qui modifient quelque chose) et avant express.json() (le corps doit rester un flux).
app.use(liveProxy.middleware);

app.use(express.json());

// Global API Rate Limiter
app.use('/api/', apiLimiter);

// Upload directories setup
const videosDir = path.join(uploadsDir, 'videos');
const thumbnailsDir = path.join(uploadsDir, 'thumbnails');
const partsDir = path.join(uploadsDir, 'parts');
const photosDir = path.join(uploadsDir, 'photos');
const backgroundsDir = uploadsDir;

(async () => {
    try {
        await fs.mkdir(videosDir, { recursive: true });
        await fs.mkdir(thumbnailsDir, { recursive: true });
        await fs.mkdir(partsDir, { recursive: true });
        await fs.mkdir(photosDir, { recursive: true });
    } catch (error) {
        console.error("Error creating upload directories:", error);
    }
})();

// Static assets
app.use('/videos', express.static(videosDir));
app.use('/thumbnails', express.static(thumbnailsDir));
app.use('/parts', express.static(partsDir));
app.use('/photos', express.static(photosDir));
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

// Serve frontend static files
const frontendDistPath = path.join(__dirname, '../frontend/dist');
app.use(express.static(frontendDistPath));

app.get(/.*/, (req, res) => {
  res.sendFile(path.join(frontendDistPath, 'index.html'));
});

// Global error handler
app.use((err, req, res, next) => {
  console.error('Unhandled error:', err);
  const message = isProd ? 'An unexpected error occurred on the server.' : err.message;
  res.status(err.status || 500).send({ message });
});

const server = app.listen(PORT, HOST, () => {
  console.log(`Server is running on http://localhost:${PORT} (listening on ${HOST}).`);
});
server.on('upgrade', (req, socket, head) => {
  if (!liveProxy.upgrade(req, socket, head)) socket.destroy();
});
