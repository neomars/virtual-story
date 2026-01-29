
<template>
  <div class="player-container" :style="playerContainerStyle">
    <div v-if="loading">Loading...</div>
    <div v-else-if="error">{{ error }}</div>
    <div v-else class="scene-layout">
      <!-- Parent Scenes -->
      <div class="side-panel left">
        <div class="panel-content">
          <h3>Scènes Précédentes</h3>
          <ul v-if="sceneData.parent_scenes && sceneData.parent_scenes.length > 0">
            <li v-for="parent in sceneData.parent_scenes" :key="parent.id">
              <router-link :to="`/player/${parent.id}`">{{ parent.title }}</router-link>
            </li>
          </ul>
          <p v-else>C'est le début de l'histoire.</p>
        </div>
      </div>

      <!-- Center: Thumbnail/Video -->
      <div class="center-panel">
        <div
          v-if="!isVideoPlaying"
          @click="playVideo"
          @keydown.enter.prevent="playVideo"
          @keydown.space.prevent="playVideo"
          role="button"
          tabindex="0"
          :aria-label="`Play video: ${sceneData.current_scene.title}`"
          class="thumbnail-container"
        >
          <img :src="sceneData.current_scene.thumbnail_path" :alt="`Thumbnail for ${sceneData.current_scene.title}`">
          <div class="play-icon">&#9658;</div>
          <h2>{{ sceneData.current_scene.title }}</h2>
        </div>
        <div v-if="isVideoPlaying" class="video-container">
          <video ref="videoPlayer" :src="sceneData.current_scene.video_path" controls @ended="onVideoEnd"></video>
        </div>
      </div>

      <!-- Next Choices -->
      <div class="side-panel right">
        <div class="panel-content">
          <h3>Next Choices</h3>
          <ul v-if="showChoices">
            <li v-for="choice in sceneData.next_choices" :key="choice.id">
              <router-link :to="`/player/${choice.destination_scene_id}`">{{ choice.choice_text }}</router-link>
            </li>
          </ul>
          <p v-else>Watch the video to see the choices.</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, computed, onMounted } from 'vue';
import { useRoute } from 'vue-router';
import axios from 'axios';

const props = defineProps({
  id: String
});

const route = useRoute();
const sceneData = ref(null);
const loading = ref(true);
const error = ref(null);
const isVideoPlaying = ref(false);
const showChoices = ref(false);
const videoPlayer = ref(null);
const backgroundUrl = ref(null);


// --- Background Image Handling ---
const playerContainerStyle = computed(() => {
  if (backgroundUrl.value) {
    return {
      'background-image': `url(${backgroundUrl.value})`,
      'background-size': 'cover',
      'background-position': 'center',
      'background-attachment': 'fixed',
    };
  }
  return {};
});

const fetchBackground = async () => {
  try {
    const response = await axios.get('/api/settings/background');
    if (response.data.backgroundUrl) {
      backgroundUrl.value = response.data.backgroundUrl;
    }
  } catch (err) {
    console.error('Failed to fetch background:', err);
    // Non-critical, so we don't show an error to the user
  }
};


// --- Scene Data Handling ---
const fetchSceneData = async (sceneId, prevSceneId) => {
  loading.value = true;
  error.value = null;
  isVideoPlaying.value = false;
  showChoices.value = false;

  try {
    let url = `/api/player/scenes/${sceneId}`;
    if (prevSceneId) {
      url += `?previous_scene_id=${prevSceneId}`;
    }
    const response = await axios.get(url);
    sceneData.value = response.data;
  } catch (err) {
    error.value = 'Failed to load scene data.';
    console.error(err);
  } finally {
    loading.value = false;
  }
};

const playVideo = () => {
  isVideoPlaying.value = true;
  // Use nextTick to ensure the video element is in the DOM
  import('vue').then(({ nextTick }) => {
    nextTick(() => {
      if (videoPlayer.value) {
        videoPlayer.value.play();
        videoPlayer.value.requestFullscreen();
      }
    });
  });
};

const onVideoEnd = () => {
  showChoices.value = true;
  if (document.fullscreenElement) {
    document.exitFullscreen();
  }
  // We don't set isVideoPlaying to false, so the (ended) video remains visible.
  // The user can then click a choice to navigate away.
};


// --- Lifecycle Hooks ---

// Watch for route changes to fetch new scene data
watch(() => props.id, (newId, oldId) => {
  fetchSceneData(newId, oldId);
}, { immediate: true });

onMounted(() => {
  fetchBackground();
});
</script>

<style scoped>
.player-container {
  flex-grow: 1;
  width: 100%;
  display: flex;
  padding-top: 2rem;
  padding-bottom: 2rem;
}

.scene-layout {
  display: flex;
  justify-content: space-between;
  gap: 1.5rem;
  width: 100%;
}

.side-panel {
  flex: 1;
  background-color: rgba(30, 30, 30, 0.5); /* 50% transparency */
  padding: 1.5rem;
  border-radius: 8px;
}

.center-panel {
  flex: 2;
  display: flex;
  justify-content: center;
  align-items: center;
  position: relative;
}

.thumbnail-container {
  cursor: pointer;
  position: relative;
  width: 100%;
  height: 360px; /* Set a fixed height */
  max-width: 640px;
  border-radius: 8px;
  overflow: hidden;
  background-color: #000; /* Add a background color */
}

.thumbnail-container img {
  width: 100%;
  height: 100%;
  object-fit: cover; /* This will now work correctly */
  display: block;
}

.thumbnail-container h2 {
    position: absolute;
    bottom: 1rem;
    left: 1rem;
    margin: 0;
    color: white;
    background-color: rgba(0, 0, 0, 0.7);
    padding: 0.5rem 1rem;
    border-radius: 5px;
}

.play-icon {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: 4rem;
  color: white;
  background-color: rgba(0, 0, 0, 0.5);
  border-radius: 50%;
  width: 80px;
  height: 80px;
  display: flex;
  justify-content: center;
  align-items: center;
  transition: background-color 0.2s;
}

.thumbnail-container:hover .play-icon {
  background-color: rgba(66, 185, 131, 0.8);
}

.video-container {
  width: 100%;
}

video {
  width: 100%;
  height: auto;
  display: block;
}

ul {
  list-style: none;
  padding: 0;
}

li {
  margin-bottom: 1rem;
}

li a {
  display: block;
  padding: 1rem;
  background-color: #2a2a2a;
  border-radius: 5px;
  color: #42b983;
  text-decoration: none;
  transition: background-color 0.2s, opacity 0.2s;
  opacity: 0.3; /* Opacity set to 30% */
}

li a:hover {
  background-color: #3a3a3a;
  opacity: 1; /* Full opacity on hover */
}
</style>
