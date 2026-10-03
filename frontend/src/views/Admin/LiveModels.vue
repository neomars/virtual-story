<template>
  <div class="wrap">
    <h2>Modèles IA (live)</h2>

    <section class="gpu">
      <div v-if="status.gpu">
        <b>{{ status.gpu.name }}</b> — VRAM {{ gb(status.gpu.used_mb * MB) }} / {{ gb(status.gpu.total_mb * MB) }} utilisés
        <div class="bar"><div :style="{ width: pct(status.gpu.used_mb, status.gpu.total_mb) + '%' }" /></div>
      </div>
      <div v-else class="muted">GPU NVIDIA non détecté (<code>nvidia-smi</code> introuvable) — les calculs utilisent {{ status.configured_vram_gb }} Go de VRAM de la configuration.</div>
      <div v-if="status.managed === false" class="warn">Mode manuel (<code>engine.managed = false</code>) : l'app ne lance pas les serveurs, elle se connecte à ceux que vous avez démarrés.</div>
    </section>

    <section class="cards">
      <div v-for="c in comps" :key="c.key" class="card">
        <div class="title">{{ c.label }} <span :class="['pill', stateOf(c.key)]">{{ stateLabel(stateOf(c.key)) }}</span></div>
        <div class="muted">{{ modelName(status[c.key]?.model) || 'aucun modèle chargé' }}</div>
        <div v-if="status[c.key]?.error" class="err">{{ status[c.key].error }}</div>
        <div v-if="status[c.key]?.warning" class="warn">⚠ {{ status[c.key].warning }}</div>
        <div v-if="c.key === 'llm' && status.llm?.plan" class="muted small">
          {{ status.llm.plan.full_offload ? 'Tout sur le GPU' : 'Offload partiel' }} ·
          {{ status.llm.plan.n_gpu_layers }} couches GPU · contexte {{ status.llm.plan.ctx_tokens }} ·
          VRAM ≈ {{ status.llm.plan.vram_used_gb.toFixed(1) }} / {{ status.llm.plan.vram_budget_gb.toFixed(1) }} Go
          <div v-for="n in status.llm.plan.notes" :key="n">{{ n }}</div>
        </div>
        <div v-if="c.key === 'tts'" class="muted small">
          Moteur de voix : {{ rtLabel }}
          <button v-if="rt === 'absent' || rt === 'error'" class="primary" @click="installRuntime">Installer le moteur de voix (≈ 3 Go)</button>
          <div v-if="status.tts_runtime?.error" class="err">{{ status.tts_runtime.error }}</div>
          <div v-if="rt === 'installing'">Installation en cours… (voir les journaux)</div>
        </div>
        <div class="row">
          <button v-if="stateOf(c.key) !== 'stopped'" @click="unload(c.key)">Décharger</button>
          <button @click="toggleLogs(c.key)">{{ logsFor === c.key ? 'Masquer' : 'Journaux' }}</button>
        </div>
      </div>
    </section>
    <pre v-if="logsFor" class="logs">{{ logLines.join('\n') || '(vide)' }}</pre>
    <p v-if="message" class="msg">{{ message }}</p>

    <template v-for="c in comps" :key="'t' + c.key">
      <h3>{{ c.label }} — catalogue</h3>
      <template v-for="g in groupsFor(c.key)" :key="g.label">
        <h4 v-if="g.label" class="group">{{ g.label }}</h4>
        <table>
          <tbody>
            <tr v-for="m in g.items" :key="m.entry.id">
              <td class="name">
                <b>{{ m.entry.name }}</b>
                <div class="muted small">{{ m.entry.description }}</div>
                <div class="muted small">{{ m.entry.repo }}</div>
              </td>
              <td class="size">
                {{ m.size_bytes ? gb(m.size_bytes) : m.entry.size_gb ? '≈ ' + m.entry.size_gb + ' Go' : '' }}
                <div v-if="c.key === 'llm' && m.entry.size_gb" class="muted small">VRAM ≈ {{ (m.entry.size_gb + 1.2).toFixed(1) }} Go</div>
              </td>
              <td class="act">
                <template v-if="busy(m)">
                  <div class="bar"><div :style="{ width: pct(m.download.downloaded, m.download.total) + '%' }" /></div>
                  <span class="small">{{ m.download.status === 'listing' ? 'recherche du fichier…' : m.download.status === 'verifying' ? 'vérification SHA-256…' : gb(m.download.downloaded) + ' / ' + gb(m.download.total) }}</span>
                  <button @click="cancel(m)">Annuler</button>
                </template>
                <template v-else-if="m.installed">
                  <button class="primary" :disabled="isLoaded(m)" @click="load(m)">{{ isLoaded(m) ? 'Chargé' : 'Charger' }}</button>
                  <button @click="remove(m)">Supprimer</button>
                </template>
                <template v-else>
                  <button class="primary" @click="download(m)">Télécharger</button>
                  <span v-if="m.download?.status === 'error'" class="err small">{{ m.download.error }}</span>
                  <span v-else-if="m.download?.status === 'cancelled'" class="muted small">annulé (reprise possible)</span>
                </template>
              </td>
            </tr>
          </tbody>
        </table>
      </template>
    </template>
    <p class="muted small">
      Les fichiers viennent de Hugging Face (<code>HF_TOKEN</code> pour les dépôts protégés). Les téléchargements reprennent là où ils
      se sont arrêtés et sont vérifiés par SHA-256. Ajoutez vos propres modèles dans <code>catalog.json</code> (dossier de données).
    </p>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import axios from 'axios'

