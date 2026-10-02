#!/usr/bin/env bash
# Installe le moteur de voix (Chatterbox multilingue, PyTorch/CUDA) dans un environnement Python isolé.
# ≈ 3 Go de téléchargement. Lancé par l'app (« Installer le moteur de voix ») ou à la main.
set -euo pipefail
VENV="${VS_TTS_VENV:-${XDG_DATA_HOME:-$HOME/.local/share}/virtual-story/tts-venv}"
PYV="${TTS_PYTHON:-3.11}"          # Chatterbox est validé sur Python 3.11 (Ubuntu 24.04 fournit 3.12)
mkdir -p "$(dirname "$VENV")"

echo "→ environnement : $VENV (Python $PYV)"
UV=""
if command -v uv >/dev/null 2>&1; then UV="$(command -v uv)"
else
  echo "→ installation de uv (gestionnaire Python)"
  BOOT="$(dirname "$VENV")/.uv-boot"
  python3 -m venv "$BOOT"
  "$BOOT/bin/pip" install --quiet uv
  UV="$BOOT/bin/uv"
fi

[ -x "$VENV/bin/python" ] || "$UV" venv --python "$PYV" "$VENV"
echo "→ installation de Chatterbox (peut durer plusieurs minutes)"
"$UV" pip install --python "$VENV/bin/python" chatterbox-tts
"$VENV/bin/python" - <<'PY'
import torch
print("PyTorch", torch.__version__, "— CUDA disponible :", torch.cuda.is_available())
PY
touch "$VENV/.ready"
echo "✓ moteur de voix installé"
