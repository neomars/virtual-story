
<template>
  <div>
    <h2>{{ isEditing ? 'Edit Scene' : 'Add New Scene' }}</h2>
    <form @submit.prevent="saveScene">
      <div class="form-group">
        <label for="title">Title</label>
        <input type="text" id="title" v-model="scene.title" required>
      </div>

      <div class="form-group">
        <label for="video">Video File</label>
        <input type="file" id="video" @change="handleFileUpload" :required="!isEditing">
        <div v-if="isEditing && scene.thumbnail_path" class="thumbnail-preview">
          <p>Current Thumbnail:</p>
          <img :src="scene.thumbnail_path" alt="Thumbnail">
        </div>
      </div>

      <button type="submit" class="button">{{ isEditing ? 'Save Changes' : 'Create Scene' }}</button>
      <router-link to="/admin" class="button secondary">Cancel</router-link>
    </form>

    <div v-if="isEditing" class="choices-section">
      <h3>Choices</h3>
      <form @submit.prevent="addChoice" class="add-choice-form">
        <div class="form-group">
          <label for="choice-text">Choice Text</label>
          <input type="text" id="choice-text" v-model="newChoice.choice_text" required>
        </div>
        <div class="form-group">
          <label for="destination">Destination Scene</label>
          <select id="destination" v-model="newChoice.destination_scene_id" required>
            <option disabled value="">Select a scene</option>
            <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
          </select>
        </div>
        <button type="submit" class="button">Add Choice</button>
      </form>
      <ul class="choices-list">
        <li v-for="choice in choices" :key="choice.id">
          <span>{{ choice.choice_text }} &rarr; {{ choice.destination_title }}</span>
          <button @click="deleteChoice(choice.id)" class="button delete">Remove</button>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import axios from 'axios';

const props = defineProps({
  id: String
});

const route = useRoute();
const router = useRouter();
const isEditing = computed(() => !!props.id);

const scene = ref({ title: '' });
const videoFile = ref(null);
const choices = ref([]);
const allScenes = ref([]);
const newChoice = ref({ choice_text: '', destination_scene_id: '' });

const fetchSceneData = async () => {
  if (!isEditing.value) return;

  try {
    // Fetch the scene being edited
    const sceneResponse = await axios.get(`/api/scenes/${props.id}`);
    scene.value = sceneResponse.data;

    // Fetch its choices
    const choicesResponse = await axios.get(`/api/scenes/${props.id}/choices`);
    choices.value = choicesResponse.data;

  } catch (err) {
    console.error('Failed to load scene data:', err);
    alert('Failed to load scene data.');
  }
};

const fetchAllScenes = async () => {
  try {
    const response = await axios.get('/api/scenes');
    // Exclude the current scene from the list of possible destinations
    allScenes.value = response.data.filter(s => s.id !== (isEditing.value ? parseInt(props.id) : -1));
  } catch (err) {
    console.error('Failed to load all scenes:', err);
  }
};

const handleFileUpload = (event) => {
  videoFile.value = event.target.files[0];
};

const saveScene = async () => {
  const formData = new FormData();
  formData.append('title', scene.value.title);
  if (videoFile.value) {
    formData.append('video', videoFile.value);
  }

  try {
    if (isEditing.value) {
      // Note: We're not supporting video replacement in this simple UI for now.
      // We'll just update the title.
      await axios.put(`/api/scenes/${props.id}`, { title: scene.value.title });
      alert('Scene updated!');
    } else {
      await axios.post('/api/scenes', formData, {
        headers: {
          'Content-Type': 'multipart/form-data'
        }
      });
      alert('Scene created!');
    }
    router.push('/admin');
  } catch (err) {
    console.error('Failed to save scene:', err);
    alert('Failed to save scene.');
  }
};

const addChoice = async () => {
  try {
    const response = await axios.post(`/api/scenes/${props.id}/choices`, newChoice.value);
    // Add the new choice to the list for immediate feedback
    const destinationScene = allScenes.value.find(s => s.id === newChoice.value.destination_scene_id);
    choices.value.push({
      id: response.data.id,
      choice_text: newChoice.value.choice_text,
      destination_title: destinationScene ? destinationScene.title : 'Unknown'
    });
    // Reset form
    newChoice.value.choice_text = '';
    newChoice.value.destination_scene_id = '';
  } catch (err) {
    console.error('Failed to add choice:', err);
    alert('Failed to add choice.');
  }
};

const deleteChoice = async (choiceId) => {
  try {
    await axios.delete(`/api/choices/${choiceId}`);
    choices.value = choices.value.filter(c => c.id !== choiceId);
  } catch (err) {
    console.error('Failed to delete choice:', err);
    alert('Failed to delete choice.');
  }
};


onMounted(() => {
  fetchSceneData();
  fetchAllScenes();
});
</script>

<style scoped>
.form-group {
  margin-bottom: 1.5rem;
}

label {
  display: block;
  margin-bottom: 0.5rem;
  font-weight: bold;
}

input[type="text"],
select {
  width: 100%;
  padding: 0.75rem;
  background-color: #2a2a2a;
  border: 1px solid #444;
  border-radius: 5px;
  color: #e0e0e0;
}

.thumbnail-preview img {
  max-width: 200px;
  margin-top: 1rem;
  border-radius: 5px;
}

.choices-section {
  margin-top: 3rem;
  border-top: 1px solid #444;
  padding-top: 2rem;
}

.add-choice-form {
  background-color: #2a2a2a;
  padding: 1.5rem;
  border-radius: 8px;
  margin-bottom: 2rem;
}

.choices-list {
  list-style: none;
  padding: 0;
}

.choices-list li {
  background-color: #2a2a2a;
  padding: 1rem;
  border-radius: 5px;
  margin-bottom: 1rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.button {
  background-color: #42b983;
  color: white;
  border: none;
  padding: 0.75rem 1.5rem;
  border-radius: 5px;
  text-decoration: none;
  cursor: pointer;
  font-size: 1rem;
  margin-right: 0.5rem;
}

.button.secondary {
  background-color: #555;
}

.button.delete {
    background-color: #ef4444;
    padding: 0.5rem 1rem;
    font-size: 0.9rem;
}
</style>