const MB = 1024 * 1024
const comps = [
  { key: 'llm', label: 'IA (texte)' },
  { key: 'stt', label: 'Micro (reconnaissance vocale)' },
  { key: 'tts', label: 'Voix de l\'IA' },
]
const status = ref({})
const models = ref([])
const message = ref('')
const logsFor = ref('')
const logLines = ref([])
let timer = null

const headers = () => {
  const t = localStorage.getItem('live_admin_token')
  return t ? { Authorization: `Bearer ${t}` } : {}
}
const gb = (b) => (b / 1024 ** 3).toFixed(b < 1024 ** 3 ? 2 : 1) + ' Go'
const pct = (a, t) => (t ? Math.min(100, Math.round((a / t) * 100)) : 0)
const byKind = (k) => models.value.filter((m) => m.entry.kind === k)
// Les modèles de texte sont groupés par taille de fichier : on voit tout de suite les plus légers.
const SIZE_GROUPS = [
  { max: 3.5, label: 'Mini — ≈ 2 à 3 Go (téléchargement rapide, tient dans 4 Go de VRAM)' },
  { max: 6, label: 'Léger — ≈ 4 à 5 Go (laisse de la place à la voix et au micro)' },
  { max: 10, label: 'Standard — ≈ 7 à 9 Go' },
  { max: Infinity, label: 'Grand — 10 Go et plus (plus intelligent, un peu en RAM sur 15 Go)' },
]
function groupsFor(kind) {
  const items = [...byKind(kind)].sort((a, b) => (a.entry.size_gb || 0) - (b.entry.size_gb || 0))
  if (kind !== 'llm') return [{ label: '', items }]
  return SIZE_GROUPS.map((g, i) => ({
    label: g.label,
    items: items.filter((m) => (m.entry.size_gb || 0) < g.max && (m.entry.size_gb || 0) >= (SIZE_GROUPS[i - 1]?.max ?? 0)),
  })).filter((g) => g.items.length)
}
const stateOf = (k) => status.value[k]?.state || 'stopped'
const stateLabel = (s) => ({ stopped: 'arrêté', loading: 'chargement…', ready: 'prêt', error: 'erreur' })[s] || s
const modelName = (id) => models.value.find((m) => m.entry.id === id)?.entry.name
const rt = computed(() => status.value.tts_runtime?.state || 'absent')
const rtLabel = computed(() => ({ absent: 'non installé', installing: 'installation…', ready: 'installé', error: 'erreur' })[rt.value] || rt.value)
const installRuntime = () => { logsFor.value = 'tts'; return act(() => axios.post('/api/live/engine/tts-runtime/install', {}, { headers: headers() })) }
const busy = (m) => ['listing', 'downloading', 'verifying'].includes(m.download?.status)
const isLoaded = (m) => status.value[m.entry.kind]?.model === m.entry.id && stateOf(m.entry.kind) !== 'stopped'

