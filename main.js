const { app, BrowserWindow, dialog } = require('electron');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');
const http = require('http');

const ENGINE_PORT = 3001;
const WEB_PORT = 3000;

// ---- Emplacements --------------------------------------------------------------------------
// App empaquetée (AppImage/deb) : le dossier de l'app est en lecture seule → tout ce qui est
// modifiable (base, médias, modèles IA) vit dans le dossier utilisateur.
const userData = app.getPath('userData'); // ~/.config/Virtual Story
const dataDir = process.env.VS_DATA_DIR || userData;
const modelsDir =
  process.env.VS_MODELS_DIR ||
  path.join(process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local', 'share'), 'virtual-story', 'models');
const resources = app.isPackaged ? process.resourcesPath : __dirname;
const engineBin = app.isPackaged
  ? path.join(resources, 'engine', 'live-engine')
  : path.join(__dirname, 'live-engine', 'target', 'release', 'live-engine');
const binDir = app.isPackaged ? path.join(resources, 'bin') : path.join(__dirname, 'bin');

let mainWindow = null;
let engine = null;
let quitting = false;

function logStream(name) {
  fs.mkdirSync(path.join(userData, 'logs'), { recursive: true });
  return fs.createWriteStream(path.join(userData, 'logs', name), { flags: 'a' });
}

// ---- Moteur IA (Rust) ----------------------------------------------------------------------
function startEngine() {
  if (!fs.existsSync(engineBin)) {
    console.warn(`[live] moteur introuvable (${engineBin}) : mode IA indisponible. Compile-le avec scripts/build-linux.sh`);
    return;
  }
  const log = logStream('live-engine.log');
  fs.mkdirSync(modelsDir, { recursive: true });
  engine = spawn(engineBin, [], {
    // stdin en tube : si l'app meurt, le tube se ferme et le moteur décharge les modèles tout seul.
    stdio: ['pipe', 'pipe', 'pipe'],
    env: {
      ...process.env,
      LIVE_BIND: `127.0.0.1:${ENGINE_PORT}`,
      LIVE_DATA_DIR: path.join(dataDir, 'live'),
      LIVE_MODELS_DIR: modelsDir,
      LIVE_BIN_DIR: binDir,
      LIVE_UPLOADS_DIR: path.join(dataDir, 'uploads'),
      LIVE_FRONTEND_DIR: path.join(dataDir, 'no-frontend'), // l'interface est servie par Express
      LIVE_EXIT_ON_STDIN_EOF: '1',
    },
  });
  engine.stdout.pipe(log);
  engine.stderr.pipe(log);
  engine.on('exit', (code, signal) => {
    console.warn(`[live] moteur arrêté (code ${code}, signal ${signal})`);
    engine = null;
  });
  engine.on('error', (e) => console.error('[live] impossible de lancer le moteur :', e.message));
}

function stopEngine() {
  if (!engine) return Promise.resolve();
  const e = engine;
  return new Promise((resolve) => {
    const kill = setTimeout(() => e.kill('SIGKILL'), 8000);
    e.once('exit', () => { clearTimeout(kill); resolve(); });
    e.kill('SIGTERM'); // le moteur décharge les modèles (libère le GPU) avant de quitter
  });
}

// ---- Fenêtre -------------------------------------------------------------------------------
function waitForServer(port, tries = 60) {
  return new Promise((resolve) => {
    const attempt = (n) => {
      const req = http.get({ host: '127.0.0.1', port, path: '/', timeout: 1000 }, (res) => { res.resume(); resolve(true); });
      req.on('error', () => (n > 0 ? setTimeout(() => attempt(n - 1), 250) : resolve(false)));
      req.on('timeout', () => req.destroy());
    };
    attempt(tries);
  });
}

async function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 800,
    autoHideMenuBar: true,
    webPreferences: { nodeIntegration: false, contextIsolation: true },
  });
  // Micro pour la reconnaissance vocale : autorisé uniquement pour notre propre page locale.
  mainWindow.webContents.session.setPermissionRequestHandler((wc, permission, cb) => {
    cb(permission === 'media' && wc.getURL().startsWith(`http://localhost:${WEB_PORT}`));
  });
  if (!(await waitForServer(WEB_PORT))) {
    dialog.showErrorBox('Virtual Story', `Le serveur local n'a pas démarré sur le port ${WEB_PORT}.`);
    return;
  }
  mainWindow.loadURL(`http://localhost:${WEB_PORT}`);
}

// ---- Cycle de vie --------------------------------------------------------------------------
if (!app.requestSingleInstanceLock()) {
  app.quit(); // une seule instance : sinon deux moteurs se disputeraient les ports et le GPU
} else {
  app.on('second-instance', () => {
    if (mainWindow) { if (mainWindow.isMinimized()) mainWindow.restore(); mainWindow.focus(); }
  });

  app.whenReady().then(() => {
    process.env.PORT = String(WEB_PORT);
    process.env.HOST = '127.0.0.1'; // l'app est locale : pas d'exposition sur le réseau
    process.env.NODE_ENV = 'production';
    process.env.VS_DATA_DIR = dataDir;
    fs.mkdirSync(dataDir, { recursive: true });
    startEngine();
    require('./backend/server.js');
    createWindow();
  });

  app.on('before-quit', (ev) => {
    if (quitting || !engine) return;
    ev.preventDefault();
    quitting = true;
    stopEngine().finally(() => app.quit());
  });

  app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') app.quit();
  });
}
