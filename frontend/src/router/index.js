
import { createRouter, createWebHistory } from 'vue-router'
import SceneList from '../views/Admin/SceneList.vue'
import SceneEdit from '../views/Admin/SceneEdit.vue'
import Player from '../views/Player.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      // For simplicity, redirect to the first scene.
      // A real app might have a proper landing page.
      redirect: '/player/1'
    },
    {
      path: '/player/:id',
      name: 'player',
      component: Player,
      props: true
    },
    {
      path: '/admin',
      name: 'admin',
      component: SceneList
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

export default router
