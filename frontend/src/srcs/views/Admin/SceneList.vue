
<template>
  <div>
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

const scenes = ref([]);
const loading = ref(true);
const error = ref(null);

const fetchScenes = async () => {
  try {
    const response = await axios.get('http://localhost:3000/api/scenes');
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
    await axios.delete(`http://localhost:3000/api/scenes/${id}`);
    scenes.value = scenes.value.filter(scene => scene.id !== id);
  } catch (err) {
    alert('Failed to delete scene.');
    console.error(err);
  }
};

onMounted(fetchScenes);
</script>

<style scoped>
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
