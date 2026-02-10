
<template>
  <div>
    <!-- Section des Paramètres (conservée) -->
    <div class="settings-section">
      <h2>Arrière-plan du Lecteur</h2>
      <div class="upload-form">
        <label for="background-upload" class="button">Choisir une Image</label>
        <input id="background-upload" type="file" @change="handleFileChange" accept="image/png, image/jpeg" class="sr-only" />
        <span v-if="selectedFile" class="file-name">{{ selectedFile.name }}</span>
        <button @click="uploadBackground" class="button" :disabled="!selectedFile">Téléverser</button>
      </div>
      <p v-if="uploadStatus" :class="{ 'status-success': isSuccess, 'status-error': !isSuccess }">
        {{ uploadStatus }}
      </p>
    </div>

    <hr class="separator" />

    <!-- Section de Gestion des Parties -->
    <div class="settings-section">
      <div class="section-header">
        <h2>Gestion des Parties (Chapitres)</h2>
        <button @click="syncDatabase" class="button sync-button" :disabled="isSyncing">
          {{ isSyncing ? 'Synchronisation...' : 'Synchroniser la Base de Données' }}
        </button>
      </div>
      <p class="instruction-text">Note : Utilisez le bouton "Synchroniser" si vous rencontrez des erreurs de chargement ou de création.</p>
      <form @submit.prevent="createPart" class="upload-form multipart-form">
        <div class="form-row">
          <div class="input-group">
            <label for="new-part-title" class="sr-only">Titre de la partie</label>
            <input id="new-part-title" type="text" v-model="newPart.title" placeholder="Titre de la partie" required />
          </div>
          <div class="input-group">
            <label for="new-part-scene" class="sr-only">Scène de départ</label>
            <select id="new-part-scene" v-model="newPart.first_scene_id" required>
              <option disabled value="">Scène de départ</option>
              <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
            </select>
          </div>
        </div>
        <div class="form-row">
          <label for="part-loop-upload" class="button secondary-btn">Vidéo Boucle (Optionnel)</label>
          <input id="part-loop-upload" type="file" @change="handlePartFileChange" accept="video/mp4" class="sr-only" />
          <span v-if="partLoopFile" class="file-name">{{ partLoopFile.name }}</span>
          <button type="submit" class="button">Ajouter la Partie</button>
        </div>
      </form>
      <ul class="parts-list">
        <li v-for="part in parts" :key="part.id">
          <div v-if="editingPartId === part.id" class="edit-part-inline">
            <label :for="'edit-title-' + part.id" class="sr-only">Titre</label>
            <input :id="'edit-title-' + part.id" type="text" v-model="editPartData.title" placeholder="Titre" />

            <label :for="'edit-scene-' + part.id" class="sr-only">Scène de départ</label>
            <select :id="'edit-scene-' + part.id" v-model="editPartData.first_scene_id">
              <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
            </select>

            <label :for="'edit-loop-' + part.id" class="button secondary-btn mini">Vidéo Loop</label>
            <input :id="'edit-loop-' + part.id" type="file" @change="handleEditFileChange" accept="video/mp4" class="sr-only" />
            <button @click="updatePart(part.id)" class="button mini">Enregistrer</button>
            <button @click="cancelEdit" class="button secondary-btn mini">Annuler</button>
          </div>
          <div v-else class="part-item-content">
            <span>
              <strong>{{ part.title }}</strong> (Départ ID: {{ part.first_scene_id }})
              <span v-if="part.loop_video_path" class="badge-video">📹 Loop</span>
            </span>
            <div class="part-actions">
              <button @click="startEdit(part)" class="button mini">Éditer</button>
              <button @click="deletePart(part.id)" class="button-delete" aria-label="Supprimer la partie">&times;</button>
            </div>
          </div>
        </li>
      </ul>
    </div>

    <hr class="separator" />

    <!-- Nouvelle Section du Graphe de l'Histoire -->
    <div class="header-container">
      <h2 class="page-title">Graphe de l'Histoire</h2>
      <div class="header-actions">
        <router-link to="/admin/users" class="button secondary-btn">Utilisateurs & Profil</router-link>
        <router-link to="/admin/scenes/new" class="button">Ajouter une Scène Racine</router-link>
      </div>
    </div>

    <div v-if="loading">Chargement du graphe...</div>
    <div v-else-if="error" class="error-state">
      <p>{{ error }}</p>
      <p>Astuce : Si c'est votre première utilisation ou après une mise à jour, cliquez sur le bouton <strong>"Synchroniser la Base de Données"</strong> ci-dessus pour préparer les tables.</p>
    </div>
    <div v-else-if="storyGraph.length > 0" class="story-graph-container">
      <div v-for="rootScene in storyGraph" :key="rootScene.id" class="root-scene" :class="{ 'is-collapsed': !expandedChapters[rootScene.id] }">
        <div class="chapter-header" @click="toggleChapter(rootScene.id)" role="button" tabindex="0" @keydown.enter="toggleChapter(rootScene.id)">
          <div class="chapter-info">
            <div v-if="rootScene.part_title" class="part-badge">
              Chapitre : {{ rootScene.part_title }}
            </div>
            <span class="root-title">{{ rootScene.title }}</span>
          </div>
          <span class="arrow" :class="{ 'is-rotated': expandedChapters[rootScene.id] }">▼</span>
        </div>
        <div v-if="expandedChapters[rootScene.id]" class="chapter-content">
          <SceneNode :scene="rootScene" />
        </div>
      </div>
    </div>
    <div v-else class="empty-state">
      <p>Aucune scène trouvée. Commencez par créer une scène racine !</p>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, provide } from 'vue';
