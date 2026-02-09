
<template>
  <div id="app">
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
        <a v-if="isAuthenticated" href="#" @click.prevent="handleLogout" class="logout-link">Déconnexion ({{ currentUser?.username }})</a>
      </nav>
    </header>
    <main>
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

<style>
html, body {
  height: 100%;
  width: 100%;
  margin: 0;
  padding: 0;
  background-color: #121212;
  color: #e0e0e0;
  font-family: sans-serif;
}

#app {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
}

header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 2rem;
  background-color: #1a1a1a;
  border-bottom: 1px solid #333;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 2rem;
}

.header-title {
  text-decoration: none;
  color: inherit;
}

.parts-nav {
  display: flex;
  gap: 1.5rem;
  border-left: 1px solid #444;
  padding-left: 1.5rem;
}

.part-link {
  color: #e0e0e0;
  text-decoration: none;
  font-size: 0.9rem;
  transition: color 0.2s;
}

.part-link:hover {
  color: #42b983;
}

h1 {
  margin: 0;
}

nav a, nav .logout-link {
  color: #42b983;
  text-decoration: none;
  margin-left: 1.5rem;
}

.logout-link {
  cursor: pointer;
  font-size: 0.9rem;
}

nav a.router-link-exact-active {
  font-weight: bold;
}

main {
  flex-grow: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto; /* Allow content to scroll if needed */
}
</style>
