
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

    <!-- Simple creation form -->
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
        <label for="video">Video File (Upload or <a href="#" @click.prevent="showExisting = !showExisting">Use existing</a>)</label>
        <input v-if="!showExisting" type="file" id="video" @change="handleFileUpload" :required="!scene.existing_video_filename">
        <div v-else class="existing-videos-grid">
          <button v-for="file in existingFiles" :key="file.video"
               type="button"
               class="existing-video-card"
               :class="{ selected: scene.existing_video_filename === file.video }"
               :aria-pressed="scene.existing_video_filename === file.video"
               :aria-label="'Select video: ' + file.video"
               @click="selectExistingVideo(file.video)">
            <img :src="file.thumbnail || '/placeholder-thumb.png'" alt="Thumbnail" class="card-thumb">
            <span class="card-title">{{ file.video }}</span>
          </button>
          <p v-if="existingFiles.length === 0" class="empty-state">No previously uploaded videos found.</p>
        </div>
      </div>
      <button type="submit" class="button" :disabled="isSaving">
        {{ isSaving ? 'Creating...' : 'Create Scene' }}
      </button>
      <router-link to="/admin" class="button secondary">Cancel</router-link>
    </form>

    <!-- Graphical editing view in three columns -->
    <div v-if="isEditing && relations" class="editor-layout">

      <!-- Column 1: Parent Scenes -->
      <div class="side-panel">
        <h3>Accessible From (Parents)</h3>
        <ul class="relation-list">
          <li v-for="parent in relations.parent_scenes" :key="parent.id" class="relation-item">
            <router-link :to="`/admin/scenes/${parent.id}/edit`">
              <strong>{{ parent.title }}</strong><br>
              <small>"{{ parent.choice_text }}"</small>
            </router-link>
            <button
              @click="removeParentLink(parent.choice_id)"
              class="button-delete"
              :disabled="deletingParentLinkId === parent.choice_id"
              :aria-label="'Remove link from ' + parent.title"
              :title="'Remove link from ' + parent.title"
            >
              <span v-if="deletingParentLinkId === parent.choice_id" aria-hidden="true">...</span>
              <span v-else aria-hidden="true">&times;</span>
            </button>
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
            <button type="submit" class="button mini" :disabled="isAddingParentLink">
              {{ isAddingParentLink ? 'Linking...' : 'Link this scene' }}
            </button>
          </form>
        </div>
      </div>

      <!-- Column 2: Current Scene -->
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
              <label for="video-edit">New Video File (Upload or <a href="#" @click.prevent="showExisting = !showExisting">Use existing</a>)</label>
              <input v-if="!showExisting" type="file" id="video-edit" @change="handleFileUpload">
              <div v-else class="existing-videos-grid">
                <button v-for="file in existingFiles" :key="file.video"
                     type="button"
                     class="existing-video-card"
                     :class="{ selected: scene.existing_video_filename === file.video }"
                     :aria-pressed="scene.existing_video_filename === file.video"
                     :aria-label="'Select video: ' + file.video"
                     @click="selectExistingVideo(file.video)">
                  <img :src="file.thumbnail || '/placeholder-thumb.png'" alt="Thumbnail" class="card-thumb">
                  <span class="card-title">{{ file.video }}</span>
                </button>
                <p v-if="existingFiles.length === 0" class="empty-state">No previously uploaded videos found.</p>
              </div>
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
            <div
              v-else
              @click="isPlayingPreview = true"
              @keydown.enter.prevent="isPlayingPreview = true"
              @keydown.space.prevent="isPlayingPreview = true"
              class="thumbnail-preview"
              title="Click to play preview"
              role="button"
              tabindex="0"
              aria-label="Play video preview"
            >
              <img :src="scene.thumbnail_path" alt="Thumbnail">
              <div class="play-overlay">
                <span class="play-icon" aria-hidden="true">&#9658;</span>
              </div>
            </div>
          </div>
          <button type="submit" class="button" :disabled="isSaving">
            {{ isSaving ? 'Saving...' : 'Save changes' }}
          </button>
        </form>
      </div>

      <!-- Column 3: Child Scenes (Choices) -->
      <div class="side-panel">
        <h3>Leads To (Choices)</h3>
        <ul class="relation-list">
          <li v-for="child in relations.child_scenes" :key="child.id" class="relation-item">
            <router-link :to="`/admin/scenes/${child.id}/edit`">
              "{{ child.choice_text }}" <span aria-hidden="true">&rarr;</span> <strong>{{ child.title }}</strong>
            </router-link>
             <button
               @click="removeChoice(child.choice_id)"
               class="button-delete"
               :disabled="deletingChoiceId === child.choice_id"
               :aria-label="'Delete choice leading to ' + child.title"
               :title="'Delete choice leading to ' + child.title"
             >
               <span v-if="deletingChoiceId === child.choice_id" aria-hidden="true">...</span>
               <span v-else aria-hidden="true">&times;</span>
             </button>
          </li>
          <li v-if="relations.child_scenes.length === 0" class="empty-state">
            No choices lead from this scene.
          </li>
        </ul>
        <div class="add-choice-form">
           <h4>Add new choice (existing scene)</h4>
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
              <button type="submit" class="button mini" :disabled="isAddingChoice">
                {{ isAddingChoice ? 'Adding...' : 'Add Choice' }}
              </button>
           </form>
        </div>

        <div class="add-choice-form">
          <h4>Add new choice (new video)</h4>
          <form @submit.prevent="addNewSceneAndChoice">
            <div class="form-group">
              <label for="new-scene-choice-text">Choice text</label>
              <input type="text" id="new-scene-choice-text" v-model="quickAdd.choice_text" required>
            </div>
            <div class="form-group">
              <label for="new-scene-title">New scene title</label>
              <input type="text" id="new-scene-title" v-model="quickAdd.title" required>
            </div>
            <div class="form-group">
              <label for="new-scene-video" class="button secondary mini">Choose Video File</label>
              <input type="file" id="new-scene-video" @change="handleQuickAddFileUpload" required class="sr-only">
              <span v-if="quickAdd.video" class="file-name">{{ quickAdd.video.name }}</span>
            </div>
            <button type="submit" class="button mini" :disabled="isAddingQuick">
              {{ isAddingQuick ? 'Creating & Linking...' : 'Create & Link' }}
            </button>
          </form>
        </div>
      </div>

    </div>
    <router-link :to="`/player/${props.id}`" class="back-link preview-link" v-if="isEditing" target="_blank">
      <span aria-hidden="true">&nearr;</span> View in player
    </router-link>
    <router-link to="/admin" class="back-link" v-if="isEditing">
      <span aria-hidden="true">&larr;</span> Back to list
    </router-link>
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

