
import { createRouter, createWebHistory } from 'vue-router'
import Home from '../views/Home.vue'
import SceneList from '../views/Admin/SceneList.vue'
import SceneEdit from '../views/Admin/SceneEdit.vue'
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
