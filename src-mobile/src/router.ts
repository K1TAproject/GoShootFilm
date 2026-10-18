import { createRouter, createWebHashHistory } from 'vue-router'
import { hasCachedData, init, store } from './lib/store'
import HomeView from './views/HomeView.vue'
import EquipmentView from './views/EquipmentView.vue'
import FilmsView from './views/FilmsView.vue'
import RollsView from './views/RollsView.vue'
import AlbumsView from './views/AlbumsView.vue'
import RollDetailView from './views/RollDetailView.vue'
import AlbumDetailView from './views/AlbumDetailView.vue'
import SettingsView from './views/SettingsView.vue'
import PairView from './views/PairView.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'home', component: HomeView, meta: { nav: 'home' } },
    {
      path: '/equipment',
      name: 'equipment',
      component: EquipmentView,
      meta: { nav: 'equipment' },
    },
    { path: '/films', name: 'films', component: FilmsView, meta: { nav: 'films' } },
    { path: '/rolls', name: 'rolls', component: RollsView, meta: { nav: 'rolls' } },
    {
      path: '/rolls/:id',
      name: 'roll-detail',
      component: RollDetailView,
      meta: { hideNav: true },
    },
    { path: '/albums', name: 'albums', component: AlbumsView, meta: { nav: 'albums' } },
    {
      path: '/albums/:id',
      name: 'album-detail',
      component: AlbumDetailView,
      meta: { hideNav: true },
    },
    {
      path: '/settings',
      name: 'settings',
      component: SettingsView,
      meta: { hideNav: true },
    },
    { path: '/pair', name: 'pair', component: PairView, meta: { hideNav: true } },
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
  scrollBehavior(_to, _from, savedPosition) {
    return savedPosition ?? { top: 0 }
  },
})

router.beforeEach(async (to) => {
  await init()
  if (!store.paired && !hasCachedData.value && to.name !== 'pair' && to.name !== 'settings') {
    return { name: 'pair' }
  }
  if (store.paired && to.name === 'pair') {
    return { name: 'home' }
  }
  return true
})

export default router
