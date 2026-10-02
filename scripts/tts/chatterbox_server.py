#!/usr/bin/env python3
"""Serveur de synthèse vocale compatible OpenAI (/v1/audio/speech) pour Chatterbox multilingue.

Lancé et arrêté par live-engine. Le modèle est chargé AVANT l'ouverture du port : dès que le port répond,
la voix est prête (même convention que whisper-server).

  POST /v1/audio/speech  {"input": "...", "voice": "camille", "exaggeration": 0.7, "cfg_weight": 0.3,
                          "temperature": 0.8, "language": "fr"}      → audio/wav (24 kHz, mono)
  GET  /v1/voices        → {"voices": ["camille", ...]}   (fichiers de référence de --voices-dir)
  GET  /health           → {"status": "ok"}

La voix vient d'un court échantillon (≈ 10-20 s, une seule personne, sans bruit ni musique) placé dans
--voices-dir sous le nom <voix>.wav / .mp3 / .flac / .ogg. Sans échantillon : voix par défaut du modèle.
TTS_FAKE=1 : génère un simple bip (tests sans GPU ni modèle).
"""
import argparse
import array
import io
import json
import math
import os
import re
import sys
import threading
import wave
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

AUDIO_EXT = (".wav", ".mp3", ".flac", ".ogg")
NAME_RE = re.compile(r"^[A-Za-z0-9_-]{1,64}$")
MAX_CHARS = 1200


def log(*a):
    print(*a, file=sys.stderr, flush=True)


class Backend:
    def __init__(self, a):
        self.a = a
        self.fake = os.environ.get("TTS_FAKE") == "1"
        self.lock = threading.Lock()
        self.cur_voice = None
        self.sr = 24000
        self.model = None
        self.default_conds = None
        if not self.fake:
            self._load()

    def _load(self):
        import torch  # noqa: import tardif : lent, et inutile en mode TTS_FAKE
        from chatterbox.mtl_tts import ChatterboxMultilingualTTS

        device = self.a.device
        if device == "cuda" and not torch.cuda.is_available():
            log("! CUDA indisponible : repli sur le CPU (très lent)")
            device = "cpu"
        model_dir = Path(self.a.model_dir)
        log(f"chargement de Chatterbox multilingue ({device}) depuis {model_dir} …")
        self.model = None
        if hasattr(ChatterboxMultilingualTTS, "from_local") and any(model_dir.glob("*.safetensors")):
            try:
                self.model = ChatterboxMultilingualTTS.from_local(str(model_dir), device)
            except Exception as e:  # noqa: BLE001 — format de fichiers inattendu : on tente le repli
                log(f"! chargement local impossible ({e!r}) : repli sur from_pretrained")
        if self.model is None:
            # Repli : la bibliothèque télécharge elle-même (cache HF_HOME dans le dossier de l'app).
            log("téléchargement par la bibliothèque (première utilisation, plusieurs Go)")
            self.model = ChatterboxMultilingualTTS.from_pretrained(device=device)
        self.sr = getattr(self.model, "sr", 24000)
        self.default_conds = getattr(self.model, "conds", None)
        log("modèle chargé")

    def voices(self):
        d = Path(self.a.voices_dir)
        if not d.is_dir():
            return []
        return sorted({p.stem for p in d.iterdir() if p.suffix.lower() in AUDIO_EXT and NAME_RE.match(p.stem)})

    def voice_path(self, name):
        if not name or not NAME_RE.match(name):
            return None
        for ext in AUDIO_EXT:
            p = Path(self.a.voices_dir) / (name + ext)
            if p.is_file():
                return str(p)
        return None

    def synth(self, text, voice, exaggeration, cfg_weight, temperature, language):
        """Retourne (échantillons int16 sous forme de array('h'), fréquence)."""
        if self.fake:
            hz = 180 + (sum(map(ord, voice or "")) % 120)
            n = int(self.sr * min(0.6, 0.05 + len(text) / 120))
            return array.array("h", (int(9000 * math.sin(2 * math.pi * hz * i / self.sr)) for i in range(n))), self.sr
        import numpy as np

        path = self.voice_path(voice)
        with self.lock:
            kw = dict(exaggeration=exaggeration, cfg_weight=cfg_weight, temperature=temperature)
            if path is not None and path != self.cur_voice:
                kw["audio_prompt_path"] = path  # la voix de référence n'est analysée que si elle change
                self.cur_voice = path
            elif path is None and self.cur_voice is not None:
                if self.default_conds is not None:
                    self.model.conds = self.default_conds
                self.cur_voice = None
            wav = self.model.generate(text, language_id=language, **kw)
        pcm = (np.clip(wav.squeeze().detach().cpu().numpy(), -1.0, 1.0) * 32767).astype("int16")
        return array.array("h", pcm.tolist()), self.sr


def to_wav(samples, sr):
    buf = io.BytesIO()
    with wave.open(buf, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(samples.tobytes())
    return buf.getvalue()


def make_handler(be, a):
    class H(BaseHTTPRequestHandler):
        def log_message(self, *x):
            pass

        def _json(self, code, obj):
            b = json.dumps(obj).encode()
            self.send_response(code)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(b)))
            self.end_headers()
            self.wfile.write(b)

        def do_GET(self):
            if self.path.startswith("/v1/voices"):
                return self._json(200, {"voices": be.voices()})
            self._json(200, {"status": "ok", "fake": be.fake})

        def do_POST(self):
            if not self.path.startswith("/v1/audio/speech"):
                return self._json(404, {"error": "route inconnue"})
            try:
                n = int(self.headers.get("Content-Length", 0))
                req = json.loads(self.rfile.read(n) or b"{}")
                text = str(req.get("input", "")).strip()[:MAX_CHARS]
                if not text:
                    return self._json(400, {"error": "champ « input » vide"})
                num = lambda k, d: float(req[k]) if req.get(k) is not None else d  # noqa: E731
                samples, sr = be.synth(
                    text,
                    req.get("voice"),
                    min(2.0, max(0.25, num("exaggeration", a.exaggeration))),
                    min(1.0, max(0.0, num("cfg_weight", a.cfg_weight))),
                    min(1.5, max(0.1, num("temperature", a.temperature))),
                    str(req.get("language") or a.language),
                )
                body = to_wav(samples, sr)
            except Exception as e:  # noqa: BLE001 — on renvoie l'erreur au moteur, qui la journalise
                log("erreur de synthèse :", repr(e))
                return self._json(500, {"error": str(e)})
            self.send_response(200)
            self.send_header("Content-Type", "audio/wav")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    return H


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=8880)
    p.add_argument("--model-dir", default=".")
    p.add_argument("--voices-dir", default="voices")
    p.add_argument("--device", default="cuda")
    p.add_argument("--language", default="fr")
    p.add_argument("--exaggeration", type=float, default=0.6)
    p.add_argument("--cfg-weight", type=float, default=0.35)
    p.add_argument("--temperature", type=float, default=0.8)
    a = p.parse_args()
    Path(a.voices_dir).mkdir(parents=True, exist_ok=True)
    be = Backend(a)
    srv = ThreadingHTTPServer((a.host, a.port), make_handler(be, a))
    log(f"serveur de voix prêt sur http://{a.host}:{a.port} (fake={be.fake}, voix : {be.voices() or 'défaut'})")
    srv.serve_forever()


if __name__ == "__main__":
    main()
