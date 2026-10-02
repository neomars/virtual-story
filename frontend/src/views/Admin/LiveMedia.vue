<template>
  <div class="wrap">
    <h2>Médiathèque live</h2>
    <p class="hint">
      L'IA ne voit que ces annotations : elle choisit les médias par <b>tags</b>, <b>ambiance</b> et <b>intensité</b>.
      Un média n'est proposé que si <b>« Adultes vérifiés »</b> est coché — vérifiez que seuls des adultes consentants y figurent.
    </p>
    <div class="actions">
      <button @click="run('scan')">Scanner les uploads (vidéos + photos)</button>
      <button @click="run('import-legacy')">Importer les scènes existantes</button>
      <button :disabled="!selectedIds.length" @click="bulk({ adults_verified: true })">Vérifier la sélection ({{ selectedIds.length }})</button>
      <button :disabled="!selectedIds.length" @click="bulk({ ambient: true })">Marquer « boucle d'ambiance »</button>
      <span class="muted">{{ message }}</span>
    </div>

    <table>
      <thead>
        <tr><th></th><th>Aperçu</th><th>Titre</th><th>Tags (virgules)</th><th>Ambiance</th><th>Intensité</th><th>Ambiance fond</th><th>Adultes vérifiés</th><th></th></tr>
      </thead>
      <tbody>
        <tr v-for="m in items" :key="m.id">
          <td><input type="checkbox" :value="m.id" v-model="selectedIds" /></td>
          <td class="thumb">
            <video v-if="m.kind === 'video'" :src="m.url" muted preload="metadata" @mouseenter="$event.target.play()" @mouseleave="$event.target.pause()" />
            <img v-else :src="m.url" alt="" />
          </td>
          <td><input v-model="m.title" /></td>
          <td><input v-model="m.tagsText" placeholder="plage, soleil, été" /></td>
          <td><input v-model="m.mood" placeholder="romantique" class="short" /></td>
          <td><input type="number" min="1" max="5" v-model.number="m.intensity" class="num" /></td>
          <td><input type="checkbox" v-model="m.ambient" /></td>
          <td><input type="checkbox" v-model="m.adults_verified" /></td>
          <td><button @click="save(m)">Enregistrer</button></td>
        </tr>
      </tbody>
    </table>
    <p v-if="!items.length" class="muted">Aucun média : cliquez sur « Scanner les uploads ».</p>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import axios from 'axios'

const items = ref([])
const selectedIds = ref([])
const message = ref('')

const headers = () => {
  const t = localStorage.getItem('live_admin_token')
  return t ? { Authorization: `Bearer ${t}` } : {}
}

async function load() {
  const { data } = await axios.get('/api/live/media')
  items.value = data.map((m) => ({ ...m, tagsText: m.tags.join(', ') }))
}

async function run(action) {
  try {
    const { data } = await axios.post(`/api/live/media/${action}`, {}, { headers: headers() })
    message.value = `${data.added} média(s) ajouté(s)`
    await load()
  } catch (e) { message.value = e.response?.data?.message || e.message }
}

async function save(m) {
  const body = {
    title: m.title, description: m.description,
    tags: m.tagsText.split(',').map((t) => t.trim()).filter(Boolean),
    mood: m.mood, intensity: m.intensity, ambient: m.ambient, adults_verified: m.adults_verified,
  }
  try { await axios.patch(`/api/live/media/${m.id}`, body, { headers: headers() }); message.value = 'Enregistré' }
  catch (e) { message.value = e.response?.data?.message || e.message }
}

async function bulk(patch) {
  await axios.post('/api/live/media/bulk', { ids: selectedIds.value, ...patch }, { headers: headers() })
  selectedIds.value = []
  await load()
}

onMounted(load)
</script>

<style scoped>
.wrap { padding: 1.5rem 2rem; overflow: auto; }
.hint, .muted { color: #999; }
.actions { display: flex; gap: .6rem; flex-wrap: wrap; align-items: center; margin: 1rem 0; }
table { width: 100%; border-collapse: collapse; }
th, td { padding: .4rem; border-bottom: 1px solid #2a2a2a; text-align: left; vertical-align: middle; }
.thumb video, .thumb img { width: 110px; height: 64px; object-fit: cover; border-radius: 4px; }
input:not([type=checkbox]) { width: 100%; box-sizing: border-box; padding: .35rem; background: #1b1b1b; color: #eee; border: 1px solid #444; border-radius: 6px; }
.num { width: 4rem !important; } .short { min-width: 8rem; }
button { cursor: pointer; padding: .4rem .8rem; border-radius: 6px; border: 1px solid #555; background: #2a2a2a; color: #eee; }
button:disabled { opacity: .5; cursor: not-allowed; }
</style>
