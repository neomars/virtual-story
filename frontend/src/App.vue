
<template>
  <div id="app">
    <header>
      <h1>Virtual Story</h1>
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
body {
  background-color: #121212;
  color: #e0e0e0;
  font-family: sans-serif;
  margin: 0;
}

#app {
  padding: 1rem;
}

header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 2rem;
  border-bottom: 1px solid #333;
  padding-bottom: 1rem;
}

h1 {
  margin: 0;
}

nav a {
  color: #42b983;
  text-decoration: none;
  margin-left: 1rem;
}

nav a:hover,
nav a:focus-visible {
  text-decoration: underline;
}

nav a.router-link-exact-active {
  font-weight: bold;
}

main {
  background-color: #1e1e1e;
  padding: 1.5rem;
  border-radius: 8px;
}
</style>
