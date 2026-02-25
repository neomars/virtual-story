
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
        <h2>Chapters Management (Parts) <span v-if="reorderStatus" class="badge-video">{{ reorderStatus }}</span></h2>
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
      <ul class="parts-list" @dragover.prevent @dragenter.prevent>
        <li
          v-for="(part, index) in parts"
          :key="part.id"
          draggable="true"
          @dragstart="handleDragStart(index)"
          @dragover.prevent="handleDragOver(index)"
          @drop="handleDrop(index)"
          :class="{ 'is-dragging': draggingIndex === index }"
        >
          <div class="drag-handle" title="Drag to reorder" aria-label="Drag handle to reorder">⠿</div>
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
              <button @click="deletePart(part.id)" class="button-delete" aria-label="Delete chapter">&times;</button>
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
const reorderStatus = ref('');
const newPart = ref({ title: '', first_scene_id: '' });
const partLoopFile = ref(null);
const isSyncing = ref(false);
const draggingIndex = ref(null);

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
    alert(`Update failed: ${err.response?.data?.message || err.message}`);
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
    alert(`Failed to create chapter: ${err.response?.data?.message || err.message}`);
  }
};

const deletePart = async (id) => {
  if (confirm('Delete this chapter?')) {
    try {
      await axios.delete(`/api/parts/${id}`);
      fetchParts();
    } catch (err) {
      console.error(err);
      alert(`Deletion failed: ${err.response?.data?.message || err.message}`);
    }
  }
};

const handleDragStart = (index) => {
  draggingIndex.value = index;
};

const handleDragOver = (index) => {
  if (draggingIndex.value === index) return;
};

const handleDrop = async (index) => {
  if (draggingIndex.value === null || draggingIndex.value === index) {
    draggingIndex.value = null;
    return;
  }

  const movedPart = parts.value.splice(draggingIndex.value, 1)[0];
  parts.value.splice(index, 0, movedPart);
  draggingIndex.value = null;

  // Persist the new order
  try {
    reorderStatus.value = 'Saving...';
    const updatePromises = parts.value.map((part, i) => {
      const formData = new FormData();
      formData.append('title', part.title);
      formData.append('first_scene_id', part.first_scene_id);
      formData.append('order', i);
      return axios.put(`/api/parts/${part.id}`, formData);
    });
    await Promise.all(updatePromises);
    reorderStatus.value = 'Saved!';
    setTimeout(() => { reorderStatus.value = ''; }, 2000);
    console.log('Order updated successfully');
  } catch (err) {
    reorderStatus.value = '';
    console.error('Failed to update order:', err);
    alert('Failed to save the new order.');
    fetchParts(); // Revert on failure
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

<style scoped src="../../assets/styles/SceneList.css"></style>
