
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
        <router-link to="/admin/scenes">Admin</router-link>
      </nav>
    </header>
    <main>
      <router-view/>
    </main>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue';
import { RouterLink, RouterView } from 'vue-router'
import axios from 'axios';

const parts = ref([]);

const fetchParts = async () => {
  try {
    const response = await axios.get('/api/parts');
    parts.value = response.data;
  } catch (err) {
    console.error('Failed to fetch parts:', err);
  }
};

onMounted(fetchParts);
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

nav a {
  color: #42b983;
  text-decoration: none;
  margin-left: 1.5rem;
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
