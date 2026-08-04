<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { RadioCatalog, RadioSource } from './types/radio'
import { useRadioPlayer } from './composables/useRadioPlayer'

const {
  currentSource,
  state,
  errorMessage,
  play,
  togglePlay,
  stop,
} = useRadioPlayer()

const catalog = ref<RadioCatalog | null>(null)
const favorites = ref<Set<string>>(new Set())
const currentCategory = ref('全部')
const currentIndex = ref(0)
const showCategoryPicker = ref(false)
const showTimerPicker = ref(false)
const showSkinPicker = ref(false)
const currentSkin = ref<'wood' | 'transistor' | 'tube'>('wood')
const timerEndAt = ref<number | null>(null)
const timerDisplay = ref('')
let timerInterval: ReturnType<typeof setInterval> | null = null

const skins = [
  { id: 'wood', name: '复古木纹' },
  { id: 'transistor', name: '晶体管' },
  { id: 'tube', name: '电子管' },
] as const

const categories = computed(() => {
  if (!catalog.value) return ['全部']
  const set = new Set<string>()
  catalog.value.sources.forEach((s) => {
    if (s.region) set.add(s.region)
  })
  return ['全部', ...Array.from(set).sort()]
})

const filteredSources = computed(() => {
  if (!catalog.value) return []
  if (currentCategory.value === '全部') return catalog.value.sources
  return catalog.value.sources.filter((s) => s.region === currentCategory.value)
})

const currentStation = computed<RadioSource | null>(() => {
  const list = filteredSources.value
  if (!list.length) return null
  const idx = Math.max(0, Math.min(currentIndex.value, list.length - 1))
  return list[idx]
})

const isFavorite = computed(() => {
  return currentStation.value ? favorites.value.has(currentStation.value.id) : false
})

const isPlaying = computed(() => state.value === 'playing')

watch(currentCategory, () => {
  currentIndex.value = 0
  const station = currentStation.value
  if (station) handlePlay(station)
})

