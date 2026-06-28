
<template>
  <div>
    <!-- Section du Graphe de l'Histoire (Now at top and renamed) -->
    <div class="header-container">
      <h2 class="page-title">Story</h2>
      <div class="header-actions">
        <router-link to="/admin/users" class="button secondary-btn">Users & Profile</router-link>
        <router-link to="/admin/scenes/new" class="button">Add Root Scene</router-link>
      </div>
    </div>

    <div v-if="loading">Loading story...</div>
    <div v-else-if="error" class="error-state">
      <p>{{ error }}</p>
      <p>Tip: If this is your first time or after an update, click the <strong>"Sync Database"</strong> button below to prepare the tables.</p>
    </div>
    <div v-else-if="storyGraph.length > 0" class="story-graph-container">
      <div v-for="rootScene in storyGraph" :key="rootScene.id" class="root-scene" :class="{ 'is-collapsed': !expandedChapters[rootScene.id] }">
        <div
          class="chapter-header"
          @click="toggleChapter(rootScene.id)"
          role="button"
          tabindex="0"
          @keydown.enter="toggleChapter(rootScene.id)"
          @keydown.space.prevent="toggleChapter(rootScene.id)"
          :aria-expanded="!!expandedChapters[rootScene.id]"
          :aria-controls="'chapter-content-' + rootScene.id"
        >
          <div class="chapter-info">
            <div v-if="rootScene.part_title" class="part-badge">
              Chapter: {{ rootScene.part_title }}
            </div>
            <span class="root-title">{{ rootScene.title }}</span>
          </div>
          <span class="arrow" :class="{ 'is-rotated': expandedChapters[rootScene.id] }" aria-hidden="true">▼</span>
        </div>
        <div v-if="expandedChapters[rootScene.id]" :id="'chapter-content-' + rootScene.id" class="chapter-content">
          <SceneNode :scene="rootScene" />
        </div>
      </div>
    </div>
    <div v-else class="empty-state">
      <p>No scenes found. Start by creating a root scene!</p>
    </div>

    <hr class="separator" />

    <!-- Section des Paramètres -->
    <div class="settings-section">
      <h2>Player Background</h2>
      <div class="upload-form">
        <label for="background-upload" class="button">Choose Image</label>
        <input id="background-upload" type="file" @change="handleFileChange" accept="image/png, image/jpeg" class="sr-only" />
        <span v-if="selectedFile" class="file-name">{{ selectedFile.name }}</span>
        <button @click="uploadBackground" class="button" :disabled="!selectedFile || isUploadingBackground">
          {{ isUploadingBackground ? 'Uploading...' : 'Upload' }}
        </button>
      </div>
      <p v-if="uploadStatus" :class="{ 'status-success': isSuccess, 'status-error': !isSuccess }" role="status">
        {{ uploadStatus }}
      </p>
    </div>

    <hr class="separator" />

    <!-- Section de Gestion des Parties -->
    <div class="settings-section">
      <div class="section-header">
        <h2>Bulk Import Videos</h2>
        <button @click="fetchUnusedVideos" class="button secondary-btn mini" :disabled="isLoadingUnused">
          {{ isLoadingUnused ? 'Loading...' : 'Refresh available videos' }}
        </button>
      </div>
      <p class="instruction-text">Import multiple videos at once. Titles will be generated from filenames and thumbnails will be created automatically.</p>

      <div v-if="unusedVideos.length > 0" class="bulk-import-container">
        <div class="bulk-controls">
          <label>
            <input type="checkbox" :checked="isAllSelected" @change="toggleSelectAll"> Select All
          </label>
          <div class="import-options">
            <label for="bulk-part">Assign to chapter:</label>
            <select id="bulk-part" v-model="bulkImportData.part_id">
              <option :value="null">None</option>
              <option v-for="p in parts" :key="p.id" :value="p.id">{{ p.title }}</option>
            </select>
          </div>
          <button @click="bulkImport" class="button" :disabled="selectedUnusedVideos.length === 0 || isBulkImporting">
            {{ isBulkImporting ? 'Importing...' : 'Import Selected (' + selectedUnusedVideos.length + ')' }}
          </button>
        </div>

        <div class="unused-videos-grid">
          <div v-for="file in unusedVideos" :key="file.video"
               class="video-selection-card"
               :class="{ selected: selectedUnusedVideos.includes(file.video) }"
               @click="toggleVideoSelection(file.video)">
            <input type="checkbox" :checked="selectedUnusedVideos.includes(file.video)" @click.stop="toggleVideoSelection(file.video)">
            <img :src="file.thumbnail || '/placeholder-thumb.png'" alt="Thumbnail" class="selection-thumb">
            <span class="selection-title" :title="file.video">{{ file.video }}</span>
          </div>
        </div>
      </div>
      <div v-else class="empty-state-compact">
        No new videos found in uploads folder.
      </div>
      <p v-if="bulkImportStatus" class="status-success" role="status">{{ bulkImportStatus }}</p>
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
          <label for="new-part-title" class="sr-only">Chapter title</label>
          <input type="text" id="new-part-title" v-model="newPart.title" placeholder="Chapter title" required />
          <label for="new-part-first-scene" class="sr-only">Starting scene</label>
          <select id="new-part-first-scene" v-model="newPart.first_scene_id" required>
            <option disabled value="">Starting scene</option>
            <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
          </select>
        </div>
        <div class="form-row">
          <label for="part-loop-upload" class="button secondary-btn">Loop Video (Optional or <a href="#" @click.prevent="showExistingParts = !showExistingParts" class="link-alt" @click.stop>Use existing</a>)</label>
          <input v-if="!showExistingParts" id="part-loop-upload" type="file" @change="handlePartFileChange" accept="video/mp4" class="sr-only" />
          <span v-if="partLoopFile && !showExistingParts" class="file-name">{{ partLoopFile.name }}</span>

          <div v-if="showExistingParts" class="existing-videos-grid compact">
            <button type="button" v-for="file in existingPartFiles" :key="file"
                 class="existing-video-card"
                 :class="{ selected: newPart.existing_video_filename === file }"
                 @click="newPart.existing_video_filename = file; partLoopFile = null"
                 :aria-pressed="newPart.existing_video_filename === file"
                 :aria-label="'Select loop video: ' + file">
              <span class="card-title">{{ file }}</span>
            </button>
          </div>

          <button type="submit" class="button" :disabled="isCreatingPart">
            {{ isCreatingPart ? 'Adding...' : 'Add Chapter' }}
          </button>
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
          <div class="drag-handle" title="Drag to reorder" aria-label="Drag handle to reorder">
            <span aria-hidden="true">⠿</span>
          </div>
          <div v-if="editingPartId === part.id" class="edit-part-inline">
            <label :for="'edit-part-title-' + part.id" class="sr-only">Title</label>
            <input :id="'edit-part-title-' + part.id" type="text" v-model="editPartData.title" placeholder="Title" />
            <label :for="'edit-part-scene-' + part.id" class="sr-only">Starting scene</label>
            <select :id="'edit-part-scene-' + part.id" v-model="editPartData.first_scene_id">
              <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
            </select>
            <label :for="'edit-loop-' + part.id" class="button secondary-btn mini">Loop Video (or <a href="#" @click.prevent="showExistingParts = !showExistingParts" class="link-alt" @click.stop>Existing</a>)</label>
            <input v-if="!showExistingParts" :id="'edit-loop-' + part.id" type="file" @change="handleEditFileChange" accept="video/mp4" class="sr-only" />

            <div v-if="showExistingParts" class="existing-videos-grid compact">
              <button type="button" v-for="file in existingPartFiles" :key="file"
                   class="existing-video-card"
                   :class="{ selected: editPartData.existing_video_filename === file }"
                   @click="editPartData.existing_video_filename = file; editPartFile = null"
                   :aria-pressed="editPartData.existing_video_filename === file"
                   :aria-label="'Select loop video: ' + file">
                <span class="card-title">{{ file }}</span>
              </button>
            </div>

            <button @click="updatePart(part.id)" class="button mini" :disabled="isUpdatingPart">
              {{ isUpdatingPart ? 'Saving...' : 'Save' }}
            </button>
            <button @click="cancelEdit" class="button secondary-btn mini" :disabled="isUpdatingPart">Cancel</button>
          </div>
          <div v-else class="part-item-content">
            <span>
              <strong>{{ part.title }}</strong> (Starting ID: {{ part.first_scene_id }})
              <span v-if="part.loop_video_path" class="badge-video" aria-hidden="true">📹 Loop</span>
              <span v-if="part.loop_video_path" class="sr-only">Has ambient loop video</span>
            </span>
            <div class="part-actions">
              <button @click="startEdit(part)" class="button mini" :aria-label="'Edit chapter: ' + part.title">Edit</button>
              <button
                @click="deletePart(part.id)"
                class="button-delete"
                :disabled="deletingPartId === part.id"
                :aria-label="'Delete chapter: ' + part.title"
                :title="'Delete chapter: ' + part.title"
              >
                <span v-if="deletingPartId === part.id" aria-hidden="true">...</span>
                <span v-else aria-hidden="true">&times;</span>
              </button>
            </div>
          </div>
        </li>
      </ul>
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
const newPart = ref({ title: '', first_scene_id: '', existing_video_filename: null });
const partLoopFile = ref(null);
const existingPartFiles = ref([]);
const showExistingParts = ref(false);
const isSyncing = ref(false);
const isUploadingBackground = ref(false);
const unusedVideos = ref([]);
const selectedUnusedVideos = ref([]);
const isLoadingUnused = ref(false);
const isBulkImporting = ref(false);
const bulkImportStatus = ref('');
const bulkImportData = ref({ part_id: null });

