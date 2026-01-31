
<template>
  <div id="app">
    <header>
      <router-link to="/" class="header-title">
        <h1>Virtual Story</h1>
      </router-link>
      <nav>
        <router-link to="/player/1">Player</router-link>
        <router-link to="/admin">Admin</router-link>
      </nav>
    </header>
    <main>
      <router-view/>
    </main>
  </div>
</template>

<script setup>
import { RouterLink, RouterView } from 'vue-router'
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

.header-title {
  text-decoration: none;
  color: inherit;
}

h1 {
  margin: 0;
}

nav a {
  color: #42b983;
  text-decoration: none;
  margin-left: 1.5rem;
}

nav a:hover,
nav a:focus-visible {
  text-decoration: underline;
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
