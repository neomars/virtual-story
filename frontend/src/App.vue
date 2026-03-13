
<template>
  <div id="app">
    <a href="#main-content" class="skip-link">Skip to main content</a>
    <header>
      <div class="header-left">
        <router-link to="/" class="header-title">
          <h1>Virtual Story</h1>
        </router-link>
        <div class="parts-nav" v-if="parts.length > 0">
          <router-link
            v-for="part in parts"
            :key="part.id"
            :to="`/player/${part.first_scene_id}`"
            class="part-link"
          >
            {{ part.title }}
          </router-link>
        </div>
      </div>
      <nav>
        <router-link to="/player/1">Player</router-link>
        <router-link to="/admin/scenes" @click="handleAdminClick">Admin</router-link>
        <button v-if="isAuthenticated" @click="handleLogout" class="logout-link">Logout ({{ currentUser?.username }})</button>
      </nav>
    </header>
    <main id="main-content" tabindex="-1">
      <router-view/>
    </main>
    <LoginModal
      :is-open="isLoginModalOpen"
      @close="isLoginModalOpen = false"
      @login-success="onLoginSuccess"
    />
  </div>
</template>

<script setup>
import { ref, onMounted, provide } from 'vue';
import { RouterLink, RouterView, useRouter } from 'vue-router'
import axios from 'axios';
import LoginModal from './components/LoginModal.vue';

const router = useRouter();
const parts = ref([]);
const isAuthenticated = ref(false);
const currentUser = ref(null);
const isLoginModalOpen = ref(false);

// Provide auth state to child components
provide('auth', {
  isAuthenticated,
  currentUser,
  checkAuth
});

async function checkAuth() {
  try {
    const response = await axios.get('/api/auth/me');
    isAuthenticated.value = true;
    currentUser.value = response.data;
    return true;
  } catch (err) {
    isAuthenticated.value = false;
    currentUser.value = null;
    return false;
  }
}

const handleAdminClick = (e) => {
  if (!isAuthenticated.value) {
    e.preventDefault();
    isLoginModalOpen.value = true;
  }
};

const onLoginSuccess = (user) => {
  isAuthenticated.value = true;
  currentUser.value = user;
  isLoginModalOpen.value = false;
  router.push('/admin/scenes');
};

const handleLogout = async () => {
  try {
    await axios.post('/api/auth/logout');
    isAuthenticated.value = false;
    currentUser.value = null;
    if (router.currentRoute.value.path.startsWith('/admin')) {
      router.push('/');
    }
  } catch (err) {
    console.error('Logout failed:', err);
  }
};

const fetchParts = async () => {
  try {
    const response = await axios.get('/api/parts');
    parts.value = response.data;
  } catch (err) {
    console.error('Failed to fetch parts:', err);
  }
};

onMounted(() => {
  checkAuth();
  fetchParts();
});
</script>

<style src="./assets/styles/App.css"></style>
