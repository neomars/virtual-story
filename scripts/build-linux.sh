#!/usr/bin/env bash
# Construit l'application Ubuntu complète (AppImage + .deb) dans ./dist :
#   interface Vue + moteur Rust + llama-server/whisper-server (CUDA) + Electron.
# Prérequis : node ≥ 20, npm, Rust (cargo) — et pour les sidecars : voir scripts/build-sidecars.sh.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

need() { command -v "$1" >/dev/null 2>&1 || { echo "✗ « $1 » est introuvable. $2" >&2; exit 1; }; }
need node "Installe Node.js ≥ 20"
need npm "Installe npm"
need cargo "Installe Rust : https://rustup.rs"

echo "→ dépendances npm"
npm install --no-audit --no-fund
(cd backend && npm install --no-audit --no-fund)
(cd frontend && npm install --no-audit --no-fund)

echo "→ interface (Vue)"
npm run build:frontend

echo "→ moteur (Rust)"
npm run build:engine

echo "→ sidecars CUDA (llama-server, whisper-server)"
bash scripts/build-sidecars.sh

echo "→ paquets Ubuntu"
npx electron-builder --linux

echo "✓ Terminé :"; ls -lh dist/*.AppImage dist/*.deb 2>/dev/null || ls dist
