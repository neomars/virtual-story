
<template>
  <div class="player-container" :style="playerContainerStyle">
    <div v-if="loading">Loading...</div>
    <div v-else-if="error">{{ error }}</div>
    <div v-else class="scene-layout">
      <!-- Parent Scenes -->
      <div class="side-panel left">
        <div class="panel-content">
          <h3>Previous Scenes</h3>
          <ul v-if="sceneData.parent_scenes && sceneData.parent_scenes.length > 0">
            <li v-for="parent in sceneData.parent_scenes" :key="parent.id">
              <router-link :to="{ path: `/player/${parent.id}` }">{{ parent.title }}</router-link>
            </li>
          </ul>
          <p v-else>This is the beginning of the story.</p>

          <!-- Chapter Loop Video -->
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
            <router-link :to="{ path: `/player/${sibling.id}`, query: { from: route.query.from } }" class="sibling-link">
              {{ sibling.choice_text || sibling.title }}
            </router-link>
            <span v-if="index < sceneData.sibling_scenes.length - 1" class="separator" aria-hidden="true"> | </span>
          </template>
        </div>

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
          <h3>Next Choices</h3>
          <Transition name="fade" mode="out-in">
            <ul v-if="showChoices" key="choices">
              <li v-for="(choice, index) in sceneData.next_choices" :key="choice.id">
                <router-link :to="{ path: `/player/${choice.destination_scene_id}`, query: { from: props.id } }">
                  <span v-if="index < 9" class="shortcut-hint" aria-hidden="true">[{{ index + 1 }}]</span>
                  {{ choice.choice_text }}
                </router-link>
              </li>
            </ul>
            <p v-else key="waiting">Watch the video to see choices.</p>
          </Transition>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, computed, onMounted, onUnmounted, nextTick } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import axios from 'axios';

const props = defineProps({
  id: String
});

const route = useRoute();
const router = useRouter();
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
const fetchSceneData = async (sceneId, parentId) => {
  loading.value = true;
  error.value = null;
  // On ne remet pas isVideoPlaying à false ici pour éviter le flash de la miniature
  // si on navigue entre deux scènes qui doivent toutes deux être lues.
  showChoices.value = false;

  try {
    let url = `/api/player/scenes/${sceneId}`;
    if (parentId) {
      url += `?previous_scene_id=${parentId}`;
    }
    const response = await axios.get(url);
    sceneData.value = response.data;

    // Update the current loop video based on the scene's part (inherited or direct)
    currentPartLoopVideo.value = sceneData.value.current_scene.part_loop_video_path || null;

    isVideoPlaying.value = true;

    // Si la vidéo était déjà affichée, le watcher sur videoPlayer ne se déclenchera pas.
    // On force donc le redémarrage.
    nextTick(() => {
      if (videoPlayer.value) {
        startPlayback(videoPlayer.value);
      }
    });
  } catch (err) {
    error.value = 'Failed to load scene data.';
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

// --- Keyboard Shortcuts ---
const handleKeydown = (e) => {
  // Don't trigger if user is typing in an input or textarea
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

  if (!videoPlayer.value || !isVideoPlaying.value) return;

  switch (e.key.toLowerCase()) {
    case ' ':
    case 'k':
      e.preventDefault();
      if (videoPlayer.value.paused) {
        videoPlayer.value.play();
      } else {
        videoPlayer.value.pause();
      }
      break;
    case 'f':
      e.preventDefault();
      toggleFullscreen();
      break;
    case 'm':
      e.preventDefault();
      videoPlayer.value.muted = !videoPlayer.value.muted;
      break;
    case 'arrowleft':
    case 'j':
      e.preventDefault();
      videoPlayer.value.currentTime = Math.max(0, videoPlayer.value.currentTime - 10);
      break;
    case 'arrowright':
    case 'l':
      e.preventDefault();
      videoPlayer.value.currentTime = Math.min(videoPlayer.value.duration, videoPlayer.value.currentTime + 10);
      break;
    case '1':
    case '2':
    case '3':
    case '4':
    case '5':
    case '6':
    case '7':
    case '8':
    case '9':
      if (showChoices.value && sceneData.value?.next_choices) {
        const index = parseInt(e.key) - 1;
        if (index < sceneData.value.next_choices.length) {
          e.preventDefault();
          const choice = sceneData.value.next_choices[index];
          router.push({ path: `/player/${choice.destination_scene_id}`, query: { from: props.id } });
        }
      }
      break;
  }
};

const toggleFullscreen = () => {
  if (!videoPlayer.value) return;

  if (!document.fullscreenElement) {
    if (videoPlayer.value.requestFullscreen) {
      videoPlayer.value.requestFullscreen();
    } else if (videoPlayer.value.webkitRequestFullscreen) { /* Safari */
      videoPlayer.value.webkitRequestFullscreen();
    } else if (videoPlayer.value.msRequestFullscreen) { /* IE11 */
      videoPlayer.value.msRequestFullscreen();
    }
  } else {
    if (document.exitFullscreen) {
      document.exitFullscreen();
    }
  }
};

// --- Lifecycle Hooks ---

// GESTION DE L'AUTOPLAY ROBUSTE VIA WATCH
watch(videoPlayer, (el) => {
  if (el) {
    startPlayback(el);
  }
});

// Watch for route changes to fetch new scene data
watch([() => props.id, () => route.query.from], ([newId, newFrom]) => {
  fetchSceneData(newId, newFrom);
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
  window.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  document.body.style.overflow = '';
  window.removeEventListener('keydown', handleKeydown);
});
</script>

<style scoped src="../assets/styles/Player.css"></style>
