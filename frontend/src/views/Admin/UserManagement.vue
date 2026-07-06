
<template>
  <div class="user-management">
    <div class="header-container">
      <router-link to="/admin/scenes" class="back-link">
        <span aria-hidden="true">&larr;</span> Back to Graph
      </router-link>
      <h1>Users Management</h1>
    </div>

    <Transition name="fade">
      <div v-if="statusMessage" class="status-message" :class="isSuccess ? 'status-success' : 'status-error'" role="status">
        {{ statusMessage }}
      </div>
    </Transition>

    <!-- Password Change Section -->
    <section class="admin-section">
      <h2>Change my password</h2>
      <form @submit.prevent="changePassword" class="settings-form">
        <div class="form-group">
          <label for="old-password">Old password</label>
          <input type="password" id="old-password" v-model="passChange.oldPassword" required />
        </div>
        <div class="form-group">
          <label for="new-password">New password</label>
          <input type="password" id="new-password" v-model="passChange.newPassword" required />
        </div>
        <button type="submit" class="button" :disabled="isChangingPass">
          {{ isChangingPass ? 'Changing...' : 'Update password' }}
        </button>
      </form>
    </section>

    <hr class="separator" />

    <!-- User List & Add User Section -->
    <section class="admin-section">
      <h2>Users</h2>
      <form @submit.prevent="createUser" class="add-user-form">
        <label for="new-username" class="sr-only">Username</label>
        <input type="text" id="new-username" v-model="newUser.username" placeholder="Username" required />
        <label for="new-password-field" class="sr-only">Password</label>
        <input type="password" id="new-password-field" v-model="newUser.password" placeholder="Password" required />
        <button type="submit" class="button" :disabled="isCreatingUser">
          {{ isCreatingUser ? 'Adding...' : 'Add' }}
        </button>
      </form>

      <ul class="user-list">
        <li v-for="user in users" :key="user.id">
          <div class="user-info">
            <strong>{{ user.username }}</strong>
            <span v-if="user.id === auth.currentUser.value?.id" class="self-badge">Me</span>
          </div>
          <div class="user-actions">
            <button
              @click="deleteUser(user)"
              class="button-delete"
              :disabled="user.id === auth.currentUser.value?.id || deletingUserId === user.id"
              :title="deletingUserId === user.id ? 'Deleting...' : (user.id === auth.currentUser.value?.id ? 'You cannot delete yourself' : 'Delete user: ' + user.username)"
              :aria-label="deletingUserId === user.id ? 'Deleting user: ' + user.username : (user.id === auth.currentUser.value?.id ? 'You cannot delete yourself' : 'Delete user: ' + user.username)"
              >
                <span v-if="deletingUserId === user.id" aria-hidden="true">...</span>
                <span v-else aria-hidden="true">&times;</span>
              </button>
          </div>
        </li>
      </ul>
    </section>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, inject } from 'vue';
import axios from 'axios';

const auth = inject('auth');

const users = ref([]);
const passChange = ref({ oldPassword: '', newPassword: '' });
const newUser = ref({ username: '', password: '' });

const isChangingPass = ref(false);
const isCreatingUser = ref(false);
const deletingUserId = ref(null);
const statusMessage = ref('');
const isSuccess = ref(true);
let timeoutId = null;

const showStatus = (msg, success = true) => {
  statusMessage.value = msg;
  isSuccess.value = success;
  if (timeoutId) clearTimeout(timeoutId);
  timeoutId = setTimeout(() => {
    statusMessage.value = '';
  }, 5000);
};

onUnmounted(() => {
  if (timeoutId) clearTimeout(timeoutId);
});

const fetchUsers = async () => {
  try {
    const res = await axios.get('/api/admin/users');
    users.value = res.data;
  } catch (err) {
    console.error('Failed to fetch users');
  }
};

const changePassword = async () => {
  isChangingPass.value = true;
  try {
    await axios.post('/api/admin/change-password', passChange.value);
    showStatus('Password changed successfully!');
    passChange.value = { oldPassword: '', newPassword: '' };
  } catch (err) {
    showStatus(err.response?.data?.message || err.message || 'Failed to change password.', false);
  } finally {
    isChangingPass.value = false;
  }
};

const createUser = async () => {
  isCreatingUser.value = true;
  try {
    await axios.post('/api/admin/users', newUser.value);
    showStatus(`User "${newUser.value.username}" created!`);
    newUser.value = { username: '', password: '' };
    fetchUsers();
  } catch (err) {
    showStatus(err.response?.data?.message || err.message || 'Failed to create user.', false);
  } finally {
    isCreatingUser.value = false;
  }
};

const deleteUser = async (user) => {
  if (confirm(`Delete user "${user.username}"?`)) {
    deletingUserId.value = user.id;
    try {
      await axios.delete(`/api/admin/users/${user.id}`);
      showStatus(`User "${user.username}" deleted.`);
      fetchUsers();
    } catch (err) {
      showStatus(err.response?.data?.message || err.message || 'Deletion failed.', false);
    } finally {
      deletingUserId.value = null;
    }
  }
};

onMounted(fetchUsers);
</script>

<style scoped src="../../assets/styles/UserManagement.css"></style>
