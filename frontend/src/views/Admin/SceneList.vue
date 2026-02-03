
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
      <h2>Gestion des Parties (Chapitres)</h2>
      <p class="instruction-text">Note : Si la création échoue, assurez-vous d'avoir exécuté <code>node backend/init-db.js</code> sur le serveur.</p>
      <form @submit.prevent="createPart" class="upload-form">
        <input type="text" v-model="newPart.title" placeholder="Titre de la partie" required />
        <select v-model="newPart.first_scene_id" required>
          <option disabled value="">Scène de départ</option>
          <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
        </select>
        <button type="submit" class="button">Ajouter</button>
      </form>
      <ul class="parts-list">
        <li v-for="part in parts" :key="part.id">
          <span>{{ part.title }} (ID Départ: {{ part.first_scene_id }})</span>
          <button @click="deletePart(part.id)" class="button-delete">&times;</button>
        </li>
      </ul>
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
        <div v-if="rootScene.part_title" class="part-badge">
          Chapitre : {{ rootScene.part_title }}
        </div>
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

// --- State for Parts ---
const parts = ref([]);
const allScenes = ref([]);
const newPart = ref({ title: '', first_scene_id: '' });

const fetchParts = async () => {
  const res = await axios.get('/api/parts');
  parts.value = res.data;
};

const fetchAllScenes = async () => {
  const res = await axios.get('/api/scenes');
  allScenes.value = res.data;
};

const createPart = async () => {
  try {
    await axios.post('/api/parts', newPart.value);
    newPart.value = { title: '', first_scene_id: '' };
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
  border-top: 4px solid #42b983;
}
.part-badge {
  display: inline-block;
  background-color: #42b983;
  color: white;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 0.8rem;
  font-weight: bold;
  margin-bottom: 10px;
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
.parts-list li { display: flex; justify-content: space-between; background: #333; padding: 0.5rem; margin-bottom: 0.5rem; border-radius: 4px; }
.button-delete { background: #ef4444; color: white; border: none; border-radius: 4px; cursor: pointer; padding: 0.2rem 0.5rem; }
.instruction-text { font-size: 0.8rem; color: #aaa; margin-bottom: 1rem; }
input[type="text"], select { background: #1e1e1e; color: white; border: 1px solid #444; padding: 0.5rem; border-radius: 4px; }
</style>
