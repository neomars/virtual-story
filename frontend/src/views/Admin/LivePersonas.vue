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
        <label>Voix TTS (selon votre serveur, ex. <code>ff_siwis</code>) <input v-model="form.voice" /></label>
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
const existing = computed(() => !!form.value && personas.value.some((p) => p.id === form.value.id))

const headers = () => {
  const t = localStorage.getItem('live_admin_token')
  return t ? { Authorization: `Bearer ${t}` } : {}
}

const blank = () => ({
  id: '', name: '', age: 25, summary: '', personality: '', speaking_style: '', scenario: '',
  boundaries: '', first_message: '', language: 'fr', voice: '', temperature: 0.85,
})

async function load() { personas.value = (await axios.get('/api/live/personas')).data }
function edit(p) { form.value = { ...blank(), ...p, voice: p.voice || '' }; message.value = '' }
function create() { form.value = blank(); message.value = '' }

async function save() {
  const body = { ...form.value, voice: form.value.voice || null }
  try {
    await axios.put(`/api/live/personas/${form.value.id}`, body, { headers: headers() })
    message.value = 'Enregistré'
    await load()
  } catch (e) { message.value = e.response?.data?.message || e.message }
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
</style>
