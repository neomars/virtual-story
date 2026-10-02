<template>
  <div class="home-container">
    <h2>Welcome to Virtual Story</h2>
    <p>This is the main landing page for the interactive story experience.</p>
    <p>
      You can start the story by navigating to the
      <router-link to="/player/1">first scene</router-link>.
    </p>
    <p>
      Ou discutez en direct avec une IA : <router-link to="/live">mode Live</router-link>
      (<router-link to="/admin/live/models">modèles</router-link>,
      <router-link to="/admin/live/personas">personnages</router-link>,
      <router-link to="/admin/live/media">médiathèque</router-link>).
    </p>
    <p>
      Or, you can manage the story content in the
      <router-link to="/admin/scenes">admin panel</router-link>.
    </p>
  </div>
</template>

<script setup>
import { RouterLink } from 'vue-router'
</script>

<style scoped src="../assets/styles/Home.css"></style>
