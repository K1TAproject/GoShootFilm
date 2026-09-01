import { createRouter, createWebHistory } from 'vue-router'
import Home from './views/Home.vue'
import Cameras from './views/Cameras.vue'
import Films from './views/Films.vue'
import Rolls from './views/Rolls.vue'

const routes = [
    { path: '/', component: Home },
    { path: '/cameras', component: Cameras },
    { path: '/films', component: Films },
    { path: '/rolls', component: Rolls }
]

const router = createRouter({
    history: createWebHistory(),
    routes
})

export default router