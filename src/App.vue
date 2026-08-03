<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { RadioCatalog, RadioSource } from './types/radio'
import { useRadioPlayer } from './composables/useRadioPlayer'

const {
  currentSource,
  state,
  errorMessage,
  volume,
  play,
  togglePlay,
  stop,
  setVolume,
} = useRadioPlayer()

const catalog = ref<RadioCatalog | null>(null)
const favorites = ref<Set<string>>(new Set())
const searchQuery = ref('')
const selectedCategory = ref('全部')
const loading = ref(false)
const currentView = ref<'home' | 'settings'>('home')
const catalogUrl = ref('')
const fileInput = ref<HTMLInputElement | null>(null)
const toast = ref('')

const categories = computed(() => {
  if (!catalog.value) return ['全部']
  const set = new Set<string>()
  catalog.value.sources.forEach((s) => {
    if (s.category) set.add(s.category)
  })
  return ['全部', ...Array.from(set).sort()]
})

const filteredSources = computed(() => {
  if (!catalog.value) return []
  const query = searchQuery.value.trim().toLowerCase()
  return catalog.value.sources.filter((s) => {
    const matchCategory =
      selectedCategory.value === '全部' || s.category === selectedCategory.value
    const matchQuery =
      !query ||
      s.name.toLowerCase().includes(query) ||
      (s.region?.toLowerCase().includes(query) ?? false) ||
      (s.description?.toLowerCase().includes(query) ?? false)
    return matchCategory && matchQuery
  })
})

function showToast(message: string) {
  toast.value = message
  setTimeout(() => {
    toast.value = ''
  }, 2500)
}

async function loadCatalog() {
  try {
    const data = await invoke<RadioCatalog>('get_radio_catalog')
    catalog.value = data
  } catch (e) {
    console.error('加载目录失败:', e)
    // 降级：尝试读取内置 JSON 并保存到本地
    try {
      const res = await fetch('/radio-catalog.json')
      if (res.ok) {
        const data = await res.json()
        catalog.value = data
        await invoke('save_radio_catalog', { catalog: data }).catch(() => {})
      }
    } catch (err) {
      console.error('读取内置目录失败:', err)
    }
  }
}

async function loadFavorites() {
  try {
    const list = await invoke<string[]>('get_favorites')
    favorites.value = new Set(list)
  } catch (e) {
    console.error('加载收藏失败:', e)
  }
}

async function toggleFavorite(source: RadioSource) {
  try {
    const isFav = await invoke<boolean>('toggle_favorite', { id: source.id })
    if (isFav) {
      favorites.value.add(source.id)
    } else {
      favorites.value.delete(source.id)
    }
  } catch (e) {
    console.error('切换收藏失败:', e)
  }
}

async function updateCatalogFromUrl() {
  const url = catalogUrl.value.trim()
  if (!url) return
  loading.value = true
  try {
    await invoke('update_radio_catalog_from_url', { url })
    await loadCatalog()
    catalogUrl.value = ''
    showToast('目录更新成功')
  } catch (e) {
    showToast('更新失败: ' + String(e))
  } finally {
    loading.value = false
  }
}

function triggerFileSelect() {
  fileInput.value?.click()
}

async function onFileSelected(event: Event) {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return

  try {
    const text = await file.text()
    const data = JSON.parse(text) as RadioCatalog
    await invoke('save_radio_catalog', { catalog: data })
    await loadCatalog()
    showToast('目录导入成功')
  } catch (e) {
    showToast('导入失败: ' + String(e))
  } finally {
    target.value = ''
  }
}

function handlePlay(source: RadioSource) {
  if (currentSource.value?.id === source.id && state.value === 'playing') {
    togglePlay()
  } else {
    play(source)
    invoke('add_play_history', { id: source.id }).catch(() => {})
  }
}

onMounted(() => {
  loadCatalog()
  loadFavorites()
})
</script>

