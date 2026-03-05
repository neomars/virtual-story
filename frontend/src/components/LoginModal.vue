
<template>
  <div v-if="isOpen" class="modal-overlay" @click.self="close">
    <div class="modal-content">
      <div class="modal-header">
        <h2>Admin Login</h2>
        <button class="close-btn" @click="close" aria-label="Close"><span aria-hidden="true">&times;</span></button>
      </div>
      <form @submit.prevent="handleLogin" class="login-form">
        <div class="form-group">
          <label for="username">Username</label>
          <input
            type="text"
            id="username"
            v-model="username"
            required
            placeholder="admin"
            :disabled="isLoading"
          >
        </div>
        <div class="form-group">
          <label for="password">Password</label>
          <input
            type="password"
            id="password"
            v-model="password"
            required
            :disabled="isLoading"
          >
        </div>
        <div v-if="errorMessage" class="error-message" role="alert">
          {{ errorMessage }}
        </div>
        <button type="submit" class="submit-btn" :disabled="isLoading">
          {{ isLoading ? 'Connecting...' : 'Login' }}
        </button>
      </form>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue';
import axios from 'axios';

const props = defineProps({
  isOpen: Boolean
});

const emit = defineEmits(['close', 'login-success']);

const username = ref('');
const password = ref('');
const isLoading = ref(false);
const errorMessage = ref('');

const close = () => {
  if (isLoading.value) return;
  errorMessage.value = '';
  username.value = '';
  password.value = '';
  emit('close');
};

const handleLogin = async () => {
  isLoading.value = true;
  errorMessage.value = '';
  try {
    const response = await axios.post('/api/auth/login', {
      username: username.value,
      password: password.value
    });
    emit('login-success', response.data);
    close();
  } catch (err) {
    errorMessage.value = err.response?.data?.message || 'An error occurred during login.';
  } finally {
    isLoading.value = false;
  }
};
</script>

<style scoped src="../assets/styles/LoginModal.css"></style>