const isAllSelected = computed(() => {
  return unusedVideos.value.length > 0 && selectedUnusedVideos.value.length === unusedVideos.value.length;
});

const toggleSelectAll = () => {
  if (isAllSelected.value) {
    selectedUnusedVideos.value = [];
  } else {
    selectedUnusedVideos.value = unusedVideos.value.map(v => v.video);
  }
};

const toggleVideoSelection = (filename) => {
  const index = selectedUnusedVideos.value.indexOf(filename);
  if (index > -1) {
    selectedUnusedVideos.value.splice(index, 1);
  } else {
    selectedUnusedVideos.value.push(filename);
  }
};

const fetchUnusedVideos = async () => {
  isLoadingUnused.value = true;
  try {
    const res = await axios.get('/api/scenes/uploads');
    unusedVideos.value = res.data;
    // Clear selection of videos that are no longer available
    selectedUnusedVideos.value = selectedUnusedVideos.value.filter(v =>
      unusedVideos.value.some(uv => uv.video === v)
    );
  } catch (err) {
    console.error('Failed to fetch unused videos:', err);
  } finally {
    isLoadingUnused.value = false;
  }
};

const bulkImport = async () => {
  if (selectedUnusedVideos.value.length === 0) return;
  isBulkImporting.value = true;
  bulkImportStatus.value = '';
  try {
    const res = await axios.post('/api/scenes/bulk-import', {
      filenames: selectedUnusedVideos.value,
      part_id: bulkImportData.value.part_id
    });
    bulkImportStatus.value = res.data.message;
    selectedUnusedVideos.value = [];
    fetchUnusedVideos();
    fetchStoryGraph();
    fetchAllScenes();
  } catch (err) {
    console.error('Bulk import error:', err);
    alert('Bulk import failed: ' + (err.response?.data?.message || err.message));
  } finally {
    isBulkImporting.value = false;
  }
};
const isCreatingPart = ref(false);
const isUpdatingPart = ref(false);
const deletingPartId = ref(null);
const draggingIndex = ref(null);

