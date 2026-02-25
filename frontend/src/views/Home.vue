<template>
  <div class="home-container">
    <h2>Welcome to Virtual Story</h2>
    <p>This is the main landing page for the interactive story experience.</p>
    <p>
      You can start the story by navigating to the
      <router-link to="/player/1">first scene</router-link>.
    </p>
    <p>
      Or, you can manage the story content in the
      <router-link to="/admin">admin panel</router-link>.
    </p>
  </div>
</template>

<script setup>
import { RouterLink } from 'vue-router'
</script>

<style scoped src="../assets/styles/Home.css"></style>