const scene = ref({ title: '', part_id: null, prepend_scene_id: null, append_scene_id: null, existing_video_filename: null });
const videoFile = ref(null);
const allScenes = ref([]);
const existingFiles = ref([]);
const showExisting = ref(false);
const newChoice = ref({ choice_text: '', destination_scene_id: '' });
const newParentLink = ref({ source_scene_id: '', choice_text: '' });
const quickAdd = ref({ title: '', choice_text: '', video: null });
const relations = ref(null);
const parts = ref([]);
const successMessage = ref('');
const isSaving = ref(false);
const isAddingChoice = ref(false);
const isAddingParentLink = ref(false);
const isAddingQuick = ref(false);
const deletingChoiceId = ref(null);
const deletingParentLinkId = ref(null);
const isPlayingPreview = ref(false);
const mergeSummary = ref(null);

const fetchParts = async () => {
  const res = await axios.get('/api/parts');
  parts.value = res.data;
};

const fetchExistingFiles = async () => {
  try {
    const res = await axios.get('/api/scenes/uploads');
    existingFiles.value = res.data;
  } catch (err) {
    console.error('Failed to fetch existing files:', err);
  }
};

const fetchSceneData = async () => {
  if (!isEditing.value) return;

  try {
    const response = await axios.get(`/api/admin/scenes/${props.id}/relations`);
    relations.value = response.data;
    // Update current scene with received data
    scene.value = response.data.current_scene;

  } catch (err) {
    console.error('Failed to load scene data:', err);
    alert('Failed to load scene data.');
  }
};

const fetchAllScenes = async () => {
  try {
    const response = await axios.get('/api/scenes');
    // Exclude the current scene from the list of possible destinations and sort by title
    allScenes.value = response.data
      .filter(s => s.id !== (isEditing.value ? parseInt(props.id) : -1))
      .sort((a, b) => a.title.localeCompare(b.title));
  } catch (err) {
    console.error('Failed to load all scenes:', err);
  }
};

const handleFileUpload = (event) => {
  const file = event.target.files[0];
  videoFile.value = file;
  scene.value.existing_video_filename = null;
  if (!isEditing.value && !scene.value.title && file) {
    const filename = file.name;
    const nameWithoutExt = filename.substring(0, filename.lastIndexOf('.')) || filename;
    scene.value.title = nameWithoutExt.replace(/_/g, ' ');
  }
};

