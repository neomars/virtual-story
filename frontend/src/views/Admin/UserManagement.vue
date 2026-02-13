
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
          <label>Old password</label>
          <input type="password" v-model="passChange.oldPassword" required />
        </div>
        <div class="form-group">
          <label>New password</label>
          <input type="password" v-model="passChange.newPassword" required />
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
        <input type="text" v-model="newUser.username" placeholder="Username" required />
        <input type="password" v-model="newUser.password" placeholder="Password" required />
        <button type="submit" class="button" :disabled="isCreatingUser">Add</button>
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
              title="Delete"
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
    alert(err.response?.data?.message || 'Failed to change password.');
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
    alert(err.response?.data?.message || 'Failed to create user.');
  } finally {
    isCreatingUser.value = false;
  }
};

const deleteUser = async (id) => {
  if (confirm('Delete this user?')) {
    try {
      await axios.delete(`/api/admin/users/${id}`);
      fetchUsers();
    } catch (err) {
      alert(err.response?.data?.message || 'Deletion failed.');
    }
  }
};

onMounted(fetchUsers);
</script>

<style scoped>
.user-management {
  padding: 2rem;
  max-width: 800px;
  margin: 0 auto;
}

.header-container {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  margin-bottom: 2rem;
}

.back-link {
  color: #42b983;
  text-decoration: none;
  font-size: 0.9rem;
}

.admin-section {
  background-color: #2a2a2a;
  padding: 1.5rem;
  border-radius: 8px;
  margin-bottom: 2rem;
}

.admin-section h2 {
  margin-top: 0;
  margin-bottom: 1.5rem;
  font-size: 1.2rem;
  color: #42b983;
  border-bottom: 1px solid #444;
  padding-bottom: 0.5rem;
}

.settings-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 400px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.form-group label {
  font-size: 0.9rem;
  color: #aaa;
}

.form-group input {
  background-color: #1e1e1e;
  border: 1px solid #444;
  color: white;
  padding: 0.6rem;
  border-radius: 4px;
}

.button {
  background-color: #42b983;
  color: white;
  border: none;
  padding: 0.6rem 1rem;
  border-radius: 4px;
  cursor: pointer;
  font-weight: bold;
}

.button:disabled {
  opacity: 0.6;
}

.separator {
  border: none;
  border-top: 1px solid #444;
  margin: 2rem 0;
}

.add-user-form {
  display: flex;
  gap: 1rem;
  margin-bottom: 1.5rem;
}

.add-user-form input {
  flex: 1;
  background-color: #1e1e1e;
  border: 1px solid #444;
  color: white;
  padding: 0.6rem;
  border-radius: 4px;
}

.user-list {
  list-style: none;
  padding: 0;
}

.user-list li {
  background-color: #333;
  padding: 0.75rem 1rem;
  border-radius: 4px;
  margin-bottom: 0.5rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.self-badge {
  background-color: #42b983;
  font-size: 0.7rem;
  padding: 2px 6px;
  border-radius: 10px;
  margin-left: 10px;
}

.button-delete {
  background: #ef4444;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  padding: 0.2rem 0.5rem;
  font-size: 1rem;
}

.button-delete:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}
</style>
