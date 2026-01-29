
<template>
  <div id="app">
    <router-view/>
  </div>
</template>

<script setup>
import { RouterView } from 'vue-router'
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

main {
  flex-grow: 1;
  display: flex;
  flex-direction: column;
  background-color: transparent; /* Remove the inner background */
}
</style>
