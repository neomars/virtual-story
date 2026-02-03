
<template>
  <div class="scene-node">
    <div class="scene-info">
      <span v-if="scene.choice_text" class="choice-text">
        <span class="icon">↳</span> "{{ scene.choice_text }}" &rarr;
      </span>
      <strong class="scene-title">{{ scene.title }}</strong>
      <div class="actions">
        <button @click="deleteScene" class="button-small delete">Supprimer</button>
        <router-link :to="`/admin/scenes/${scene.id}/edit`" class="button-small edit">Éditer</router-link>
        <router-link :to="`/player/${scene.id}`" class="button-small view">Voir</router-link>
      </div>
    </div>
    <div v-if="scene.children && scene.children.length > 0" class="children-container">
      <!-- Recursive call to the component for each child -->
      <SceneNode v-for="child in scene.children" :key="child.id" :scene="child" />
    </div>
  </div>
</template>

<script setup>
import { inject } from 'vue';
import axios from 'axios';

// Define the component name for recursive self-reference
defineOptions({
  name: 'SceneNode'
});

const props = defineProps({
  scene: {
    type: Object,
    required: true
  }
});

const refreshStoryGraph = inject('refreshStoryGraph');

const deleteScene = async () => {
  if (confirm(`Êtes-vous sûr de vouloir supprimer la scène "${props.scene.title}" ?\nCela supprimera également tous les choix qui mènent à cette scène.`)) {
    try {
      await axios.delete(`/api/scenes/${props.scene.id}`);
      if (refreshStoryGraph) {
        refreshStoryGraph();
      }
    } catch (err) {
      console.error('Failed to delete scene:', err);
      alert('Échec de la suppression de la scène.');
    }
  }
};
</script>

<style scoped>
.scene-node {
  margin-left: 30px;
  border-left: 2px solid #444;
  padding-left: 15px;
}
.scene-info {
  display: flex;
  align-items: center;
  padding: 10px;
  background-color: #2a2a2a;
  border-radius: 5px;
  margin-top: 10px;
}
.choice-text {
  color: #aaa;
  margin-right: 10px;
}
.icon {
  font-weight: bold;
  color: #42b983;
}
.scene-title {
  font-weight: bold;
  flex-grow: 1;
}
.actions {
  display: flex;
  gap: 10px;
}
.button-small {
  padding: 5px 10px;
  text-decoration: none;
  border-radius: 5px;
  color: white;
  font-size: 0.9em;
}
.delete {
  background-color: #ef4444;
  border: none;
  cursor: pointer;
}
.delete:hover {
  background-color: #dc2626;
}
.edit {
  background-color: #3a3a3a;
}
.edit:hover {
  background-color: #4a4a4a;
}
.view {
  background-color: #42b983;
}
.view:hover {
  background-color: #52c993;
}
.children-container {
  margin-top: 10px;
}
</style>
