
import { createRouter, createWebHistory } from 'vue-router'
import axios from 'axios'
import Home from '../views/Home.vue'
import SceneList from '../views/Admin/SceneList.vue'
import SceneEdit from '../views/Admin/SceneEdit.vue'
import UserManagement from '../views/Admin/UserManagement.vue'
import Player from '../views/Player.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: Home
    },
    {
      path: '/player/:id',
      name: 'player',
      component: Player,
      props: true
    },
    {
      path: '/admin/scenes',
      name: 'admin-scenes',
      component: SceneList
    },
    {
      path: '/admin/users',
      name: 'admin-users',
      component: UserManagement
    },
    {
      path: '/admin/scenes/new',
      name: 'admin-scene-new',
      component: SceneEdit
    },
    {
      path: '/admin/scenes/:id/edit',
      name: 'admin-scene-edit',
      component: SceneEdit,
      props: true
    },
  ]
})

router.beforeEach(async (to, from, next) => {
  if (to.path.startsWith('/admin')) {
    try {
      await axios.get('/api/auth/me');
      next();
    } catch (err) {
      // If not authenticated, we could redirect to home or just stay where we are.
      // Since clicking the Admin link in header is handled in App.vue,
      // this guard is mostly for direct URL access.
      next('/');
      // We might want to trigger the login modal here, but that's tricky from the router.
      // App.vue will handle it if it sees we are not logged in and we are trying to go to admin.
    }
  } else {
    next();
  }
});

export default router