import axios from 'axios';
import SceneNode from '../../components/SceneNode.vue';

// --- State for Story Graph ---
const storyGraph = ref([]);
const loading = ref(true);
const error = ref(null);
const expandedChapters = ref({});

const toggleChapter = (id) => {
  expandedChapters.value[id] = !expandedChapters.value[id];
};

// --- State for Background Upload ---
const selectedFile = ref(null);
const uploadStatus = ref('');
const isSuccess = ref(false);

// --- State for Parts ---
const parts = ref([]);
const allScenes = ref([]);
const newPart = ref({ title: '', first_scene_id: '' });
const partLoopFile = ref(null);
const isSyncing = ref(false);

// State for editing parts
const editingPartId = ref(null);
const editPartData = ref({ title: '', first_scene_id: '' });
const editPartFile = ref(null);

const syncDatabase = async () => {
  isSyncing.value = true;
  try {
    const res = await axios.post('/api/admin/db-sync');
    alert(res.data.message);
    fetchParts();
    fetchStoryGraph();
  } catch (err) {
    console.error('Sync error:', err);
    alert(err.response?.data?.message || `Erreur lors de la synchronisation : ${err.message}`);
  } finally {
    isSyncing.value = false;
  }
};

const fetchParts = async () => {
  const res = await axios.get('/api/parts');
  parts.value = res.data;
};

const fetchAllScenes = async () => {
  const res = await axios.get('/api/scenes');
  allScenes.value = res.data;
};

const handlePartFileChange = (event) => {
  partLoopFile.value = event.target.files[0];
};

const startEdit = (part) => {
  editingPartId.value = part.id;
  editPartData.value = { title: part.title, first_scene_id: part.first_scene_id };
  editPartFile.value = null;
};

const cancelEdit = () => {
  editingPartId.value = null;
  editPartFile.value = null;
};

const handleEditFileChange = (event) => {
  editPartFile.value = event.target.files[0];
};

const updatePart = async (id) => {
  try {
    const formData = new FormData();
    formData.append('title', editPartData.value.title);
    formData.append('first_scene_id', editPartData.value.first_scene_id);
    formData.append('order', 0); // Default order for now
    if (editPartFile.value) {
      formData.append('loop_video', editPartFile.value);
    }

    await axios.put(`/api/parts/${id}`, formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    });

    editingPartId.value = null;
    editPartFile.value = null;
    fetchParts();
    alert('Partie mise à jour !');
  } catch (err) {
    console.error(err);
    alert('Échec de la mise à jour.');
  }
};

const createPart = async () => {
  try {
    const formData = new FormData();
    formData.append('title', newPart.value.title);
    formData.append('first_scene_id', newPart.value.first_scene_id);
    if (partLoopFile.value) {
      formData.append('loop_video', partLoopFile.value);
    }

    await axios.post('/api/parts', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    });

    newPart.value = { title: '', first_scene_id: '' };
    partLoopFile.value = null;
    const fileInput = document.querySelector('#part-loop-upload');
    if (fileInput) fileInput.value = '';

    fetchParts();
    alert('Partie créée avec succès !');
  } catch (err) {
    console.error(err);
    alert('Échec de la création de la partie. Vérifiez que la base de données est à jour.');
  }
};

const deletePart = async (id) => {
  if (confirm('Supprimer cette partie ?')) {
    try {
      await axios.delete(`/api/parts/${id}`);
      fetchParts();
    } catch (err) {
      console.error(err);
      alert('Échec de la suppression.');
    }
  }
};

const handleFileChange = (event) => {
  selectedFile.value = event.target.files[0];
  uploadStatus.value = '';
};

