
<template>
  <div class="scene-node">
    <div class="scene-info">
      <span v-if="scene.choice_text" class="choice-text">
        <span class="icon">↳</span> "{{ scene.choice_text }}" &rarr;
      </span>
      <strong class="scene-title">{{ scene.title }}</strong>
      <div class="actions">
        <button
          @click="deleteScene"
          class="button-small delete"
          :disabled="isDeleting"
          :aria-label="'Delete scene: ' + scene.title"
        >
          {{ isDeleting ? 'Deleting...' : 'Delete' }}
        </button>
        <router-link
          :to="`/admin/scenes/${scene.id}/edit`"
          class="button-small edit"
          :aria-label="'Edit scene: ' + scene.title"
        >Edit</router-link>
        <router-link
          :to="`/player/${scene.id}`"
          class="button-small view"
          :aria-label="'View scene in player: ' + scene.title"
        >View</router-link>
      </div>
    </div>
    <div v-if="scene.children && scene.children.length > 0" class="children-container">
      <!-- Recursive call to the component for each child -->
      <SceneNode v-for="child in scene.children" :key="child.id" :scene="child" />
    </div>
  </div>
</template>

<script setup>
import { ref, inject } from 'vue';
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
const isDeleting = ref(false);

const deleteScene = async () => {
  if (confirm(`Are you sure you want to delete the scene "${props.scene.title}"?\nThis will also delete all choices leading to this scene.`)) {
    isDeleting.value = true;
    try {
      await axios.delete(`/api/scenes/${props.scene.id}`);
      if (refreshStoryGraph) {
        refreshStoryGraph();
      }
    } catch (err) {
      console.error('Failed to delete scene:', err);
      alert('Failed to delete scene.');
    } finally {
      isDeleting.value = false;
    }
  }
};
</script>

<style scoped src="../assets/styles/SceneNode.css"></style>
