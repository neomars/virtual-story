
<template>
  <div>
    <h2 class="page-title">{{ isEditing ? 'Éditeur de Scène' : 'Ajouter une Scène' }}</h2>

    <!-- Formulaire de création simple -->
    <form @submit.prevent="saveScene" v-if="!isEditing" class="simple-form">
      <div class="form-group">
        <label for="title">Titre</label>
        <input type="text" id="title" v-model="scene.title" required>
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
        <h3>Accessible Depuis</h3>
        <ul class="relation-list">
          <li v-for="parent in relations.parent_scenes" :key="parent.id">
            <router-link :to="`/admin/scenes/${parent.id}/edit`">{{ parent.title }}</router-link>
          </li>
           <li v-if="relations.parent_scenes.length === 0" class="empty-state">
            Aucune scène ne mène ici. (Début d'une branche)
          </li>
        </ul>
      </div>

      <!-- Colonne 2: Scène Actuelle -->
      <div class="center-panel">
        <h3>Scène Actuelle</h3>
        <form @submit.prevent="saveScene" class="center-form">
          <div class="form-group">
            <label for="title-edit">Titre</label>
            <input type="text" id="title-edit" v-model="scene.title" required>
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

const scene = ref({ title: '' });
const videoFile = ref(null);
const allScenes = ref([]);
const newChoice = ref({ choice_text: '', destination_scene_id: '' });
const relations = ref(null); // Pour stocker les données de la nouvelle API

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

// Note: deleteChoice is not implemented in this UI, but the API endpoint exists.
// A button could be added next to each choice in the list to call this.

onMounted(() => {
  fetchSceneData();
  fetchAllScenes();
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
