
<template>
  <div>
    <!-- Section des Paramètres (conservée) -->
    <div class="settings-section">
      <h2>Player Background</h2>
      <div class="upload-form">
        <label for="background-upload" class="button">Choose Image</label>
        <input id="background-upload" type="file" @change="handleFileChange" accept="image/png, image/jpeg" class="sr-only" />
        <span v-if="selectedFile" class="file-name">{{ selectedFile.name }}</span>
        <button @click="uploadBackground" class="button" :disabled="!selectedFile">Upload</button>
      </div>
      <p v-if="uploadStatus" :class="{ 'status-success': isSuccess, 'status-error': !isSuccess }">
        {{ uploadStatus }}
      </p>
    </div>

    <hr class="separator" />

    <!-- Section de Gestion des Parties -->
    <div class="settings-section">
      <div class="section-header">
        <h2>Chapters Management (Parts)</h2>
        <button @click="syncDatabase" class="button sync-button" :disabled="isSyncing">
          {{ isSyncing ? 'Syncing...' : 'Sync Database' }}
        </button>
      </div>
      <p class="instruction-text">Note: Use the "Sync" button if you encounter loading or creation errors.</p>
      <form @submit.prevent="createPart" class="upload-form multipart-form">
        <div class="form-row">
          <input type="text" v-model="newPart.title" placeholder="Chapter title" required />
          <select v-model="newPart.first_scene_id" required>
            <option disabled value="">Starting scene</option>
            <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
          </select>
        </div>
        <div class="form-row">
          <label for="part-loop-upload" class="button secondary-btn">Loop Video (Optional)</label>
          <input id="part-loop-upload" type="file" @change="handlePartFileChange" accept="video/mp4" class="sr-only" />
          <span v-if="partLoopFile" class="file-name">{{ partLoopFile.name }}</span>
          <button type="submit" class="button">Add Chapter</button>
        </div>
      </form>
      <ul class="parts-list">
        <li v-for="part in parts" :key="part.id">
          <div v-if="editingPartId === part.id" class="edit-part-inline">
            <input type="text" v-model="editPartData.title" placeholder="Title" />
            <select v-model="editPartData.first_scene_id">
              <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
            </select>
            <label :for="'edit-loop-' + part.id" class="button secondary-btn mini">Loop Video</label>
            <input :id="'edit-loop-' + part.id" type="file" @change="handleEditFileChange" accept="video/mp4" class="sr-only" />
            <button @click="updatePart(part.id)" class="button mini">Save</button>
            <button @click="cancelEdit" class="button secondary-btn mini">Cancel</button>
          </div>
          <div v-else class="part-item-content">
            <span>
              <strong>{{ part.title }}</strong> (Starting ID: {{ part.first_scene_id }})
              <span v-if="part.loop_video_path" class="badge-video">📹 Loop</span>
            </span>
            <div class="part-actions">
              <button @click="startEdit(part)" class="button mini">Edit</button>
              <button @click="deletePart(part.id)" class="button-delete">&times;</button>
            </div>
          </div>
        </li>
      </ul>
    </div>

    <hr class="separator" />

    <!-- Nouvelle Section du Graphe de l'Histoire -->
    <div class="header-container">
      <h2 class="page-title">Story Graph</h2>
      <div class="header-actions">
        <router-link to="/admin/users" class="button secondary-btn">Users & Profile</router-link>
        <router-link to="/admin/scenes/new" class="button">Add Root Scene</router-link>
      </div>
    </div>

    <div v-if="loading">Loading graph...</div>
    <div v-else-if="error" class="error-state">
      <p>{{ error }}</p>
      <p>Tip: If this is your first time or after an update, click the <strong>"Sync Database"</strong> button above to prepare the tables.</p>
    </div>
    <div v-else-if="storyGraph.length > 0" class="story-graph-container">
      <div v-for="rootScene in storyGraph" :key="rootScene.id" class="root-scene" :class="{ 'is-collapsed': !expandedChapters[rootScene.id] }">
        <div class="chapter-header" @click="toggleChapter(rootScene.id)" role="button" tabindex="0" @keydown.enter="toggleChapter(rootScene.id)">
          <div class="chapter-info">
            <div v-if="rootScene.part_title" class="part-badge">
              Chapter: {{ rootScene.part_title }}
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
      <p>No scenes found. Start by creating a root scene!</p>
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
    alert(err.response?.data?.message || `Error during synchronization: ${err.message}`);
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
    alert('Chapter updated!');
  } catch (err) {
    console.error(err);
    alert('Update failed.');
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
    alert('Chapter created successfully!');
  } catch (err) {
    console.error(err);
    alert('Failed to create chapter. Check if database is up to date.');
  }
};

const deletePart = async (id) => {
  if (confirm('Delete this chapter?')) {
    try {
      await axios.delete(`/api/parts/${id}`);
      fetchParts();
    } catch (err) {
      console.error(err);
      alert('Deletion failed.');
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
    uploadStatus.value = err.response?.data?.message || 'Upload failed.';
    isSuccess.value = false;
  }
};

const fetchStoryGraph = async () => {
  try {
    const response = await axios.get('/api/admin/story-graph');
    storyGraph.value = response.data;
  } catch (err) {
    error.value = 'Failed to load story graph.';
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
.secondary-btn { background-color: #555 !important; }
.badge-video { background: #42b983; font-size: 0.7rem; padding: 2px 6px; border-radius: 10px; margin-left: 10px; }
</style>