async function refresh() {
  try {
    const [s, m] = await Promise.all([axios.get('/api/live/engine/status'), axios.get('/api/live/models')])
    status.value = s.data
    models.value = m.data
    if (logsFor.value) logLines.value = (await axios.get(`/api/live/engine/logs/${logsFor.value}`)).data.lines
  } catch { message.value = 'Moteur live injoignable (live-engine démarré ?)' }
}

async function act(fn) {
  message.value = ''
  try { await fn() } catch (e) { message.value = e.response?.data?.message || e.message }
  await refresh()
}

const download = (m) => act(() => axios.post(`/api/live/models/${m.entry.id}/download`, {}, { headers: headers() }))
const cancel = (m) => act(() => axios.post(`/api/live/models/${m.entry.id}/cancel`, {}, { headers: headers() }))
const load = (m) => act(() => axios.post('/api/live/engine/load', { id: m.entry.id }, { headers: headers() }))
const unload = (kind) => act(() => axios.post('/api/live/engine/unload', { kind }, { headers: headers() }))
const remove = (m) => confirm(`Supprimer « ${m.entry.name} » du disque ?`) &&
  act(() => axios.delete(`/api/live/models/${m.entry.id}`, { headers: headers() }))
const toggleLogs = (k) => { logsFor.value = logsFor.value === k ? '' : k; refresh() }

onMounted(() => { refresh(); timer = setInterval(refresh, 1500) })
onBeforeUnmount(() => clearInterval(timer))
</script>

<style scoped>
.wrap { padding: 1.5rem 2rem; overflow: auto; max-width: 1100px; }
.muted { color: #999; } .small { font-size: .82rem; } .err { color: #ff7b7b; } .warn { color: #e6b450; margin-top: .5rem; }
.gpu { background: #1b1b1b; border: 1px solid #333; border-radius: 10px; padding: .8rem 1rem; margin-bottom: 1rem; }
.bar { height: 8px; background: #2a2a2a; border-radius: 4px; overflow: hidden; margin: .4rem 0; min-width: 160px; }
.bar > div { height: 100%; background: #4a6cf7; transition: width .4s; }
.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1rem; }
.card { background: #1b1b1b; border: 1px solid #333; border-radius: 10px; padding: .8rem 1rem; display: flex; flex-direction: column; gap: .35rem; }
.title { font-weight: 600; display: flex; justify-content: space-between; gap: .5rem; }
.pill { font-size: .75rem; padding: .1rem .55rem; border-radius: 999px; background: #444; font-weight: 400; }
.pill.ready { background: #1f5a34; } .pill.loading { background: #6b5a1f; } .pill.error { background: #5a2a2a; }
.row { display: flex; gap: .5rem; margin-top: .3rem; }
.logs { background: #0d0d0d; border: 1px solid #333; border-radius: 8px; padding: .6rem; max-height: 260px; overflow: auto; font-size: .78rem; white-space: pre-wrap; }
table { width: 100%; border-collapse: collapse; }
.group { margin: 1rem 0 .2rem; color: #9db4ff; font-weight: 600; font-size: .95rem; }
td { padding: .55rem .4rem; border-bottom: 1px solid #2a2a2a; vertical-align: middle; }
.name { width: 55%; } .size { white-space: nowrap; color: #bbb; } .act { display: flex; gap: .5rem; align-items: center; flex-wrap: wrap; }
.msg { color: #ff9b7b; }
button { cursor: pointer; padding: .4rem .8rem; border-radius: 6px; border: 1px solid #555; background: #2a2a2a; color: #eee; }
button:disabled { opacity: .5; cursor: default; }
.primary { background: #4a6cf7; border-color: #4a6cf7; }
code { background: #222; padding: 0 .3rem; border-radius: 4px; }
</style>
