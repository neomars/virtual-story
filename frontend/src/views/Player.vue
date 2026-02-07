
<template>
  <div class="player-container" :style="playerContainerStyle">
    <div v-if="loading">Chargement...</div>
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

          <!-- Vidéo de Chapitre en Boucle -->
          <div v-if="currentPartLoopVideo" class="part-loop-container">
            <video :src="currentPartLoopVideo" autoplay loop muted playsinline class="loop-video" aria-hidden="true"></video>
          </div>
        </div>
      </div>

      <!-- Center: Thumbnail/Video -->
      <div class="center-panel">
        <!-- Sibling Navigation -->
        <div v-if="sceneData.sibling_scenes && sceneData.sibling_scenes.length > 0" class="siblings-nav">
          <template v-for="(sibling, index) in sceneData.sibling_scenes" :key="sibling.id">
            <router-link :to="`/player/${sibling.id}`" class="sibling-link">{{ sibling.title }}</router-link>
            <span v-if="index < sceneData.sibling_scenes.length - 1" class="separator"> | </span>
          </template>
        </div>

        <div
          v-if="!isVideoPlaying"
          @click="playVideo"
          @keydown.enter.prevent="playVideo"
          @keydown.space.prevent="playVideo"
          role="button"
          tabindex="0"
          :aria-label="`Jouer la vidéo : ${sceneData.current_scene.title}`"
          class="thumbnail-container"
        >
          <img :src="sceneData.current_scene.thumbnail_path" :alt="`Miniature pour ${sceneData.current_scene.title}`">
          <div class="play-icon" aria-hidden="true">&#9658;</div>
          <h2>{{ sceneData.current_scene.title }}</h2>
        </div>
        <div v-if="isVideoPlaying" class="video-container" :class="{ 'full-page': !showChoices }">
          <video ref="videoPlayer" :src="sceneData.current_scene.video_path" controls autoplay playsinline @ended="onVideoEnd"></video>
        </div>
      </div>

      <!-- Next Choices -->
      <div class="side-panel right">
        <div class="panel-content">
          <h3>Choix suivants</h3>
          <Transition name="fade" mode="out-in">
            <ul v-if="showChoices" key="choices">
              <li v-for="choice in sceneData.next_choices" :key="choice.id">
                <router-link :to="`/player/${choice.destination_scene_id}`">{{ choice.choice_text }}</router-link>
              </li>
            </ul>
            <p v-else key="waiting">Regardez la vidéo pour voir les choix.</p>
          </Transition>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, computed, onMounted, onUnmounted, nextTick } from 'vue';
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
const currentPartLoopVideo = ref(null);


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
  // On ne remet pas isVideoPlaying à false ici pour éviter le flash de la miniature
  // si on navigue entre deux scènes qui doivent toutes deux être lues.
  showChoices.value = false;

  try {
    let url = `/api/player/scenes/${sceneId}`;
    if (prevSceneId) {
      url += `?previous_scene_id=${prevSceneId}`;
    }
    const response = await axios.get(url);
    sceneData.value = response.data;

    // Logic for inheriting part loop video
    // If the scene belongs to a part (part_id is not null), we update the current loop video.
    // If it doesn't have a part_id, we keep the previous one (inheritance).
    if (sceneData.value.current_scene.part_id !== null && sceneData.value.current_scene.part_id !== undefined) {
      currentPartLoopVideo.value = sceneData.value.current_scene.part_loop_video_path;
    }

    isVideoPlaying.value = true;

    // Si la vidéo était déjà affichée, le watcher sur videoPlayer ne se déclenchera pas.
    // On force donc le redémarrage.
    nextTick(() => {
      if (videoPlayer.value) {
        startPlayback(videoPlayer.value);
      }
    });
  } catch (err) {
    error.value = 'Échec du chargement des données de la scène.';
    console.error(err);
  } finally {
    loading.value = false;
  }
};

const playVideo = (withFullscreen = false) => {
  isVideoPlaying.value = true;
  nextTick(() => {
    if (videoPlayer.value) {
      startPlayback(videoPlayer.value, withFullscreen);
    }
  });
};

const startPlayback = async (el, withFullscreen = false) => {
  if (!el) return;
  // Tente la lecture avec le son
  el.muted = false;
  try {
    await el.play();
  } catch (err) {
    console.warn("Lecture avec son bloquée, tentative en muet...", err);
    el.muted = true;
    await el.play().catch(() => {});
  }

  // Tente le plein écran natif
  if ((withFullscreen || !showChoices.value) && el.requestFullscreen) {
    el.requestFullscreen().catch(() => {});
  }
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

// GESTION DE L'AUTOPLAY ROBUSTE VIA WATCH
watch(videoPlayer, (el) => {
  if (el) {
    startPlayback(el);
  }
});

// Watch for route changes to fetch new scene data
watch(() => props.id, (newId, oldId) => {
  fetchSceneData(newId, oldId);
}, { immediate: true });

// Masquer les barres de défilement en mode plein écran
watch([isVideoPlaying, showChoices], ([playing, showingChoices]) => {
  if (playing && !showingChoices) {
    document.body.style.overflow = 'hidden';
  } else {
    document.body.style.overflow = '';
  }
}, { immediate: true });

onMounted(() => {
  fetchBackground();
});

onUnmounted(() => {
  document.body.style.overflow = '';
});
</script>

<style scoped>
.player-container {
  width: 100%;
  height: 100%;
  display: flex;
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
  flex-direction: column;
  justify-content: center;
  align-items: center;
  position: relative;
}

.siblings-nav {
  width: 100%;
  margin-bottom: 1rem;
  text-align: center;
  font-size: 0.85rem;
  display: flex;
  justify-content: center;
  gap: 0.5rem;
}

.sibling-link {
  color: #42b983;
  text-decoration: none;
}

.sibling-link:hover {
  text-decoration: underline;
}

.separator {
  margin: 0 0.3rem;
  color: #42b983;
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
  transition: box-shadow 0.2s;
}

.thumbnail-container:focus-visible {
  outline: 2px solid transparent;
  box-shadow: 0 0 0 4px #42b983;
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

.video-container { width: 100%; }
.video-container.full-page {
  position: fixed; top: 0; left: 0;
  width: 100vw; height: 100vh;
  z-index: 1000; background: #000;
}
.full-page video { width: 100%; height: 100%; object-fit: contain; }
video { width: 100%; height: auto; display: block; }

/* Transition de fondu (fade) */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
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
  transition: background-color 0.2s, box-shadow 0.2s;
}

li a:hover,
li a:focus-visible {
  background-color: #3a3a3a;
  outline: 2px solid transparent;
  box-shadow: 0 0 0 2px #42b983;
}

.part-loop-container {
  margin-top: 2rem;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 4px 15px rgba(0,0,0,0.5);
}

.loop-video {
  width: 100%;
  display: block;
}
</style>
