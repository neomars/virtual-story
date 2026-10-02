const http = require('http');
const net = require('net');

// Relaie /api/live/* (REST + WebSocket) vers le moteur Rust, pour que l'app n'expose qu'un seul port.
const TARGET = { host: '127.0.0.1', port: Number(process.env.LIVE_PORT || 3001) };

const isLive = (url = '') => /^\/api\/live(?:[/?]|$)/.test(url);

function proxyHttp(req, res) {
  const headers = { ...req.headers, host: `${TARGET.host}:${TARGET.port}` };
  const upstream = http.request(
    { ...TARGET, method: req.method, path: req.originalUrl || req.url, headers },
    (up) => {
      res.writeHead(up.statusCode, up.headers);
      up.pipe(res);
    }
  );
  upstream.on('error', () => {
    if (!res.headersSent) res.status(502).send({ message: 'Moteur live injoignable (live-engine démarré ?)' });
    else res.end();
  });
  req.pipe(upstream);
}

// Middleware Express : à placer AVANT express.json() pour que le corps reste un flux.
function middleware(req, res, next) {
  if (isLive(req.originalUrl || req.url)) return proxyHttp(req, res);
  next();
}

// À brancher sur server.on('upgrade', ...) ; renvoie true si la requête a été prise en charge.
function upgrade(req, socket, head) {
  if (!isLive(req.url)) return false;
  const up = net.connect(TARGET.port, TARGET.host, () => {
    const lines = [`${req.method} ${req.url} HTTP/${req.httpVersion}`];
    for (let i = 0; i < req.rawHeaders.length; i += 2) {
      const name = req.rawHeaders[i];
      lines.push(`${name}: ${name.toLowerCase() === 'host' ? `${TARGET.host}:${TARGET.port}` : req.rawHeaders[i + 1]}`);
    }
    up.write(lines.join('\r\n') + '\r\n\r\n');
    if (head && head.length) up.write(head);
    socket.pipe(up);
    up.pipe(socket);
  });
  up.on('error', () => socket.destroy());
  socket.on('error', () => up.destroy());
  socket.on('close', () => up.destroy());
  return true;
}

module.exports = { middleware, upgrade, isLive };