<template>
  <div class="app">
    <header class="app-header">
      <div class="brand">
        <span class="brand-icon">📻</span>
        <span class="brand-text">TingFM Radio</span>
      </div>
      <div class="search-box">
        <span class="search-icon">🔍</span>
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索电台..."
          @keydown.enter="searchQuery = searchQuery.trim()"
        />
      </div>
      <button class="settings-btn" @click="currentView = 'settings'">⚙️</button>
    </header>

    <main class="main-content">
      <div v-if="currentView === 'home'" class="home-view">
        <div class="category-bar">
          <button
            v-for="cat in categories"
            :key="cat"
            class="category-chip"
            :class="{ active: selectedCategory === cat }"
            @click="selectedCategory = cat"
          >
            {{ cat }}
          </button>
        </div>

        <div v-if="!catalog" class="loading-state">
          <p>正在加载电台目录...</p>
        </div>

        <div v-else-if="filteredSources.length === 0" class="empty-state">
          <p>没有找到匹配的电台</p>
        </div>

        <div v-else class="source-grid">
          <div
            v-for="source in filteredSources"
            :key="source.id"
            class="source-card"
            :class="{ playing: currentSource?.id === source.id }"
          >
            <div class="card-cover">
              <img
                v-if="source.logo"
                :src="source.logo"
                :alt="source.name"
                @error="($event.target as HTMLImageElement).style.display = 'none'"
              />
              <div v-else class="cover-fallback">📻</div>
              <button class="play-overlay" @click="handlePlay(source)">
                {{ currentSource?.id === source.id && state === 'playing' ? '⏸' : '▶' }}
              </button>
            </div>
            <div class="card-info">
              <h3 class="card-title">{{ source.name }}</h3>
              <p class="card-meta">
                <span v-if="source.category">{{ source.category }}</span>
                <span v-if="source.region">{{ source.region }}</span>
              </p>
              <p v-if="source.description" class="card-desc">{{ source.description }}</p>
            </div>
            <button
              class="favorite-btn"
              :class="{ active: favorites.has(source.id) }"
              @click="toggleFavorite(source)"
            >
              {{ favorites.has(source.id) ? '❤️' : '🤍' }}
            </button>
          </div>
        </div>
      </div>

      <div v-else class="settings-view">
        <div class="settings-header">
          <button class="back-btn" @click="currentView = 'home'">←</button>
          <h2>设置</h2>
          <div class="spacer"></div>
        </div>

        <div class="settings-content">
          <section class="section">
            <h3>更新电台目录</h3>
            <div class="input-row">
              <input
                v-model="catalogUrl"
                type="text"
                placeholder="输入目录 JSON 地址..."
                @keydown.enter="updateCatalogFromUrl"
              />
              <button :disabled="loading" @click="updateCatalogFromUrl">
                {{ loading ? '更新中...' : '更新' }}
              </button>
            </div>
            <button class="secondary-btn" @click="triggerFileSelect">
              从本地 JSON 文件导入
            </button>
            <input
              ref="fileInput"
              type="file"
              accept=".json,application/json"
              style="display: none"
              @change="onFileSelected"
            />
          </section>

          <section class="section">
            <h3>关于</h3>
            <div class="about-card">
              <p><strong>TingFM Radio</strong></p>
              <p>基于 Tauri 2 + Vue 3 构建</p>
              <p>目录版本: {{ catalog?.version ?? '未加载' }}</p>
              <p>电台数量: {{ catalog?.sources.length ?? 0 }}</p>
            </div>
          </section>
        </div>
      </div>
    </main>

    <footer v-if="currentView === 'home'" class="player-bar">
      <div class="player-info">
        <div class="player-cover">
          <img
            v-if="currentSource?.logo"
            :src="currentSource.logo"
            :alt="currentSource.name"
          />
          <div v-else class="player-cover-fallback">📻</div>
        </div>
        <div class="player-text">
          <p class="player-name">{{ currentSource?.name || '未在播放' }}</p>
          <p class="player-status">
            <span v-if="errorMessage" class="error">{{ errorMessage }}</span>
            <span v-else-if="state === 'loading'">加载中...</span>
            <span v-else-if="state === 'playing'">正在播放</span>
            <span v-else-if="state === 'paused'">已暂停</span>
            <span v-else>选择电台开始收听</span>
          </p>
        </div>
      </div>

      <div class="player-controls">
        <button class="control-btn" :disabled="!currentSource" @click="togglePlay">
          {{ state === 'playing' ? '⏸' : '▶' }}
        </button>
        <button class="control-btn" :disabled="!currentSource" @click="stop">⏹</button>
      </div>

      <div class="player-volume">
        <span>🔊</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          :value="volume"
          @input="setVolume(Number(($event.target as HTMLInputElement).value))"
        />
      </div>
    </footer>

    <div v-if="toast" class="toast">{{ toast }}</div>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  height: 100dvh;
  background: linear-gradient(135deg, #0f0c29 0%, #302b63 50%, #24243e 100%);
  color: #fff;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  overflow: hidden;
  padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom)
    env(safe-area-inset-left);
}

.app-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 20px;
  background: rgba(0, 0, 0, 0.25);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 20px;
  font-weight: 700;
  flex-shrink: 0;
}

.brand-icon {
  font-size: 24px;
}

.brand-text {
  background: linear-gradient(90deg, #00d4ff, #a855f7);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.search-box {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 10px;
  padding: 8px 14px;
  max-width: 420px;
}

.search-box input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: #fff;
  font-size: 14px;
}