const selectExistingVideo = (filename) => {
  scene.value.existing_video_filename = filename;
  videoFile.value = null;
  if (!isEditing.value && !scene.value.title && filename) {
    const nameWithoutExt = filename.substring(0, filename.lastIndexOf('.')) || filename;
    scene.value.title = nameWithoutExt.replace(/_/g, ' ');
  }
};

const handleQuickAddFileUpload = (event) => {
  const file = event.target.files[0];
  quickAdd.value.video = file;
  if (!quickAdd.value.title && file) {
    const filename = file.name;
    const nameWithoutExt = filename.substring(0, filename.lastIndexOf('.')) || filename;
    quickAdd.value.title = nameWithoutExt.replace(/_/g, ' ');
  }
};

const saveScene = async () => {
  const formData = new FormData();
  formData.append('title', scene.value.title);
  formData.append('part_id', scene.value.part_id || '');
  formData.append('prepend_scene_id', scene.value.prepend_scene_id || '');
  formData.append('append_scene_id', scene.value.append_scene_id || '');
  if (videoFile.value) {
    formData.append('video', videoFile.value);
  } else if (scene.value.existing_video_filename) {
    formData.append('existing_video_filename', scene.value.existing_video_filename);
  }

  isSaving.value = true;
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
  } finally {
    isSaving.value = false;
  }
};

const addChoice = async () => {
  isAddingChoice.value = true;
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
  } finally {
    isAddingChoice.value = false;
  }
};

const removeChoice = async (choiceId) => {
  if (!confirm('Are you sure you want to delete this choice?')) {
    return;
  }
  deletingChoiceId.value = choiceId;
  try {
    await axios.delete(`/api/choices/${choiceId}`);
    fetchSceneData();
  } catch (err) {
    console.error('Failed to delete choice:', err);
    alert('Failed to delete choice.');
  } finally {
    deletingChoiceId.value = null;
  }
};

const addNewSceneAndChoice = async () => {
  if (!quickAdd.value.video || !quickAdd.value.title || !quickAdd.value.choice_text) {
    alert('Please fill in all fields for the new scene.');
    return;
  }

  isAddingQuick.value = true;
  try {
    // 1. Create the new scene
    const formData = new FormData();
    formData.append('title', quickAdd.value.title);
    formData.append('part_id', scene.value.part_id || '');
    formData.append('video', quickAdd.value.video);

    const sceneRes = await axios.post('/api/scenes', formData);
    const newSceneId = sceneRes.data.id;

    // 2. Link it to the current scene
    await axios.post(`/api/scenes/${props.id}/choices`, {
      destination_scene_id: newSceneId,
      choice_text: quickAdd.value.choice_text
    });

    // 3. Refresh and Reset
    await fetchSceneData();
    await fetchAllScenes();
    quickAdd.value = { title: '', choice_text: '', video: null };
    const fileInput = document.getElementById('new-scene-video');
    if (fileInput) fileInput.value = '';

    successMessage.value = 'New scene created and linked successfully!';
    setTimeout(() => { successMessage.value = ''; }, 3000);
  } catch (err) {
    console.error('Failed to quick-add scene:', err);
    alert('Failed to create and link new scene.');
  } finally {
    isAddingQuick.value = false;
  }
};

const addParentLink = async () => {
  if (!newParentLink.value.source_scene_id || !newParentLink.value.choice_text) {
    alert('Please select a source scene and enter a choice text.');
    return;
  }

  const choiceData = {
    destination_scene_id: props.id,
    choice_text: newParentLink.value.choice_text,
  };

  isAddingParentLink.value = true;
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
  } finally {
    isAddingParentLink.value = false;
  }
};

const removeParentLink = async (choiceId) => {
  if (!confirm('Are you sure you want to remove this origin link?')) {
    return;
  }
  deletingParentLinkId.value = choiceId;
  try {
    await axios.delete(`/api/choices/${choiceId}`);
    await fetchSceneData(); // Refresh the parent list
  } catch (err) {
    console.error('Failed to remove parent link:', err);
    alert('Failed to remove parent link.');
  } finally {
    deletingParentLinkId.value = null;
  }
};

onMounted(() => {
  fetchSceneData();
  fetchAllScenes();
  fetchParts();
  fetchExistingFiles();
});

// Re-fetch data when the route changes (e.g., navigating from one scene edit to another)
watch(() => props.id, () => {
    isPlayingPreview.value = false;
    fetchSceneData();
    fetchAllScenes();
});

</script>

<style scoped src="../../assets/styles/SceneEdit.css"></style>
