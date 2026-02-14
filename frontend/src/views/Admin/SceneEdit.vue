
<template>
  <div>
    <h2 class="page-title">{{ isEditing ? 'Scene Editor' : 'Add Scene' }}</h2>

    <Transition name="fade">
      <div v-if="successMessage" class="success-banner" role="alert">
        {{ successMessage }}
        <div v-if="mergeSummary" class="merge-summary-details">
          <p><strong>Merging Result:</strong> {{ mergeSummary.mergedVideos.join(' + ') }}</p>
          <div v-if="mergeSummary.brokenLinks.length > 0">
            <p>The following links were removed:</p>
            <ul>
              <li v-for="(link, i) in mergeSummary.brokenLinks" :key="i">
                [{{ link.scene }}] {{ link.type === 'incoming' ? 'From' : 'To' }} {{ link.other_scene }} ("{{ link.choice_text }}")
              </li>
            </ul>
          </div>
          <p v-else>No links were broken during this merge.</p>
        </div>
      </div>
    </Transition>

    <!-- Formulaire de création simple -->
    <form @submit.prevent="saveScene" v-if="!isEditing" class="simple-form">
      <div class="form-group">
        <label for="title">Title</label>
        <input type="text" id="title" v-model="scene.title" required>
      </div>
      <div class="form-group">
        <label for="part">Chapter (Optional)</label>
        <select id="part" v-model="scene.part_id">
          <option :value="null">None</option>
          <option v-for="p in parts" :key="p.id" :value="p.id">{{ p.title }}</option>
        </select>
      </div>

      <fieldset class="video-merging-section">
        <legend>Video Merging Options (Optional)</legend>
        <div class="form-group">
          <label for="prepend">Video BEFORE</label>
          <select id="prepend" v-model="scene.prepend_scene_id">
            <option :value="null">None</option>
            <option v-for="s in allScenes" :key="'pre-'+s.id" :value="s.id">{{ s.title }}</option>
          </select>
        </div>
        <div class="form-group">
          <label for="append">Video AFTER</label>
          <select id="append" v-model="scene.append_scene_id">
            <option :value="null">None</option>
            <option v-for="s in allScenes" :key="'app-'+s.id" :value="s.id">{{ s.title }}</option>
          </select>
        </div>
        <p class="help-text">Selected videos will be merged with the new upload automatically.</p>
      </fieldset>

      <div class="form-group">
        <label for="video">Video File</label>
        <input type="file" id="video" @change="handleFileUpload" required>
      </div>
      <button type="submit" class="button">Create Scene</button>
      <router-link to="/admin" class="button secondary">Cancel</router-link>
    </form>

    <!-- Vue d'édition graphique en trois colonnes (Restored as requested) -->
    <div v-if="isEditing && relations" class="editor-layout">

      <!-- Colonne 1: Scènes Parentes -->
      <div class="side-panel">
        <h3>Accessible From (Parents)</h3>
        <ul class="relation-list">
          <li v-for="parent in relations.parent_scenes" :key="parent.id" class="relation-item">
            <router-link :to="`/admin/scenes/${parent.id}/edit`">
              <strong>{{ parent.title }}</strong><br>
              <small>"{{ parent.choice_text }}"</small>
            </router-link>
            <button @click="removeParentLink(parent.choice_id)" class="button-delete" aria-label="Remove link">&times;</button>
          </li>
          <li v-if="relations.parent_scenes.length === 0" class="empty-state">
            No scenes lead here.
          </li>
        </ul>
        <div class="add-choice-form">
          <h4>Add origin link</h4>
          <form @submit.prevent="addParentLink">
            <div class="form-group">
              <label for="source-scene">Source scene</label>
              <select id="source-scene" v-model="newParentLink.source_scene_id" required>
                <option disabled value="">Choose a scene...</option>
                <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
              </select>
            </div>
            <div class="form-group">
              <label for="parent-choice-text">Choice text leading here</label>
              <input type="text" id="parent-choice-text" v-model="newParentLink.choice_text" required>
            </div>
            <button type="submit" class="button">Link this scene</button>
          </form>
        </div>
      </div>

      <!-- Colonne 2: Scène Actuelle -->
      <div class="center-panel">
        <h3>Current Scene</h3>
        <form @submit.prevent="saveScene" class="center-form">
          <div class="form-group">
            <label for="title-edit">Title</label>
            <input type="text" id="title-edit" v-model="scene.title" required>
          </div>
          <div class="form-group">
            <label for="part-edit">Chapter</label>
            <select id="part-edit" v-model="scene.part_id">
              <option :value="null">None</option>
              <option v-for="p in parts" :key="p.id" :value="p.id">{{ p.title }}</option>
            </select>
          </div>

          <fieldset class="video-merging-section">
            <legend>Replace/Merge Video</legend>
            <div class="form-group">
              <label for="video-edit">New Video File</label>
              <input type="file" id="video-edit" @change="handleFileUpload">
            </div>
            <div class="form-group">
              <label for="prepend-edit">Video BEFORE</label>
              <select id="prepend-edit" v-model="scene.prepend_scene_id">
                <option :value="null">None</option>
                <option v-for="s in allScenes" :key="'pre-edit-'+s.id" :value="s.id">{{ s.title }}</option>
              </select>
            </div>
            <div class="form-group">
              <label for="append-edit">Video AFTER</label>
              <select id="append-edit" v-model="scene.append_scene_id">
                <option :value="null">None</option>
                <option v-for="s in allScenes" :key="'app-edit-'+s.id" :value="s.id">{{ s.title }}</option>
              </select>
            </div>
            <p class="help-text">Select BEFORE or AFTER to merge existing scenes with this one. If you don't upload a new file, the current video will be used as the base.</p>
          </fieldset>

          <div v-if="scene.video_path" class="video-preview-container">
            <video v-if="isPlayingPreview" :src="scene.video_path" controls autoplay class="preview-video"></video>
            <div v-else @click="isPlayingPreview = true" class="thumbnail-preview" title="Click to play preview">
              <img :src="scene.thumbnail_path" alt="Thumbnail">
              <div class="play-overlay">
                <span class="play-icon">▶</span>
              </div>
            </div>
          </div>
          <button type="submit" class="button">Save changes</button>
        </form>
      </div>

      <!-- Colonne 3: Scènes Enfants (Choix) -->
      <div class="side-panel">
        <h3>Leads To (Choices)</h3>
        <ul class="relation-list">
          <li v-for="child in relations.child_scenes" :key="child.id">
            <router-link :to="`/admin/scenes/${child.id}/edit`">
              "{{ child.choice_text }}" &rarr; <strong>{{ child.title }}</strong>
            </router-link>
             <button @click="removeChoice(child.choice_id)" class="button-delete" aria-label="Delete choice">&times;</button>
          </li>
        </ul>
        <div class="add-choice-form">
           <h4>Add new choice</h4>
           <form @submit.prevent="addChoice">
              <div class="form-group">
                <label for="choice-text">Choice text</label>
                <input type="text" id="choice-text" v-model="newChoice.choice_text" required>
              </div>
              <div class="form-group">
                <label for="destination">Destination scene</label>
                <select id="destination" v-model="newChoice.destination_scene_id" required>
                  <option disabled value="">Choose a scene...</option>
                  <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
                </select>
              </div>
              <button type="submit" class="button">Add Choice</button>
           </form>
        </div>
      </div>

    </div>
    <router-link to="/admin" class="back-link" v-if="isEditing">&larr; Back to list</router-link>
  </div>
