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

rm -f "$VENV/.ready"                 # tant que l'installation n'est pas validée, le moteur de voix n'est pas « prêt »
[ -x "$VENV/bin/python" ] || "$UV" venv --python "$PYV" "$VENV"
echo "→ installation de Chatterbox (peut durer plusieurs minutes)"
# setuptools<81 : le filigrane audio « perth » de Chatterbox importe pkg_resources, retiré des setuptools récents
# (≥ 82) et absent d'un environnement créé par uv. Sans lui, PerthImplicitWatermarker vaut None et le modèle ne charge pas.
"$UV" pip install --python "$VENV/bin/python" "setuptools<81" chatterbox-tts
"$VENV/bin/python" - <<'PY'
import sys, torch
print("PyTorch", torch.__version__, "— CUDA disponible :", torch.cuda.is_available())
import perth
if getattr(perth, "PerthImplicitWatermarker", None) is None:
    sys.exit("✗ le module de filigrane « perth » ne s'initialise pas (pkg_resources manquant ?)")
print("perth : OK")
PY
echo 2 > "$VENV/.ready"             # version de l'installation (voir TTS_RUNTIME_VERSION dans live-engine/src/engine.rs)
echo "✓ moteur de voix installé"
