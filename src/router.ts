import { createRouter, createWebHashHistory } from 'vue-router'
import Home from './views/Home.vue'
import Cameras from './views/Cameras.vue'
import Films from './views/Films.vue'
import Rolls from './views/Rolls.vue'

const routes = [
    { path: '/', name: 'home', component: Home },
    { path: '/cameras', name: 'cameras', component: Cameras },
    { path: '/films', name: 'films', component: Films },
    { path: '/rolls', name: 'rolls', component: Rolls },
    { path: '/:pathMatch(.*)*', redirect: '/' }
]

const router = createRouter({
    history: createWebHashHistory(),
    routes
})

export default router