// State for editing parts
const editingPartId = ref(null);
const editPartData = ref({ title: '', first_scene_id: '', existing_video_filename: null });
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

const fetchExistingPartFiles = async () => {
  try {
    const res = await axios.get('/api/parts/uploads');
    existingPartFiles.value = res.data;
  } catch (err) {
    console.error('Failed to fetch existing part files:', err);
  }
};

const fetchAllScenes = async () => {
  const res = await axios.get('/api/scenes');
  allScenes.value = res.data.sort((a, b) => a.title.localeCompare(b.title));
};

const handlePartFileChange = (event) => {
  partLoopFile.value = event.target.files[0];
  newPart.value.existing_video_filename = null;
};

const startEdit = (part) => {
  editingPartId.value = part.id;
  editPartData.value = { title: part.title, first_scene_id: part.first_scene_id, existing_video_filename: null };
  editPartFile.value = null;
};

const cancelEdit = () => {
  editingPartId.value = null;
  editPartFile.value = null;
};

const handleEditFileChange = (event) => {
  editPartFile.value = event.target.files[0];
  editPartData.value.existing_video_filename = null;
};

const updatePart = async (id) => {
  isUpdatingPart.value = true;
  try {
    const formData = new FormData();
    formData.append('title', editPartData.value.title);
    formData.append('first_scene_id', editPartData.value.first_scene_id);
    formData.append('order', 0); // Default order for now
    if (editPartFile.value) {
      formData.append('loop_video', editPartFile.value);
    } else if (editPartData.value.existing_video_filename) {
      formData.append('existing_video_filename', editPartData.value.existing_video_filename);
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
  } finally {
    isUpdatingPart.value = false;
  }
};

const createPart = async () => {
  isCreatingPart.value = true;
  try {
    const formData = new FormData();
    formData.append('title', newPart.value.title);
    formData.append('first_scene_id', newPart.value.first_scene_id);
    if (partLoopFile.value) {
      formData.append('loop_video', partLoopFile.value);
    } else if (newPart.value.existing_video_filename) {
      formData.append('existing_video_filename', newPart.value.existing_video_filename);
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
  } finally {
    isCreatingPart.value = false;
  }
};

const deletePart = async (id) => {
  if (confirm('Delete this chapter?')) {
    deletingPartId.value = id;
    try {
      await axios.delete(`/api/parts/${id}`);
      fetchParts();
    } catch (err) {
      console.error(err);
      alert(`Deletion failed: ${err.response?.data?.message || err.message}`);
    } finally {
      deletingPartId.value = null;
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
  isUploadingBackground.value = true;
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
  } finally {
    isUploadingBackground.value = false;
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
  fetchExistingPartFiles();
  fetchUnusedVideos();
});
</script>

<style scoped src="../../assets/styles/SceneList.css"></style>