</template>

<script setup>
import { ref, onMounted, computed, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import axios from 'axios';

const props = defineProps({
  id: String
});

const route = useRoute();
const router = useRouter();
const isEditing = computed(() => !!props.id);

const scene = ref({ title: '', part_id: null, prepend_scene_id: null, append_scene_id: null });
const videoFile = ref(null);
const allScenes = ref([]);
const newChoice = ref({ choice_text: '', destination_scene_id: '' });
const newParentLink = ref({ source_scene_id: '', choice_text: '' });
const relations = ref(null);
const parts = ref([]);
const successMessage = ref('');
const isPlayingPreview = ref(false);
const mergeSummary = ref(null);

const fetchParts = async () => {
  const res = await axios.get('/api/parts');
  parts.value = res.data;
};

const fetchSceneData = async () => {
  if (!isEditing.value) return;

  try {
    const response = await axios.get(`/api/admin/scenes/${props.id}/relations`);
    relations.value = response.data;
    // On met à jour la scène actuelle avec les données reçues
    scene.value = response.data.current_scene;

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
  formData.append('part_id', scene.value.part_id || '');
  formData.append('prepend_scene_id', scene.value.prepend_scene_id || '');
  formData.append('append_scene_id', scene.value.append_scene_id || '');
  if (videoFile.value) {
    formData.append('video', videoFile.value);
  }

  try {
    successMessage.value = 'Processing...';
    mergeSummary.value = null;

    if (isEditing.value) {
      const res = await axios.put(`/api/scenes/${props.id}`, formData);
      successMessage.value = 'Scene updated successfully!';
      mergeSummary.value = res.data.concatenationSummary;

      // Reset concat options
      scene.value.prepend_scene_id = null;
      scene.value.append_scene_id = null;
      videoFile.value = null;
      const fileInput = document.getElementById('video-edit');
      if (fileInput) fileInput.value = '';

      await fetchSceneData();
      await fetchAllScenes();

      if (!mergeSummary.value) {
        setTimeout(() => { successMessage.value = ''; }, 2000);
      }
    } else {
      const res = await axios.post('/api/scenes', formData);
      successMessage.value = `Scene "${scene.value.title}" created!`;
      mergeSummary.value = res.data.concatenationSummary;

      // Reset form for next scene
      scene.value = {
        title: '',
        part_id: scene.value.part_id,
        prepend_scene_id: null,
        append_scene_id: null
      };
      videoFile.value = null;
      const fileInput = document.getElementById('video');
      if (fileInput) fileInput.value = '';

      await fetchAllScenes();

      if (!mergeSummary.value) {
        setTimeout(() => { successMessage.value = ''; }, 3000);
      }
    }
  } catch (err) {
    console.error('Failed to save scene:', err);
    let msg = err.message || 'Unknown error';
    if (err.response && err.response.data && err.response.data.message) {
      msg = err.response.data.message;
    }

    let details = '';
    if (err.response) {
      details = ` (Status: ${err.response.status})`;
    } else if (err.request) {
      details = ' (No response received - verify that backend is running on port 3000 and Vite proxy is correctly configured to 127.0.0.1:3000)';
    }

    alert(`Failed to save scene: ${msg}${details}`);
    successMessage.value = '';
  }
};

const addChoice = async () => {
  try {
    await axios.post(`/api/scenes/${props.id}/choices`, newChoice.value);
    // Re-fetch data to show the new choice
    fetchSceneData();
    // Reset form
    newChoice.value.choice_text = '';
    newChoice.value.destination_scene_id = '';
  } catch (err) {
    console.error('Failed to add choice:', err);
    let msg = err.message || 'Unknown error';
    if (err.response && err.response.data && err.response.data.message) {
      msg = err.response.data.message;
    }
    let details = err.response ? ` (Status: ${err.response.status})` : (err.request ? ' (No response from server)' : '');
    alert(`Failed to add choice: ${msg}${details}`);
  }
};

const removeChoice = async (choiceId) => {
  if (!confirm('Are you sure you want to delete this choice?')) {
    return;
  }
  try {
    await axios.delete(`/api/choices/${choiceId}`);
    fetchSceneData();
  } catch (err) {
    console.error('Failed to delete choice:', err);
    alert('Failed to delete choice.');
  }
};

const addParentLink = async () => {
  if (!newParentLink.value.source_scene_id || !newParentLink.value.choice_text) {
    alert('Veuillez sélectionner une scène d\'origine et saisir un texte pour le choix.');
    return;
  }

  const choiceData = {
    destination_scene_id: props.id,
    choice_text: newParentLink.value.choice_text,
  };

  try {
    await axios.post(`/api/scenes/${newParentLink.value.source_scene_id}/choices`, choiceData);
    await fetchSceneData(); // Refresh the parent list
    newParentLink.value = { source_scene_id: '', choice_text: '' }; // Reset form
  } catch (err) {
    console.error('Failed to add parent link:', err);
    let msg = err.message || 'Unknown error';
    if (err.response && err.response.data && err.response.data.message) {
      msg = err.response.data.message;
    }
    let details = err.response ? ` (Status: ${err.response.status})` : (err.request ? ' (No response from server)' : '');
    alert(`Failed to add parent link: ${msg}${details}`);
  }
};

const removeParentLink = async (choiceId) => {
  if (!confirm('Are you sure you want to remove this origin link?')) {
    return;
  }
  try {
    await axios.delete(`/api/choices/${choiceId}`);
    await fetchSceneData(); // Refresh the parent list
  } catch (err) {
    console.error('Failed to remove parent link:', err);
    alert('Failed to remove parent link.');
  }
};

onMounted(() => {
  fetchSceneData();
  fetchAllScenes();
  fetchParts();
});

// Re-fetch data when the route changes (e.g., navigating from one scene edit to another)
watch(() => props.id, () => {
    isPlayingPreview.value = false;
    fetchSceneData();
    fetchAllScenes();
});

</script>

<style scoped>
.page-title {
  border-bottom: 1px solid #444;
  padding-bottom: 1rem;
  margin-bottom: 2rem;
}
.success-banner {
  background-color: #2e7d32;
  color: white;
  padding: 1.5rem;
  border-radius: 8px;
  margin-bottom: 2rem;
  text-align: left;
  border-left: 5px solid #42b983;
}
.merge-summary-details {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid rgba(255,255,255,0.3);
  font-weight: normal;
  font-size: 0.95rem;
  color: #e0e0e0;
}
.merge-summary-details ul {
  margin: 0.5rem 0 0 1.5rem;
  padding: 0;
  list-style: disc;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.5s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.editor-layout {
  display: grid;
  grid-template-columns: 1fr 2fr 1fr;
  gap: 2rem;
}
.side-panel, .center-panel {
  background-color: #2a2a2a;
  padding: 1.5rem;
  border-radius: 8px;
}
.center-form .form-group {
  margin-bottom: 1rem;
}
.video-preview-container {
  margin-top: 1rem;
  border-radius: 8px;
  overflow: hidden;
  background-color: #000;
  position: relative;
  height: 240px;
}
.preview-video {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
.thumbnail-preview {
  position: relative;
  cursor: pointer;
  height: 100%;
}
.thumbnail-preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: opacity 0.3s;
}
.thumbnail-preview:hover img {
  opacity: 0.7;
}
.play-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  background-color: rgba(66, 185, 131, 0.8);
  width: 60px;
  height: 60px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.3s;
}
.thumbnail-preview:hover .play-overlay {
  opacity: 1;
}
.play-icon {
  color: white;
  font-size: 2rem;
  margin-left: 5px;
}
.relation-list {
  list-style: none;
  padding: 0;
  margin: 0;
}
.relation-list li {
  margin-bottom: 1rem;
}
.relation-list li .relation-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}
.relation-list a {
  display: block;
  flex: 1;
  padding: 1rem;
  background-color: #3a3a3a;
  border-radius: 5px;
  color: #e0e0e0;
  text-decoration: none;
  transition: background-color 0.2s;
}
.relation-list a:hover {
  background-color: #4a4a4a;
}
.empty-state {
  font-style: italic;
  color: #888;
}
.add-choice-form {
  margin-top: 2rem;
  border-top: 1px solid #444;
  padding-top: 1.5rem;
}
.form-group {
  margin-bottom: 1rem;
}
label {
  display: block;
  margin-bottom: 0.5rem;
}
input[type="text"], select {
  width: 100%;
  padding: 0.5rem;
  background-color: #1e1e1e;
  border: 1px solid #444;
  border-radius: 5px;
  color: #e0e0e0;
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
  width: 100%;
  margin-top: 1rem;
}
.button.secondary {
  background-color: #555;
}
.back-link {
  display: inline-block;
  margin-top: 2rem;
  color: #42b983;
}
.simple-form {
  max-width: 500px;
}
.video-merging-section {
  border: 1px dashed #42b983;
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1.5rem;
  background-color: rgba(66, 185, 131, 0.05);
}
.video-merging-section legend {
  padding: 0 0.5rem;
  color: #42b983;
  font-weight: bold;
}
.help-text {
  font-size: 0.8rem;
  color: #aaa;
  margin-top: 0.5rem;
}
.button-delete {
  background: #ef4444;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  padding: 0.2rem 0.5rem;
  font-size: 0.8rem;
  margin-left: 0.5rem;
}
.button-delete:hover {
  background-color: #dc2626;
}
</style>
