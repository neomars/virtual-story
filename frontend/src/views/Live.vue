<template>
  <div class="live">
    <!-- Scène : ambiance en fond, média principal par-dessus -->
    <div class="stage" aria-hidden="true">
      <video v-if="ambient" :key="ambient.id" :src="ambient.url" class="layer" autoplay loop muted playsinline />
      <Transition name="fade">
        <video v-if="main && main.kind === 'video'" :key="main.id" :src="main.url" class="layer main"
               autoplay playsinline :loop="!ambient" :muted="!mediaSound" @ended="onMainEnded" />
        <img v-else-if="main" :key="main.id" :src="main.url" class="layer main" alt="" />
      </Transition>
      <div class="vignette" />
    </div>

    <!-- Choix du persona -->
    <section v-if="!started" class="panel start">
      <h2>Conversation en direct</h2>
      <label>Personnage
        <select v-model="personaId">
          <option v-for="p in personas" :key="p.id" :value="p.id">{{ p.name }}, {{ p.age }} ans</option>
        </select>
      </label>
      <p v-if="selected" class="muted">{{ selected.summary }}</p>
      <label>Intensité maximale des médias : <b>{{ maxIntensity }}</b> / 5
        <input type="range" min="1" max="5" v-model.number="maxIntensity" />
      </label>
      <p class="status">
        <span :class="['pill', hello.llm && 'ok']">IA</span>
        <span :class="['pill', hello.stt && 'ok']">Micro</span>
        <span :class="['pill', hello.tts && 'ok']">Voix</span>
        <span v-if="!connected" class="muted"> connexion…</span>
      </p>
      <button class="primary" :disabled="!personaId || !connected" @click="start">Commencer</button>
      <p v-if="!personas.length" class="muted">Aucun personnage : créez-en un dans
        <router-link to="/admin/live/personas">l'administration</router-link>.</p>
    </section>

    <!-- Conversation -->
    <section v-else class="chat">
      <div class="toolbar">
        <strong>{{ persona?.name }}</strong>
        <label class="inline">Intensité
          <input type="range" min="1" max="5" v-model.number="maxIntensity" @change="pushSettings" />
          {{ maxIntensity }}
        </label>
        <label class="inline"><input type="checkbox" v-model="voiceOn" @change="pushSettings" /> Voix IA</label>
        <label class="inline"><input type="checkbox" v-model="mediaSound" /> Son des médias</label>
        <button @click="restart">Changer</button>
      </div>

      <div class="log" ref="logEl" aria-live="polite">
        <div v-for="(m, i) in messages" :key="i" :class="['msg', m.role]">
          <span v-html="render(m.text)" />
        </div>
      </div>

      <div v-if="replies.length" class="replies">
        <button v-for="r in replies" :key="r" @click="sendText(r)">{{ r }}</button>
      </div>

      <form class="input" @submit.prevent="sendText(draft)">
        <button type="button" :class="['mic', recording && 'rec']" :disabled="!hello.stt"
                :title="hello.stt ? 'Parler (clic pour démarrer / arrêter)' : 'Micro indisponible'"
                @click="toggleMic">{{ recording ? '■' : '🎤' }}</button>
        <input v-model="draft" placeholder="Écrire…" autocomplete="off" />
        <button class="primary" type="submit" :disabled="!draft.trim()">Envoyer</button>
        <button type="button" v-if="busy" @click="interrupt" title="Couper la parole">⏹</button>
      </form>
      <p v-if="error" class="error">{{ error }}</p>
    </section>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import axios from 'axios'
import { Recorder } from '../utils/audio.js'

const personas = ref([])
const personaId = ref('')
const persona = ref(null)
const selected = computed(() => personas.value.find((p) => p.id === personaId.value))
const started = ref(false)
const connected = ref(false)
const hello = ref({})
const maxIntensity = ref(3)
const voiceOn = ref(true)
const mediaSound = ref(false)
const messages = ref([])
const replies = ref([])
const draft = ref('')
const busy = ref(false)
const recording = ref(false)
const error = ref('')
const main = ref(null)
const ambient = ref(null)
const logEl = ref(null)

let ws = null
let currentGen = 0
let pendingAudio = null
let player = null
const audioQueue = []
const recorder = new Recorder()

function connect() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  ws = new WebSocket(`${proto}://${location.host}/api/live/ws`)
  ws.binaryType = 'arraybuffer'
  ws.onopen = () => (connected.value = true)
  ws.onclose = () => { connected.value = false; setTimeout(() => ws && connect(), 2000) }
  ws.onmessage = onMessage
}

function onMessage(e) {
  if (typeof e.data !== 'string') {
    if (pendingAudio && pendingAudio.gen === currentGen) {
      audioQueue.push(new Blob([e.data], { type: pendingAudio.mime }))
      playNext()
    }
    pendingAudio = null
    return
  }
  const m = JSON.parse(e.data)
  switch (m.type) {
    case 'hello': hello.value = m; break
    case 'persona': persona.value = m.persona; break
    case 'transcript':
      if (m.text) pushMessage('user', m.text)
      break
    case 'token':
      if (m.gen !== currentGen) { currentGen = m.gen; busy.value = true; messages.value.push({ role: 'assistant', text: '' }) }
      messages.value[messages.value.length - 1].text += m.text
      scroll()
      break
    case 'media':
      if (m.mode === 'ambient') ambient.value = m.item
      else main.value = m.item
      break
    case 'replies': replies.value = m.items; break
    case 'audio': pendingAudio = m; break
    case 'done': busy.value = false; break
    case 'error': error.value = m.message; busy.value = false; break
  }
}