const uploadBackground = async () => {
  if (!selectedFile.value) return;
  const formData = new FormData();
  formData.append('background', selectedFile.value);
  try {
    const response = await axios.post('/api/admin/background', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    });
    uploadStatus.value = response.data.message;
    isSuccess.value = true;
    selectedFile.value = null;
    document.querySelector('#background-upload').value = '';
  } catch (err) {
    uploadStatus.value = err.response?.data?.message || 'Échec du téléversement.';
    isSuccess.value = false;
  }
};

const fetchStoryGraph = async () => {
  try {
    const response = await axios.get('/api/admin/story-graph');
    storyGraph.value = response.data;
  } catch (err) {
    error.value = 'Échec du chargement du graphe de l\'histoire.';
    console.error(err);
  } finally {
    loading.value = false;
  }
};

provide('refreshStoryGraph', fetchStoryGraph);

onMounted(() => {
  fetchStoryGraph();
  fetchParts();
  fetchAllScenes();
});
</script>

<style scoped>
/* Styles existants pour la section des paramètres */
.settings-section {
  background-color: #2a2a2a;
  padding: 1.5rem;
  border-radius: 5px;
  margin-bottom: 2rem;
}
.settings-section h2 {
  margin-top: 0;
  border-bottom: 1px solid #444;
  padding-bottom: 0.5rem;
  margin-bottom: 1rem;
}
.upload-form {
  display: flex;
  align-items: center;
  gap: 1rem;
}
.file-name { color: #ccc; font-style: italic; }
.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border-width: 0; }
.status-success { color: #42b983; margin-top: 1rem; }
.status-error { color: #ef4444; margin-top: 1rem; }
.separator { border: none; border-top: 1px solid #444; margin: 2rem 0; }

/* Nouveaux styles pour la vue en graphe */
.header-container {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
}
.header-actions {
  display: flex;
  gap: 1rem;
}
.page-title {
  margin: 0;
}
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 1rem;
}
.sync-button {
  background-color: #3a3a3a;
  font-size: 0.8rem;
}
.button {
  background-color: #42b983;
  color: white;
  border: none;
  padding: 0.6rem 1rem;
  border-radius: 5px;
  text-decoration: none;
  cursor: pointer;
  font-size: 0.9rem;
}
.story-graph-container {
  background-color: #1e1e1e;
  padding: 20px;
  border-radius: 8px;
}
.root-scene {
  background-color: #2a2a2a;
  border-radius: 5px;
  margin-bottom: 20px;
  border-top: 4px solid #42b983;
  overflow: hidden;
}
.chapter-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px;
  cursor: pointer;
  transition: background-color 0.2s;
}
.chapter-header:hover {
  background-color: #333;
}
.chapter-info {
  display: flex;
  align-items: center;
  gap: 15px;
}
.root-title {
  font-weight: bold;
  color: #eee;
}
.arrow {
  font-size: 0.8rem;
  color: #42b983;
  transition: transform 0.3s ease;
}
.arrow.is-rotated {
  transform: rotate(180deg);
}
.chapter-content {
  padding: 0 15px 15px 15px;
}
.part-badge {
  display: inline-block;
  background-color: #42b983;
  color: white;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 0.8rem;
  font-weight: bold;
}
.root-scene > :deep(.scene-node) {
  margin-left: 0;
  border-left: none;
  padding-left: 0;
}
.empty-state, .error-state {
  text-align: center;
  font-style: italic;
  color: #888;
  margin-top: 3rem;
}
.parts-list { list-style: none; padding: 0; margin-top: 1rem; }
.parts-list li { background: #333; padding: 0.5rem; margin-bottom: 0.5rem; border-radius: 4px; }
.part-item-content { display: flex; justify-content: space-between; align-items: center; }
.part-actions { display: flex; gap: 0.5rem; }
.edit-part-inline { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
.edit-part-inline input, .edit-part-inline select { flex: 1; min-width: 120px; }
.button.mini { padding: 0.3rem 0.6rem; font-size: 0.8rem; margin: 0; }
.button-delete { background: #ef4444; color: white; border: none; border-radius: 4px; cursor: pointer; padding: 0.2rem 0.5rem; font-size: 0.8rem; }
.instruction-text { font-size: 0.8rem; color: #aaa; margin-bottom: 1rem; }
input[type="text"], select { background: #1e1e1e; color: white; border: 1px solid #444; padding: 0.5rem; border-radius: 4px; flex: 1; }

.multipart-form { flex-direction: column; align-items: stretch; }
.form-row { display: flex; gap: 1rem; align-items: center; margin-bottom: 1rem; }
.input-group { flex: 1; display: flex; flex-direction: column; }
.secondary-btn { background-color: #555 !important; }
.badge-video { background: #42b983; font-size: 0.7rem; padding: 2px 6px; border-radius: 10px; margin-left: 10px; }
</style>
