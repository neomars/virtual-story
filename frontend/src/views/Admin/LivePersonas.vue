<template>
  <div class="wrap">
    <h2>Personnages (IA live)</h2>
    <div class="layout">
      <ul class="list">
        <li v-for="p in personas" :key="p.id" :class="{ active: p.id === form.id }" @click="edit(p)">{{ p.name }}</li>
        <li class="new" @click="create">+ Nouveau</li>
      </ul>

      <form v-if="form" class="form" @submit.prevent="save">
        <label>Identifiant <input v-model="form.id" :disabled="existing" pattern="[A-Za-z0-9_-]+" required /></label>
        <label>Nom <input v-model="form.name" required /></label>
        <label>Âge (adulte uniquement) <input type="number" min="18" max="120" v-model.number="form.age" required /></label>
        <label>Présentation <textarea v-model="form.summary" rows="2" /></label>
        <label>Personnalité <textarea v-model="form.personality" rows="4" /></label>
        <label>Style d'expression <textarea v-model="form.speaking_style" rows="3" /></label>
        <label>Scénario / contexte <textarea v-model="form.scenario" rows="4" /></label>
        <label>Limites du personnage <textarea v-model="form.boundaries" rows="2" /></label>
        <label>Premier message (peut contenir des directives <code>[[show: …]]</code>, <code>[[replies: …]]</code>) <textarea v-model="form.first_message" rows="3" /></label>
        <fieldset class="voice">
          <legend>Voix</legend>
          <label>Voix de référence
            <select v-model="form.voice">
              <option value="">(voix par défaut du modèle)</option>
              <option v-for="v in voices" :key="v" :value="v">{{ v }}</option>
            </select>
          </label>
          <div class="upload">
            <input v-model="newVoice" placeholder="nom de la nouvelle voix (ex. camille)" pattern="[A-Za-z0-9_-]+" />
            <input type="file" accept="audio/*" ref="fileEl" />
            <button type="button" @click="uploadVoice">Ajouter</button>
            <button type="button" v-if="form.voice" @click="deleteVoice">Supprimer « {{ form.voice }} »</button>
          </div>
          <p class="hint">Échantillon de 10 à 20 secondes, une seule personne, sans bruit ni musique, avec le ton recherché
            (voix grave, soufflée, posée…). Utilisez votre propre voix ou un enregistrement dont vous détenez les droits.</p>
          <label>Expressivité : <b>{{ form.tts_exaggeration }}</b>
            <input type="range" min="0.25" max="1.5" step="0.05" v-model.number="form.tts_exaggeration" /></label>
          <label>Rythme : <b>{{ form.tts_cfg_weight }}</b> <span class="hint">(plus bas = plus lent et posé)</span>
            <input type="range" min="0" max="1" step="0.05" v-model.number="form.tts_cfg_weight" /></label>
          <div class="upload">
            <input v-model="previewText" />
            <button type="button" :disabled="previewing" @click="preview">{{ previewing ? '…' : 'Écouter' }}</button>
          </div>
        </fieldset>
        <label>Créativité (température) <input type="number" step="0.05" min="0.1" max="1.5" v-model.number="form.temperature" /></label>
        <div class="row">
          <button class="primary" type="submit">Enregistrer</button>
          <button type="button" v-if="existing" @click="remove">Supprimer</button>
          <span class="msg">{{ message }}</span>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import axios from 'axios'

const personas = ref([])
const form = ref(null)
const message = ref('')
const voices = ref([])
const newVoice = ref('')
const fileEl = ref(null)
const previewText = ref('Bonsoir… je suis contente que tu sois venu. Viens, assieds-toi près de moi.')
const previewing = ref(false)
const existing = computed(() => !!form.value && personas.value.some((p) => p.id === form.value.id))

const headers = () => {
  const t = localStorage.getItem('live_admin_token')
  return t ? { Authorization: `Bearer ${t}` } : {}
}

const blank = () => ({
  id: '', name: '', age: 25, summary: '', personality: '', speaking_style: '', scenario: '',
  boundaries: '', first_message: '', language: 'fr', voice: '', temperature: 0.85,
  tts_exaggeration: 0.6, tts_cfg_weight: 0.35,
})

