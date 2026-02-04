
<template>
  <div>
    <h2 class="page-title">{{ isEditing ? 'Éditeur de Scène' : 'Ajouter une Scène' }}</h2>

    <Transition name="fade">
      <div v-if="successMessage" class="success-banner" role="alert">
        {{ successMessage }}
      </div>
    </Transition>

    <!-- Formulaire de création simple -->
    <form @submit.prevent="saveScene" v-if="!isEditing" class="simple-form">
      <div class="form-group">
        <label for="title">Titre</label>
        <input type="text" id="title" v-model="scene.title" required>
      </div>
      <div class="form-group">
        <label for="part">Partie (Optionnel)</label>
        <select id="part" v-model="scene.part_id">
          <option :value="null">Aucune</option>
          <option v-for="p in parts" :key="p.id" :value="p.id">{{ p.title }}</option>
        </select>
      </div>
      <div class="form-group">
        <label for="video">Fichier Vidéo</label>
        <input type="file" id="video" @change="handleFileUpload" required>
      </div>
      <button type="submit" class="button">Créer la Scène</button>
      <router-link to="/admin" class="button secondary">Annuler</router-link>
    </form>

    <!-- Vue d'édition graphique en trois colonnes -->
    <div v-if="isEditing && relations" class="editor-layout">

      <!-- Colonne 1: Scènes Parentes -->
      <div class="side-panel">
        <h3>Accessible Depuis (Parents)</h3>
        <ul class="relation-list">
          <li v-for="parent in relations.parent_scenes" :key="parent.id" class="relation-item">
            <router-link :to="`/admin/scenes/${parent.id}/edit`">
              <strong>{{ parent.title }}</strong><br>
              <small>"{{ parent.choice_text }}"</small>
            </router-link>
            <button @click="removeParentLink(parent.choice_id)" class="button-delete">&times;</button>
          </li>
          <li v-if="relations.parent_scenes.length === 0" class="empty-state">
            Aucune scène ne mène ici.
          </li>
        </ul>
        <div class="add-choice-form">
          <h4>Ajouter un lien d'origine</h4>
          <form @submit.prevent="addParentLink">
            <div class="form-group">
              <label for="source-scene">Scène d'origine</label>
              <select id="source-scene" v-model="newParentLink.source_scene_id" required>
                <option disabled value="">Choisir une scène...</option>
                <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
              </select>
            </div>
            <div class="form-group">
              <label for="parent-choice-text">Texte du choix menant ici</label>
              <input type="text" id="parent-choice-text" v-model="newParentLink.choice_text" required>
            </div>
            <button type="submit" class="button">Lier cette scène</button>
          </form>
        </div>
      </div>

      <!-- Colonne 2: Scène Actuelle -->
      <div class="center-panel">
        <h3>Scène Actuelle</h3>
        <form @submit.prevent="saveScene" class="center-form">
          <div class="form-group">
            <label for="title-edit">Titre</label>
            <input type="text" id="title-edit" v-model="scene.title" required>
          </div>
          <div class="form-group">
            <label for="part-edit">Partie</label>
            <select id="part-edit" v-model="scene.part_id">
              <option :value="null">Aucune</option>
              <option v-for="p in parts" :key="p.id" :value="p.id">{{ p.title }}</option>
            </select>
          </div>
          <div v-if="scene.thumbnail_path" class="thumbnail-preview">
            <img :src="scene.thumbnail_path" alt="Thumbnail">
          </div>
          <button type="submit" class="button">Enregistrer les changements</button>
        </form>
      </div>

      <!-- Colonne 3: Scènes Enfants (Choix) -->
      <div class="side-panel">
        <h3>Mène Vers (Choix)</h3>
        <ul class="relation-list">
          <li v-for="child in relations.child_scenes" :key="child.id">
            <router-link :to="`/admin/scenes/${child.id}/edit`">
              "{{ child.choice_text }}" &rarr; <strong>{{ child.title }}</strong>
            </router-link>
             <!-- Ici, on pourrait ajouter un bouton pour supprimer le choix directement -->
          </li>
        </ul>
        <div class="add-choice-form">
           <h4>Ajouter un nouveau choix</h4>
           <form @submit.prevent="addChoice">
              <div class="form-group">
                <label for="choice-text">Texte du choix</label>
                <input type="text" id="choice-text" v-model="newChoice.choice_text" required>
              </div>
              <div class="form-group">
                <label for="destination">Scène de destination</label>
                <select id="destination" v-model="newChoice.destination_scene_id" required>
                  <option disabled value="">Choisir une scène...</option>
                  <option v-for="s in allScenes" :key="s.id" :value="s.id">{{ s.title }}</option>
                </select>
              </div>
              <button type="submit" class="button">Ajouter le Choix</button>
           </form>
        </div>
      </div>

    </div>
    <router-link to="/admin" class="back-link" v-if="isEditing">&larr; Retour à la liste</router-link>
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

const scene = ref({ title: '', part_id: null });
const videoFile = ref(null);
const allScenes = ref([]);
const newChoice = ref({ choice_text: '', destination_scene_id: '' });
const newParentLink = ref({ source_scene_id: '', choice_text: '' });
const relations = ref(null);
const parts = ref([]);
const successMessage = ref('');

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
  if (scene.value.part_id) formData.append('part_id', scene.value.part_id);
  if (videoFile.value) {
    formData.append('video', videoFile.value);
  }

  try {
    if (isEditing.value) {
      await axios.put(`/api/scenes/${props.id}`, { title: scene.value.title, part_id: scene.value.part_id });
      successMessage.value = 'Scène mise à jour avec succès !';
      setTimeout(() => router.push('/admin'), 1500);
    } else {
      await axios.post('/api/scenes', formData, {
        headers: { 'Content-Type': 'multipart/form-data' }
      });
      successMessage.value = `Scène "${scene.value.title}" créée ! Prêt pour la suivante.`;

      // Reset form for next scene
      scene.value = { title: '', part_id: scene.value.part_id }; // Keep the part_id for convenience
      videoFile.value = null;
      const fileInput = document.getElementById('video');
      if (fileInput) fileInput.value = '';

      // Clear success message after 3 seconds
      setTimeout(() => { successMessage.value = ''; }, 3000);
    }
  } catch (err) {
    console.error('Failed to save scene:', err);
    alert('Échec de l\'enregistrement de la scène.');
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
    alert('Failed to add choice.');
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
    alert('Failed to add parent link.');
  }
};

const removeParentLink = async (choiceId) => {
  if (!confirm('Êtes-vous sûr de vouloir supprimer ce lien d\'origine ?')) {
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
import { watch } from 'vue';
watch(() => props.id, () => {
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
  background-color: #42b983;
  color: white;
  padding: 1rem;
  border-radius: 5px;
  margin-bottom: 2rem;
  text-align: center;
  font-weight: bold;
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
.thumbnail-preview img {
  width: 100%; /* Make it fill the container */
  height: 240px; /* Give it a fixed height */
  object-fit: cover; /* This is the magic property */
  border-radius: 5px;
  margin-top: 1rem;
}
.relation-list {
  list-style: none;
  padding: 0;
  margin: 0;
}
.relation-list li {
  margin-bottom: 1rem;
}
.relation-list a {
  display: block;
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
</style>
