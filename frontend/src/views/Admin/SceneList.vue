
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

    <!-- Nouvelle Section du Graphe de l'Histoire -->
    <div class="header-container">
      <h2 class="page-title">Graphe de l'Histoire</h2>
      <router-link to="/admin/scenes/new" class="button">Ajouter une Scène Racine</router-link>
    </div>

    <div v-if="loading">Chargement du graphe...</div>
    <div v-else-if="error">{{ error }}</div>
    <div v-else-if="storyGraph.length > 0" class="story-graph-container">
      <div v-for="rootScene in storyGraph" :key="rootScene.id" class="root-scene">
        <SceneNode :scene="rootScene" />
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

// --- State for Background Upload ---
const selectedFile = ref(null);
const uploadStatus = ref('');
const isSuccess = ref(false);

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

onMounted(fetchStoryGraph);
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
.page-title {
  margin: 0;
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
  padding: 15px;
  border-radius: 5px;
  margin-bottom: 20px;
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
</style>