async function load() {
  personas.value = (await axios.get('/api/live/personas')).data
  await loadVoices()
}
async function loadVoices() {
  try { voices.value = (await axios.get('/api/live/voices')).data.voices } catch { voices.value = [] }
}
function edit(p) {
  form.value = {
    ...blank(), ...p, voice: p.voice || '',
    tts_exaggeration: p.tts_exaggeration ?? 0.6, tts_cfg_weight: p.tts_cfg_weight ?? 0.35,
  }
  message.value = ''
}
function create() { form.value = blank(); message.value = '' }

async function save() {
  const body = { ...form.value, voice: form.value.voice || null }
  try {
    await axios.put(`/api/live/personas/${form.value.id}`, body, { headers: headers() })
    message.value = 'Enregistré'
    await load()
  } catch (e) { message.value = e.response?.data?.message || e.message }
}

async function uploadVoice() {
  const file = fileEl.value?.files?.[0]
  if (!file || !newVoice.value) { message.value = 'Choisissez un fichier audio et donnez un nom à la voix.'; return }
  try {
    await axios.put(`/api/live/voices/${encodeURIComponent(newVoice.value)}`, file,
      { headers: { ...headers(), 'Content-Type': 'application/octet-stream' } })
    await loadVoices()
    form.value.voice = newVoice.value
    newVoice.value = ''
    fileEl.value.value = ''
    message.value = 'Voix ajoutée'
  } catch (e) { message.value = e.response?.data?.message || e.message }
}

async function deleteVoice() {
  if (!confirm(`Supprimer la voix « ${form.value.voice} » ?`)) return
  try {
    await axios.delete(`/api/live/voices/${encodeURIComponent(form.value.voice)}`, { headers: headers() })
    form.value.voice = ''
    await loadVoices()
  } catch (e) { message.value = e.response?.data?.message || e.message }
}

async function preview() {
  previewing.value = true
  message.value = ''
  try {
    const { data } = await axios.post('/api/live/tts/preview', {
      text: previewText.value, voice: form.value.voice || null,
      exaggeration: form.value.tts_exaggeration, cfg_weight: form.value.tts_cfg_weight,
    }, { headers: headers(), responseType: 'blob' })
    const url = URL.createObjectURL(data)
    const a = new Audio(url)
    a.onended = () => URL.revokeObjectURL(url)
    await a.play()
  } catch (e) {
    let msg = e.message
    try { msg = JSON.parse(await e.response.data.text()).message } catch { /* corps non JSON */ }
    message.value = `Écoute impossible : ${msg} (la voix est-elle chargée dans « Modèles IA » ?)`
  } finally { previewing.value = false }
}

async function remove() {
  if (!confirm(`Supprimer ${form.value.name} ?`)) return
  await axios.delete(`/api/live/personas/${form.value.id}`, { headers: headers() })
  form.value = null
  await load()
}

onMounted(load)
</script>

<style scoped>
.wrap { padding: 1.5rem 2rem; overflow: auto; }
.layout { display: flex; gap: 2rem; align-items: flex-start; }
.list { list-style: none; padding: 0; margin: 0; min-width: 180px; }
.list li { padding: .5rem .8rem; cursor: pointer; border-radius: 6px; }
.list li:hover, .list li.active { background: #2a2a2a; }
.list .new { color: #8fb0ff; }
.form { flex: 1; max-width: 720px; display: flex; flex-direction: column; gap: .8rem; }
label { display: flex; flex-direction: column; gap: .25rem; font-size: .9rem; color: #bbb; }
input, textarea { padding: .5rem; background: #1b1b1b; color: #eee; border: 1px solid #444; border-radius: 6px; font: inherit; }
.row { display: flex; gap: .6rem; align-items: center; }
button { cursor: pointer; padding: .5rem 1rem; border-radius: 6px; border: 1px solid #555; background: #2a2a2a; color: #eee; }
.primary { background: #4a6cf7; border-color: #4a6cf7; }
.msg { color: #9c9; }
.voice { border: 1px solid #333; border-radius: 8px; padding: .8rem; display: flex; flex-direction: column; gap: .7rem; }
.voice legend { color: #bbb; padding: 0 .4rem; }
.upload { display: flex; gap: .5rem; align-items: center; flex-wrap: wrap; }
.upload input[type=text], .upload input:not([type]) { flex: 1; min-width: 180px; }
.hint { color: #888; font-size: .82rem; margin: 0; }
select { padding: .5rem; background: #1b1b1b; color: #eee; border: 1px solid #444; border-radius: 6px; }
</style>
