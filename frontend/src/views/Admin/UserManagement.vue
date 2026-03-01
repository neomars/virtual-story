
<template>
  <div class="user-management">
    <div class="header-container">
      <router-link to="/admin/scenes" class="back-link">&larr; Back to Graph</router-link>
      <h1>Users Management</h1>
    </div>

    <!-- Password Change Section -->
    <section class="admin-section">
      <h2>Change my password</h2>
      <form @submit.prevent="changePassword" class="settings-form">
        <div class="form-group">
          <label for="old-password">Old password</label>
          <input id="old-password" type="password" v-model="passChange.oldPassword" :disabled="isChangingPass" required />
        </div>
        <div class="form-group">
          <label for="new-password">New password</label>
          <input id="new-password" type="password" v-model="passChange.newPassword" :disabled="isChangingPass" required />
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
        <label for="new-username" class="sr-only">New username</label>
        <input id="new-username" type="text" v-model="newUser.username" placeholder="Username" :disabled="isCreatingUser" required />
        <label for="new-password-input" class="sr-only">New user password</label>
        <input id="new-password-input" type="password" v-model="newUser.password" placeholder="Password" :disabled="isCreatingUser" required />
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
              @click="deleteUser(user.id)"
              class="button-delete"
              :disabled="user.id === auth.currentUser.value?.id"
              :title="user.id === auth.currentUser.value?.id ? 'You cannot delete yourself' : 'Delete user: ' + user.username"
              :aria-label="user.id === auth.currentUser.value?.id ? 'You cannot delete yourself' : 'Delete user: ' + user.username"
            >&times;</button>
          </div>
        </li>
      </ul>
    </section>
  </div>
</template>

<script setup>
import { ref, onMounted, inject } from 'vue';
import axios from 'axios';

const auth = inject('auth');

const users = ref([]);
const passChange = ref({ oldPassword: '', newPassword: '' });
const newUser = ref({ username: '', password: '' });

const isChangingPass = ref(false);
const isCreatingUser = ref(false);

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
    alert('Password changed successfully!');
    passChange.value = { oldPassword: '', newPassword: '' };
  } catch (err) {
    alert(err.response?.data?.message || err.message || 'Failed to change password.');
  } finally {
    isChangingPass.value = false;
  }
};

const createUser = async () => {
  isCreatingUser.value = true;
  try {
    await axios.post('/api/admin/users', newUser.value);
    newUser.value = { username: '', password: '' };
    fetchUsers();
  } catch (err) {
    alert(err.response?.data?.message || err.message || 'Failed to create user.');
  } finally {
    isCreatingUser.value = false;
  }
};

const deleteUser = async (id) => {
  const user = users.value.find(u => u.id === id);
  const username = user ? user.username : 'this user';
  if (confirm(`Delete user "${username}"?`)) {
    try {
      await axios.delete(`/api/admin/users/${id}`);
      fetchUsers();
    } catch (err) {
      alert(err.response?.data?.message || err.message || 'Deletion failed.');
    }
  }
};

onMounted(fetchUsers);
</script>

<style scoped src="../../assets/styles/UserManagement.css"></style>
