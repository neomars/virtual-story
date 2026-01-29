
<template>
  <div>
    <div class="settings-section">
      <h2>Player Background</h2>
      <div class="upload-form">
        <label for="background-upload" class="button">Choose Image</label>
        <input id="background-upload" type="file" @change="handleFileChange" accept="image/png, image/jpeg" class="sr-only" />
        <span v-if="selectedFile" class="file-name">{{ selectedFile.name }}</span>
        <button @click="uploadBackground" class="button" :disabled="!selectedFile">Upload Image</button>
      </div>
       <p v-if="uploadStatus" :class="{ 'status-success': isSuccess, 'status-error': !isSuccess }">
        {{ uploadStatus }}
      </p>
    </div>

    <hr class="separator" />

    <h2>Scenes</h2>
    <router-link to="/admin/scenes/new" class="button">Add New Scene</router-link>
    <div v-if="loading">Loading scenes...</div>
    <div v-else-if="error">{{ error }}</div>
    <ul v-else class="scene-list">
      <li v-for="scene in scenes" :key="scene.id">
        <span>{{ scene.title }}</span>
        <div class="actions">
          <router-link :to="`/admin/scenes/${scene.id}/edit`" class="button edit">Edit</router-link>
          <button @click="deleteScene(scene.id)" class="button delete">Delete</button>
        </div>
      </li>
    </ul>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue';
import axios from 'axios';

// --- State for Scene List ---
const scenes = ref([]);
const loading = ref(true);
const error = ref(null);

// --- State for Background Upload ---
const selectedFile = ref(null);
const uploadStatus = ref('');
const isSuccess = ref(false);


const handleFileChange = (event) => {
  selectedFile.value = event.target.files[0];
  uploadStatus.value = ''; // Reset status on new file selection
};

const uploadBackground = async () => {
  if (!selectedFile.value) {
    return;
  }

  const formData = new FormData();
  formData.append('background', selectedFile.value);

  try {
    const response = await axios.post('/api/admin/background', formData, {
      headers: {
        'Content-Type': 'multipart/form-data',
      },
    });
    uploadStatus.value = response.data.message;
    isSuccess.value = true;
    selectedFile.value = null; // Reset file input
    document.querySelector('input[type="file"]').value = ''; // Clear the file input visually
  } catch (err) {
    uploadStatus.value = err.response?.data?.message || 'Failed to upload background.';
    isSuccess.value = false;
    console.error(err);
  }
};


const fetchScenes = async () => {
  try {
    const response = await axios.get('/api/scenes');
    scenes.value = response.data;
  } catch (err) {
    error.value = 'Failed to load scenes.';
    console.error(err);
  } finally {
    loading.value = false;
  }
};

const deleteScene = async (id) => {
  if (!confirm('Are you sure you want to delete this scene?')) {
    return;
  }
  try {
    await axios.delete(`/api/scenes/${id}`);
    scenes.value = scenes.value.filter(scene => scene.id !== id);
  } catch (err) {
    alert('Failed to delete scene.');
    console.error(err);
  }
};

onMounted(fetchScenes);
</script>

<style scoped>
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

.file-name {
  color: #ccc;
  font-style: italic;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border-width: 0;
}

.status-success {
  color: #42b983;
  margin-top: 1rem;
}

.status-error {
  color: #ef4444;
  margin-top: 1rem;
}

.separator {
  border: none;
  border-top: 1px solid #444;
  margin: 2rem 0;
}


.scene-list {
  list-style: none;
  padding: 0;
  margin-top: 1.5rem;
}

.scene-list li {
  background-color: #2a2a2a;
  padding: 1rem;
  border-radius: 5px;
  margin-bottom: 1rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.actions {
  display: flex;
  gap: 0.5rem;
}

.button {
  background-color: #42b983;
  color: white;
  border: none;
  padding: 0.5rem 1rem;
  border-radius: 5px;
  text-decoration: none;
  cursor: pointer;
  font-size: 0.9rem;
}

.button.edit {
  background-color: #3b82f6;
}

.button.delete {
  background-color: #ef4444;
}
</style>
