// Enregistrement micro → WAV PCM 16 bits, 16 kHz, mono (format attendu par whisper.cpp).

function encodeWav(samples, sampleRate) {
  const buf = new ArrayBuffer(44 + samples.length * 2)
  const v = new DataView(buf)
  const str = (o, s) => [...s].forEach((c, i) => v.setUint8(o + i, c.charCodeAt(0)))
  str(0, 'RIFF'); v.setUint32(4, 36 + samples.length * 2, true); str(8, 'WAVE')
  str(12, 'fmt '); v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, 1, true)
  v.setUint32(24, sampleRate, true); v.setUint32(28, sampleRate * 2, true)
  v.setUint16(32, 2, true); v.setUint16(34, 16, true)
  str(36, 'data'); v.setUint32(40, samples.length * 2, true)
  for (let i = 0; i < samples.length; i++) {
    const s = Math.max(-1, Math.min(1, samples[i]))
    v.setInt16(44 + i * 2, s < 0 ? s * 0x8000 : s * 0x7fff, true)
  }
  return buf
}

async function toWav16k(blob) {
  const ctx = new (window.AudioContext || window.webkitAudioContext)()
  const decoded = await ctx.decodeAudioData(await blob.arrayBuffer())
  ctx.close()
  const length = Math.ceil(decoded.duration * 16000)
  const off = new OfflineAudioContext(1, length, 16000)
  const src = off.createBufferSource()
  src.buffer = decoded
  src.connect(off.destination) // le mixage mono est fait par le contexte hors-ligne
  src.start()
  const rendered = await off.startRendering()
  return encodeWav(rendered.getChannelData(0), 16000)
}

export class Recorder {
  constructor() { this.rec = null; this.chunks = []; this.stream = null }

  async start() {
    this.stream = await navigator.mediaDevices.getUserMedia({
      audio: { echoCancellation: true, noiseSuppression: true },
    })
    this.chunks = []
    this.rec = new MediaRecorder(this.stream)
    this.rec.ondataavailable = (e) => e.data.size && this.chunks.push(e.data)
    this.rec.start()
  }

  /** Arrête et renvoie un ArrayBuffer WAV (ou null si trop court). */
  stop() {
    return new Promise((resolve) => {
      if (!this.rec) return resolve(null)
      this.rec.onstop = async () => {
        this.stream.getTracks().forEach((t) => t.stop())
        const blob = new Blob(this.chunks, { type: this.rec.mimeType })
        try {
          const wav = await toWav16k(blob)
          resolve(wav.byteLength > 44 + 16000 * 2 * 0.3 ? wav : null) // < 0,3 s : on ignore
        } catch { resolve(null) }
      }
      this.rec.stop()
    })
  }
}