function pushMessage(role, text) {
  messages.value.push({ role, text })
  scroll()
}

function render(t) {
  const esc = t.replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c])
  return esc.replace(/\*([^*]+)\*/g, '<em>$1</em>')
}

function scroll() { nextTick(() => logEl.value && (logEl.value.scrollTop = logEl.value.scrollHeight)) }

function stopAudio() {
  audioQueue.length = 0
  if (player) { player.pause(); player = null }
}

function playNext() {
  if (player || !audioQueue.length) return
  const url = URL.createObjectURL(audioQueue.shift())
  player = new Audio(url)
  const done = () => { URL.revokeObjectURL(url); player = null; playNext() }
  player.onended = done
  player.onerror = done
  player.play().catch(done)
}

function sendJson(o) { ws && ws.readyState === 1 && ws.send(JSON.stringify(o)) }

function start() {
  messages.value = []; replies.value = []; main.value = null; ambient.value = null; error.value = ''
  currentGen = 0
  started.value = true
  sendJson({ type: 'start', persona: personaId.value, max_intensity: maxIntensity.value, tts: voiceOn.value })
}

function restart() { interrupt(); started.value = false }

function pushSettings() { sendJson({ type: 'settings', max_intensity: maxIntensity.value, tts: voiceOn.value }) }

function sendText(text) {
  text = (text || '').trim()
  if (!text) return
  stopAudio()
  currentGen = -1 // ignore l'audio de la génération précédente jusqu'au premier token de la nouvelle
  replies.value = []; error.value = ''
  pushMessage('user', text)
  draft.value = ''
  sendJson({ type: 'user', text })
}

function interrupt() { stopAudio(); currentGen = -1; busy.value = false; sendJson({ type: 'interrupt' }) }

async function toggleMic() {
  error.value = ''
  if (!recording.value) {
    try {
      stopAudio(); sendJson({ type: 'interrupt' }); currentGen = -1
      await recorder.start()
      recording.value = true
    } catch { error.value = "Accès au micro refusé." }
  } else {
    recording.value = false
    const wav = await recorder.stop()
    if (wav && ws && ws.readyState === 1) { replies.value = []; ws.send(wav) }
  }
}

function onMainEnded() { if (ambient.value) main.value = null }

onMounted(async () => {
  try {
    personas.value = (await axios.get('/api/live/personas')).data
    personaId.value = personas.value[0]?.id || ''
  } catch { error.value = 'Moteur live injoignable (live-engine démarré ?)' }
  connect()
})
onBeforeUnmount(() => { const s = ws; ws = null; s && s.close(); stopAudio() })
</script>

<style scoped>
.live { position: relative; flex: 1; min-height: 0; display: flex; flex-direction: column; justify-content: flex-end; overflow: hidden; background: #000; }
.stage { position: absolute; inset: 0; }
.layer { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
.vignette { position: absolute; inset: 0; background: linear-gradient(to top, rgba(0,0,0,.85) 0%, rgba(0,0,0,.25) 45%, transparent 70%); pointer-events: none; }
.fade-enter-active, .fade-leave-active { transition: opacity .8s; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
.panel { position: relative; margin: auto; width: min(440px, 92%); padding: 1.5rem; background: rgba(20,20,20,.88); border: 1px solid #333; border-radius: 12px; display: flex; flex-direction: column; gap: .9rem; }
.chat { position: relative; width: min(820px, 100%); margin: 0 auto; padding: 0 1rem 1rem; display: flex; flex-direction: column; gap: .6rem; max-height: 60%; }
.toolbar { display: flex; gap: 1rem; align-items: center; flex-wrap: wrap; font-size: .85rem; }
.inline { display: flex; align-items: center; gap: .4rem; }
.log { overflow-y: auto; display: flex; flex-direction: column; gap: .5rem; padding-right: .3rem; }
.msg { max-width: 85%; padding: .55rem .85rem; border-radius: 14px; line-height: 1.4; background: rgba(30,30,30,.82); backdrop-filter: blur(6px); }
.msg.user { align-self: flex-end; background: rgba(60,80,140,.82); }
.msg :deep(em) { color: #b9b9c9; }
.replies { display: flex; flex-wrap: wrap; gap: .5rem; }
.replies button { border-radius: 999px; background: rgba(40,40,40,.85); border: 1px solid #555; color: #eee; padding: .4rem .9rem; cursor: pointer; }
.replies button:hover { background: #3a3a3a; }
.input { display: flex; gap: .5rem; }
.input input { flex: 1; padding: .6rem .8rem; border-radius: 8px; border: 1px solid #444; background: rgba(20,20,20,.9); color: #eee; }
button { cursor: pointer; padding: .5rem .9rem; border-radius: 8px; border: 1px solid #555; background: #2a2a2a; color: #eee; }
button:disabled { opacity: .5; cursor: not-allowed; }
.primary { background: #4a6cf7; border-color: #4a6cf7; }
.mic.rec { background: #c0392b; border-color: #c0392b; animation: pulse 1s infinite; }
@keyframes pulse { 50% { opacity: .6; } }
.muted { color: #999; font-size: .9rem; }
.error { color: #ff7b7b; margin: 0; }
.pill { padding: .15rem .6rem; border-radius: 999px; background: #5a2a2a; font-size: .8rem; margin-right: .3rem; }
.pill.ok { background: #1f5a34; }
select, label { color: #eee; }
select { width: 100%; padding: .5rem; margin-top: .3rem; background: #1b1b1b; border: 1px solid #444; border-radius: 8px; }
</style>