function updateTimerDisplay() {
  if (!timerEndAt.value) {
    timerDisplay.value = ''
    return
  }
  const diff = Math.max(0, timerEndAt.value - Date.now())
  if (diff <= 0) {
    timerDisplay.value = ''
    return
  }
  const totalSeconds = Math.ceil(diff / 1000)
  const m = Math.floor(totalSeconds / 60)
  const s = totalSeconds % 60
  timerDisplay.value = `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
}

function startTimerInterval() {
  stopTimerInterval()
  updateTimerDisplay()
  timerInterval = setInterval(() => {
    updateTimerDisplay()
    if (timerEndAt.value && Date.now() >= timerEndAt.value) {
      stop()
      timerEndAt.value = null
      stopTimerInterval()
    }
  }, 1000)
}

function stopTimerInterval() {
  if (timerInterval) {
    clearInterval(timerInterval)
    timerInterval = null
  }
}

async function loadCatalog() {
  try {
    const data = await invoke<RadioCatalog>('get_radio_catalog')
    catalog.value = data
  } catch (e) {
    try {
      const res = await fetch('/radio-catalog.json')
      if (res.ok) {
        const data = await res.json()
        catalog.value = data
        await invoke('save_radio_catalog', { catalog: data }).catch(() => {})
      }
    } catch (err) {
      console.error('读取目录失败:', err)
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

async function toggleFavorite() {
  const station = currentStation.value
  if (!station) return
  try {
    const isFav = await invoke<boolean>('toggle_favorite', { id: station.id })
    if (isFav) favorites.value.add(station.id)
    else favorites.value.delete(station.id)
  } catch (e) {
    console.error('切换收藏失败:', e)
  }
}

function handlePlay(station: RadioSource) {
  if (currentSource.value?.id === station.id && isPlaying.value) {
    togglePlay()
  } else {
    play(station)
    invoke('add_play_history', { id: station.id }).catch(() => {})
  }
}

function nextStation() {
  const list = filteredSources.value
  if (!list.length) return
  currentIndex.value = (currentIndex.value + 1) % list.length
  handlePlay(currentStation.value!)
}

function prevStation() {
  const list = filteredSources.value
  if (!list.length) return
  currentIndex.value = (currentIndex.value - 1 + list.length) % list.length
  handlePlay(currentStation.value!)
}

function selectCategory(cat: string) {
  currentCategory.value = cat
  showCategoryPicker.value = false
}

function setTimer(minutes: number) {
  if (minutes <= 0) {
    timerEndAt.value = null
    stopTimerInterval()
    showTimerPicker.value = false
    return
  }
  timerEndAt.value = Date.now() + minutes * 60 * 1000
  startTimerInterval()
  showTimerPicker.value = false
}

function selectSkin(skin: 'wood' | 'transistor' | 'tube') {
  currentSkin.value = skin
  showSkinPicker.value = false
}

function onTunerClick(event: MouseEvent) {
  const target = event.currentTarget as HTMLElement
  const rect = target.getBoundingClientRect()
  const x = event.clientX - rect.left
  const center = rect.width / 2
  if (x < center) prevStation()
  else nextStation()
}

onMounted(() => {
  loadCatalog()
  loadFavorites()
})

onUnmounted(() => {
  stopTimerInterval()
})
</script>

<template>
  <div class="radio-app" :data-skin="currentSkin">
    <header class="app-header">
      <div class="brand-mark">📻</div>
      <button class="skin-btn" @click="showSkinPicker = true">
        <span class="skin-icon">🎨</span>
        <span class="skin-name">{{ skins.find((s) => s.id === currentSkin)?.name }}</span>
      </button>
    </header>

    <main class="main-stage">
      <div v-if="!catalog" class="loading-text">加载中...</div>
      <div v-else-if="!currentStation" class="empty-text">暂无电台</div>

      <div v-else class="station-card" :class="{ playing: isPlaying }">
        <div class="card-screw screw-tl"></div>
        <div class="card-screw screw-tr"></div>
        <div class="card-screw screw-bl"></div>
        <div class="card-screw screw-br"></div>

        <div class="tuner-window">
          <div class="led-panel">
            <span class="led" :class="{ on: isPlaying }"></span>
            <span class="freq-text">{{ isPlaying ? 'ON AIR' : 'STANDBY' }}</span>
          </div>
        </div>

        <div class="cover-area">
          <div class="cover-frame">
            <div class="cover-mask"></div>
            <img
              v-if="currentStation.logo"
              :src="currentStation.logo"
              :alt="currentStation.name"
              @error="($event.target as HTMLImageElement).style.display = 'none'"
            />
            <div v-else class="cover-placeholder">📻</div>
          </div>
        </div>

        <div class="info-area">
          <h1 class="station-name">{{ currentStation.name }}</h1>
          <p class="station-meta">
            <span class="meta-chip">{{ currentStation.region }}</span>
            <span v-if="currentStation.category" class="meta-chip secondary">{{ currentStation.category }}</span>
          </p>
        </div>
      </div>
    </main>

    <footer class="control-deck">
      <div class="knobs-row">
        <div class="knob-wrap">
          <button class="physical-knob tuner-knob" @click="onTunerClick">
            <div class="knob-top">
              <div class="knob-marker"></div>
            </div>
            <div class="knob-side"></div>
          </button>
          <span class="knob-label">切台</span>
        </div>

        <div class="knob-wrap">
          <button
            class="physical-knob power-knob"
            :class="{ playing: isPlaying }"
            @click="currentStation && handlePlay(currentStation)"
          >
            <div class="knob-top">
              <div class="knob-marker"></div>
            </div>
            <div class="knob-side"></div>
          </button>
          <span class="knob-label">电源</span>
        </div>
      </div>

      <div class="text-controls">
        <button class="text-btn" :class="{ active: isFavorite }" @click="toggleFavorite">
          <span>{{ isFavorite ? '已收藏' : '收藏' }}</span>
        </button>
        <button class="text-btn" :class="{ active: timerDisplay !== '' }" @click="showTimerPicker = true">
          <span>{{ timerDisplay || '定时' }}</span>
        </button>
        <button class="text-btn" @click="showCategoryPicker = true">
          <span>{{ currentCategory === '全部' ? '频道' : currentCategory }}</span>
        </button>
      </div>

      <p v-if="errorMessage" class="error-hint">{{ errorMessage }}</p>
    </footer>

    <!-- 类别选择器 -->
    <div v-if="showCategoryPicker" class="modal-overlay" @click.self="showCategoryPicker = false">
      <div class="modal-panel retro-panel">
        <h3 class="modal-title">选择频道</h3>
        <div class="category-list">
          <button
            v-for="cat in categories"
            :key="cat"
            class="category-item"
            :class="{ active: currentCategory === cat }"
            @click="selectCategory(cat)"
          >
            {{ cat }}
          </button>
        </div>
      </div>
    </div>

    <!-- 定时选择器 -->
    <div v-if="showTimerPicker" class="modal-overlay" @click.self="showTimerPicker = false">
      <div class="modal-panel retro-panel">
        <h3 class="modal-title">定时关闭</h3>
        <div class="timer-list">
          <button class="timer-item" @click="setTimer(15)">15 分钟</button>
          <button class="timer-item" @click="setTimer(30)">30 分钟</button>
          <button class="timer-item" @click="setTimer(60)">60 分钟</button>
          <button class="timer-item" @click="setTimer(90)">90 分钟</button>
          <button class="timer-item cancel" @click="setTimer(0)">取消定时</button>
        </div>
      </div>
    </div>

    <!-- 换肤选择器 -->
    <div v-if="showSkinPicker" class="modal-overlay" @click.self="showSkinPicker = false">
      <div class="modal-panel retro-panel">
        <h3 class="modal-title">切换皮肤</h3>
        <div class="skin-list">
          <button
            v-for="skin in skins"
            :key="skin.id"
            class="skin-item"
            :class="{ active: currentSkin === skin.id }"
            @click="selectSkin(skin.id)"
          >
            {{ skin.name }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 主题变量 */
.radio-app {
  --bg-deep: #1a0f0a;
  --bg-mid: #3d261b;
  --card-bg: #f8ecd9;
  --card-bg2: #dcc29f;
  --card-border: rgba(139, 90, 43, 0.2);
  --btn-bg: #c9a86c;
  --btn-bg2: #a08255;
  --btn-shadow: #7a6240;
  --accent: #c9a86c;
  --led-on: #ff4d4d;
  --led-off: #4a3b2a;
  --text-main: #3d261b;
  --text-sub: #f5e6d3;
  --text-muted: rgba(245, 230, 211, 0.45);
  --overlay-tint: rgba(248, 236, 217, 0.15);

  display: flex;
  flex-direction: column;
  min-height: 100vh;
  min-height: 100dvh;
  background:
    radial-gradient(circle at 20% 10%, rgba(255, 255, 255, 0.04) 0%, transparent 30%),
    linear-gradient(160deg, var(--bg-mid) 0%, var(--bg-deep) 100%);
  color: var(--text-sub);
  font-family: 'Georgia', 'Times New Roman', serif;
  overflow: hidden;
  padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom)
    env(safe-area-inset-left);
  user-select: none;
  -webkit-user-select: none;
}

.radio-app[data-skin='transistor'] {
  --bg-deep: #0a0f1a;
  --bg-mid: #1a263d;
  --card-bg: #e8eef5;
  --card-bg2: #b8c5d4;
  --card-border: rgba(80, 110, 140, 0.25);
  --btn-bg: #8da3b8;
  --btn-bg2: #5f758a;
  --btn-shadow: #3e4f5f;
  --accent: #00d4ff;
  --led-on: #00d4ff;
  --led-off: #2a3a4a;
  --text-main: #1a263d;
  --text-sub: #e8eef5;
  --text-muted: rgba(232, 238, 245, 0.45);
  --overlay-tint: rgba(232, 238, 245, 0.12);
}

.radio-app[data-skin='tube'] {
  --bg-deep: #1a0a0a;
  --bg-mid: #3d1b1b;
  --card-bg: #f5e0d3;
  --card-bg2: #d4b0a0;
  --card-border: rgba(120, 60, 50, 0.25);
  --btn-bg: #c98b6c;
  --btn-bg2: #a06655;
  --btn-shadow: #7a4438;
  --accent: #ff8c42;
  --led-on: #ff8c42;
  --led-off: #4a2e26;
  --text-main: #3d1b1b;
  --text-sub: #f5e0d3;
  --text-muted: rgba(245, 224, 211, 0.45);
  --overlay-tint: rgba(245, 224, 211, 0.12);
}

/* 顶部栏 */
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  flex-shrink: 0;
}

.brand-mark {
  font-size: 26px;
  filter: drop-shadow(0 2px 2px rgba(0, 0, 0, 0.5));
}

.skin-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.1) 0%, rgba(0, 0, 0, 0.1) 100%);
  border: 1px solid var(--accent);
  border-radius: 20px;
  color: var(--accent);
  font-size: 13px;
  cursor: pointer;
  box-shadow:
    inset 0 1px 1px rgba(255, 255, 255, 0.1),
    0 3px 6px rgba(0, 0, 0, 0.3);
}

.skin-icon {
  font-size: 16px;
}

/* 主舞台 */
.main-stage {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.loading-text,
.empty-text {
  font-size: 16px;
  color: var(--text-muted);
  letter-spacing: 2px;
}

/* 复古电台卡片 */
.station-card {
  position: relative;
  width: 100%;
  max-width: 360px;
  background: linear-gradient(145deg, var(--card-bg) 0%, var(--card-bg2) 100%);
  border-radius: 28px;
  padding: 26px 22px 30px;
  box-shadow:
    0 24px 48px rgba(0, 0, 0, 0.55),
    0 12px 24px rgba(0, 0, 0, 0.35),
    inset 0 2px 4px rgba(255, 255, 255, 0.6),
    inset 0 -4px 8px rgba(0, 0, 0, 0.12);
  border: 2px solid var(--card-border);
}

.station-card.playing {
  box-shadow:
    0 24px 48px rgba(0, 0, 0, 0.55),
    0 12px 24px rgba(0, 0, 0, 0.35),
    0 0 30px color-mix(in srgb, var(--accent) 30%, transparent),
    inset 0 2px 4px rgba(255, 255, 255, 0.6),
    inset 0 -4px 8px rgba(0, 0, 0, 0.12);
}

.card-screw {
  position: absolute;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: radial-gradient(circle at 30% 30%, #b8a080, #6b5344);
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.5);
}

.screw-tl { top: 14px; left: 14px; }
.screw-tr { top: 14px; right: 14px; }
.screw-bl { bottom: 14px; left: 14px; }
.screw-br { bottom: 14px; right: 14px; }

.tuner-window {
  background: linear-gradient(180deg, #1a120d 0%, #0d0906 100%);
  border-radius: 14px;
  padding: 12px 16px;
  margin-bottom: 22px;
  box-shadow:
    inset 0 3px 6px rgba(0, 0, 0, 0.8),
    0 1px 1px rgba(255, 255, 255, 0.3);
  border: 1px solid var(--accent);
}

.led-panel {
  display: flex;
  align-items: center;
  gap: 10px;
}

.led {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--led-off);
  box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.6);
  transition: all 0.3s ease;
}

.led.on {
  background: var(--led-on);
  box-shadow:
    0 0 8px var(--led-on),
    0 0 16px color-mix(in srgb, var(--led-on) 60%, transparent),
    inset 0 -1px 2px rgba(0, 0, 0, 0.3);
}

.freq-text {
  font-size: 12px;
  letter-spacing: 3px;
  color: var(--accent);
  font-weight: 600;
}

/* 封面区域 */
.cover-area {
  display: flex;
  justify-content: center;
  margin-bottom: 24px;
}

.cover-frame {
  position: relative;
  width: 170px;
  height: 170px;
  border-radius: 50%;
  padding: 8px;
  background: linear-gradient(145deg, rgba(0, 0, 0, 0.2) 0%, rgba(0, 0, 0, 0.1) 100%);
  box-shadow:
    0 8px 20px rgba(0, 0, 0, 0.3),
    inset 0 2px 4px rgba(255, 255, 255, 0.4),
    inset 0 -2px 4px rgba(0, 0, 0, 0.2);
  overflow: hidden;
}

.cover-frame img,
.cover-placeholder {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  object-fit: cover;
  background: linear-gradient(145deg, var(--bg-mid) 0%, var(--bg-deep) 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 64px;
  animation: spin 24s linear infinite;
}

.cover-mask {
  position: absolute;
  inset: 8px;
  border-radius: 50%;
  pointer-events: none;
  background:
    radial-gradient(circle at 30% 30%, transparent 40%, var(--overlay-tint) 100%);
  box-shadow:
    inset 0 0 20px var(--overlay-tint),
    inset 0 0 40px rgba(0, 0, 0, 0.15);
  z-index: 2;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* 信息区域 */
.info-area {
  text-align: center;
}

.station-name {
  margin: 0 0 12px;
  font-size: 22px;
  font-weight: 700;
  color: var(--text-main);
  letter-spacing: 1px;
  text-shadow: 0 1px 1px rgba(255, 255, 255, 0.4);
}

.station-meta {
  display: flex;
  justify-content: center;
  gap: 10px;
  margin: 0;
}

.meta-chip {
  padding: 5px 12px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-sub);
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.35) 0%, rgba(0, 0, 0, 0.5) 100%);
  box-shadow:
    inset 0 1px 1px rgba(255, 255, 255, 0.15),
    0 2px 4px rgba(0, 0, 0, 0.2);
}

.meta-chip.secondary {
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.25) 0%, rgba(0, 0, 0, 0.4) 100%);
}

/* 底部控制面板 */
.control-deck {
  flex-shrink: 0;
  padding: 16px 24px calc(16px + env(safe-area-inset-bottom));
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.05) 0%, rgba(0, 0, 0, 0.3) 100%);
  border-top: 1px solid rgba(255, 255, 255, 0.06);
}

.knobs-row {
  display: flex;
  justify-content: center;
  align-items: flex-start;
  gap: 48px;
  margin-bottom: 28px;
}

.knob-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}

/* 物理旋钮 */
.physical-knob {
  position: relative;
  width: 90px;
  height: 90px;
  border-radius: 50%;
  border: none;
  background: transparent;
  cursor: pointer;
  padding: 0;
  -webkit-tap-highlight-color: transparent;
}

.knob-top {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: linear-gradient(145deg, var(--btn-bg) 0%, var(--btn-bg2) 100%);
  box-shadow:
    0 8px 0 var(--btn-shadow),
    0 12px 20px rgba(0, 0, 0, 0.45),
    inset 0 2px 3px rgba(255, 255, 255, 0.4),
    inset 0 -2px 3px rgba(0, 0, 0, 0.2);
  z-index: 2;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.physical-knob:active .knob-top {
  transform: translateY(8px);
  box-shadow:
    0 0 0 var(--btn-shadow),
    0 4px 8px rgba(0, 0, 0, 0.45),
    inset 0 2px 3px rgba(255, 255, 255, 0.3),
    inset 0 -2px 3px rgba(0, 0, 0, 0.3);
}

.knob-side {
  position: absolute;
  inset: 3px;
  border-radius: 50%;
  background:
    repeating-conic-gradient(
      from 0deg,
      var(--btn-shadow) 0deg 4deg,
      var(--btn-bg2) 4deg 8deg
    );
  z-index: 1;
}

.knob-marker {
  position: absolute;
  top: 14px;
  left: 50%;
  width: 4px;
  height: 16px;
  margin-left: -2px;
  background: var(--text-main);
  border-radius: 2px;
  box-shadow: inset 0 1px 1px rgba(0, 0, 0, 0.3);
}

.power-knob.playing .knob-top {
  background: linear-gradient(145deg, var(--led-on) 0%, color-mix(in srgb, var(--led-on) 70%, black) 100%);
}

.knob-label {
  font-size: 12px;
  color: var(--text-muted);
  letter-spacing: 2px;
}

/* 文字控制 */
.text-controls {
  display: flex;
  justify-content: center;
  gap: 32px;
}

.text-btn {
  background: transparent;
  border: none;
  color: var(--text-muted);
  font-size: 13px;
  letter-spacing: 2px;
  cursor: pointer;
  padding: 6px 4px;
  transition: color 0.2s ease;
  user-select: none;
}

.text-btn:hover,
.text-btn.active {
  color: var(--accent);
}

.error-hint {
  text-align: center;
  margin: 14px 0 0;
  font-size: 12px;
  color: #ff8a8a;
  min-height: 18px;
}

/* 弹窗 */
.modal-overlay {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  background: rgba(0, 0, 0, 0.65);
  z-index: 100;
  padding-bottom: env(safe-area-inset-bottom);
}

.retro-panel {
  width: 100%;
  max-width: 420px;
  background: linear-gradient(145deg, var(--card-bg) 0%, var(--card-bg2) 100%);
  border-radius: 24px 24px 0 0;
  padding: 24px;
  box-shadow:
    0 -8px 24px rgba(0, 0, 0, 0.4),
    inset 0 2px 4px rgba(255, 255, 255, 0.5);
}

.modal-title {
  margin: 0 0 18px;
  text-align: center;
  font-size: 18px;
  color: var(--text-main);
  letter-spacing: 2px;
}

.category-list,
.timer-list,
.skin-list {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

.category-item,
.timer-item,
.skin-item {
  padding: 14px 8px;
  border: none;
  border-radius: 14px;
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.15) 0%, rgba(0, 0, 0, 0.25) 100%);
  color: var(--text-main);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  box-shadow:
    0 4px 0 rgba(0, 0, 0, 0.25),
    0 5px 8px rgba(0, 0, 0, 0.2),
    inset 0 1px 1px rgba(255, 255, 255, 0.3);
  transition: all 0.1s ease;
}

.category-item:active,
.timer-item:active,
.skin-item:active {
  transform: translateY(4px);
  box-shadow:
    0 0 0 rgba(0, 0, 0, 0.25),
    0 1px 2px rgba(0, 0, 0, 0.2);
}

.category-item.active,
.timer-item.cancel,
.skin-item.active {
  background: linear-gradient(180deg, var(--accent) 0%, color-mix(in srgb, var(--accent) 70%, black) 100%);
  color: var(--bg-deep);
  box-shadow:
    0 4px 0 color-mix(in srgb, var(--accent) 50%, black),
    0 5px 8px rgba(0, 0, 0, 0.2);
}

.timer-list {
  grid-template-columns: repeat(2, 1fr);
}

@media (min-width: 480px) {
  .station-card {
    max-width: 400px;
    padding: 30px 26px 34px;
  }

  .cover-frame {
    width: 190px;
    height: 190px;
  }

  .station-name {
    font-size: 26px;
  }

  .physical-knob {
    width: 100px;
    height: 100px;
  }
}
</style>