.search-box input::placeholder {
  color: rgba(255, 255, 255, 0.45);
}

.settings-btn {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 10px;
  color: #fff;
  font-size: 18px;
  width: 40px;
  height: 40px;
  cursor: pointer;
  flex-shrink: 0;
}

.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.home-view {
  max-width: 1200px;
  margin: 0 auto;
}

.category-bar {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 20px;
}

.category-chip {
  padding: 6px 14px;
  border-radius: 20px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.8);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}

.category-chip:hover,
.category-chip.active {
  background: linear-gradient(90deg, #00d4ff, #a855f7);
  border-color: transparent;
  color: #fff;
}

.loading-state,
.empty-state {
  text-align: center;
  padding: 80px 20px;
  color: rgba(255, 255, 255, 0.5);
}

.source-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 16px;
  padding-bottom: 100px;
}

.source-card {
  position: relative;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 16px;
  padding: 14px;
  transition: all 0.2s;
}

.source-card:hover {
  background: rgba(255, 255, 255, 0.1);
  transform: translateY(-2px);
}

.source-card.playing {
  border-color: #00d4ff;
  box-shadow: 0 0 0 1px #00d4ff;
}

.card-cover {
  position: relative;
  width: 100%;
  aspect-ratio: 1;
  border-radius: 12px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.08);
  margin-bottom: 12px;
}

.card-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.cover-fallback {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 48px;
}

.play-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.35);
  border: none;
  color: #fff;
  font-size: 36px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.2s;
}

.source-card:hover .play-overlay {
  opacity: 1;
}

.card-info {
  min-width: 0;
}

.card-title {
  margin: 0 0 6px;
  font-size: 15px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.card-meta {
  margin: 0 0 6px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.55);
  display: flex;
  gap: 8px;
}

.card-desc {
  margin: 0;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.4);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.favorite-btn {
  position: absolute;
  top: 10px;
  right: 10px;
  background: rgba(0, 0, 0, 0.4);
  border: none;
  border-radius: 50%;
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: 16px;
}

.settings-view {
  max-width: 720px;
  margin: 0 auto;
}

.settings-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}

.back-btn {
  width: 36px;
  height: 36px;
  border: none;
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
  border-radius: 10px;
  cursor: pointer;
  font-size: 18px;
}

.settings-header h2 {
  flex: 1;
  margin: 0;
  font-size: 18px;
  text-align: center;
}

.spacer {
  width: 36px;
}

.settings-content {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.section h3 {
  margin: 0 0 12px;
  font-size: 14px;
  color: rgba(255, 255, 255, 0.5);
}

.input-row {
  display: flex;
  gap: 12px;
  margin-bottom: 12px;
}

.input-row input {
  flex: 1;
  padding: 12px 14px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.06);
  color: #fff;
  outline: none;
}

.input-row input::placeholder {
  color: rgba(255, 255, 255, 0.4);
}

.input-row button,
.secondary-btn {
  padding: 12px 20px;
  border-radius: 10px;
  border: none;
  background: linear-gradient(90deg, #00d4ff, #a855f7);
  color: #fff;
  font-weight: 500;
  cursor: pointer;
}

.secondary-btn {
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.about-card {
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  padding: 16px;
  color: rgba(255, 255, 255, 0.75);
}

.about-card p {
  margin: 6px 0;
}

.player-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 20px;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(16px);
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}

.player-info {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
  flex: 1;
}

.player-cover,
.player-cover-fallback {
  width: 52px;
  height: 52px;
  border-radius: 10px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}

.player-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.player-cover-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
}

.player-text {
  min-width: 0;
}

.player-name {
  margin: 0 0 4px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.player-status {
  margin: 0;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.55);
}

.player-status .error {
  color: #ff6b6b;
}

.player-controls {
  display: flex;
  align-items: center;
  gap: 12px;
}

.control-btn {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: none;
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
  font-size: 18px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.control-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.player-volume {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 140px;
  flex-shrink: 0;
}

.player-volume input {
  flex: 1;
}

.toast {
  position: fixed;
  bottom: 90px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.8);
  color: #fff;
  padding: 10px 20px;
  border-radius: 20px;
  font-size: 13px;
  z-index: 1000;
  pointer-events: none;
}

@media (max-width: 640px) {
  .app-header {
    gap: 10px;
    padding: 10px 14px;
  }

  .brand-text {
    display: none;
  }

  .search-box {
    max-width: none;
  }

  .source-grid {
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  .player-bar {
    flex-wrap: wrap;
    padding: 10px 14px;
  }

  .player-volume {
    width: 100%;
  }
}
</style>
