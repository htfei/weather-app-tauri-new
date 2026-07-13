<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface ScriptItem {
  id: string
  name: string
  namespace: string
  version: string
  homepage: string
  icon: string
  update_url: string
  matches: string[]
  includes: string[]
  excludes: string[]
  run_at: string
  requires: string[]
  grants: string[]
  enabled: boolean
}

const searchQuery = ref('')
const scriptUrl = ref('')
const currentView = ref<'home' | 'settings'>('home')
const scripts = ref<ScriptItem[]>([])
const installError = ref('')
const installSuccess = ref('')

const bookmarks = [
  { name: '掘金', url: 'https://juejin.cn/', color: '#007FFF' },
  { name: 'V2EX', url: 'https://www.v2ex.com/', color: '#7C5AFC' },
  { name: 'GitHub', url: 'https://github.com/', color: '#171515' },
  { name: '知乎', url: 'https://www.zhihu.com/', color: '#0084FF' },
  { name: '微博', url: 'https://weibo.com/', color: '#E6162D' },
  { name: '豆瓣', url: 'https://www.douban.com/', color: '#00B51D' },
  { name: 'CSDN', url: 'https://www.csdn.net/', color: '#FF6A00' },
  { name: 'B站', url: 'https://www.bilibili.com/', color: '#FB7299' },
  { name: '新闻', url: 'https://newsnow.busiyi.world/', color: '#00C4B6' },
  { name: '设置', url: '#settings', color: '#6B7280' },
]

onMounted(async () => {
  await loadScripts()
  checkHash()
  window.addEventListener('hashchange', checkHash)
})

const checkHash = () => {
  if (window.location.hash === '#settings') {
    currentView.value = 'settings'
  }
}

const loadScripts = async () => {
  try {
    const list = await invoke<ScriptItem[]>('list_scripts')
    scripts.value = list
  } catch (e) {
    console.error('加载脚本失败:', e)
  }
}

const navigate = async (url: string) => {
  try {
    await invoke('navigate_to_url', { url })
  } catch (e) {
    console.error('导航失败:', e)
    window.location.href = url
  }
}

const handleSearch = () => {
  const query = searchQuery.value.trim()
  if (!query) return

  let url = query
  if (!url.startsWith('http')) {
    if (url.includes('.') && !url.includes(' ')) {
      url = 'https://' + url
    } else {
      url = 'https://www.baidu.com/s?wd=' + encodeURIComponent(url)
    }
  }
  navigate(url)
}

const handleBookmarkClick = (item: typeof bookmarks[0]) => {
  if (item.url === '#settings') {
    currentView.value = 'settings'
  } else {
    navigate(item.url)
  }
}

const goHome = () => {
  currentView.value = 'home'
  installError.value = ''
  installSuccess.value = ''
}

const toggleScript = async (script: ScriptItem) => {
  try {
    await invoke('toggle_script', { id: script.id, enabled: !script.enabled })
    script.enabled = !script.enabled
  } catch (e) {
    console.error('切换脚本失败:', e)
  }
}

const deleteScript = async (script: ScriptItem) => {
  try {
    await invoke('delete_script', { id: script.id })
    await loadScripts()
  } catch (e) {
    console.error('删除脚本失败:', e)
  }
}

const installFromUrl = async () => {
  const url = scriptUrl.value.trim()
  if (!url) return

  installError.value = ''
  installSuccess.value = ''

  try {
    await invoke('install_script_from_url', { url })
    installSuccess.value = '脚本安装成功'
    scriptUrl.value = ''
    await loadScripts()
  } catch (e) {
    installError.value = '安装失败: ' + String(e)
  }
}

const applicableSites = (script: ScriptItem) => {
  const sites: string[] = []
  script.matches.forEach((m) => {
    if (m.startsWith('http')) {
      try {
        const url = new URL(m.replace(/\*/g, ''))
        sites.push(url.origin + '/')
      } catch {
        sites.push(m)
      }
    } else if (m.includes('://')) {
      sites.push(m)
    }
  })
  return [...new Set(sites)]
}

const openUrl = (url: string) => {
  if (!url) return
  let target = url
  if (!target.startsWith('http')) {
    target = 'https://' + target
  }
  navigate(target)
}
</script>

<template>
  <div class="app">
    <div v-if="currentView === 'home'" class="home">
      <div class="logo-section">
        <div class="logo">
          <span class="logo-icon">🌐</span>
          <span class="logo-text">WebWrapper</span>
        </div>
      </div>

      <div class="search-section">
        <div class="search-box">
          <span class="search-icon">🔍</span>
          <input
            v-model="searchQuery"
            type="text"
            placeholder="输入网址或搜索内容..."
            @keydown.enter="handleSearch"
            class="search-input"
          />
          <button @click="handleSearch" class="search-btn">搜索</button>
        </div>
      </div>

      <div class="bookmarks-section">
        <div class="grid">
          <div
            v-for="item in bookmarks"
            :key="item.name"
            @click="handleBookmarkClick(item)"
            class="bookmark-item"
          >
            <div class="bookmark-icon" :style="{ backgroundColor: item.color }">
              <span>{{ item.name.charAt(0) }}</span>
            </div>
            <span class="bookmark-name">{{ item.name }}</span>
          </div>
        </div>
      </div>

      <div class="footer">
        <span>支持油猴脚本 | 自定义浏览器</span>
      </div>
    </div>

    <div v-else class="settings">
      <div class="settings-header">
        <button @click="goHome" class="back-btn">←</button>
        <h2>设置</h2>
        <div class="spacer"></div>
      </div>

      <div class="settings-content">
        <div class="section">
          <h3>从 URL 安装脚本</h3>
          <div class="install-box">
            <input
              v-model="scriptUrl"
              type="text"
              placeholder="输入 .user.js 脚本 URL..."
              @keydown.enter="installFromUrl"
              class="install-input"
            />
            <button @click="installFromUrl" class="install-btn">安装</button>
          </div>
          <p v-if="installError" class="message error">{{ installError }}</p>
          <p v-if="installSuccess" class="message success">{{ installSuccess }}</p>
        </div>

        <div class="section">
          <h3>脚本管理</h3>
          <div v-if="scripts.length === 0" class="empty-state">
            <span>暂无脚本，可从上方 URL 安装或访问网站自动匹配已安装脚本</span>
          </div>
          <div v-else class="script-list">
            <div
              v-for="script in scripts"
              :key="script.id"
              class="script-item"
            >
              <div class="script-info">
                <img
                  v-if="script.icon"
                  :src="script.icon"
                  class="script-icon"
                  alt="icon"
                  @error="($event.target as HTMLImageElement).style.display='none'"
                />
                <div v-else class="script-icon script-icon-fallback">📜</div>
                <div class="script-detail">
                  <div class="script-title">
                    <span class="script-name">{{ script.name }}</span>
                    <span class="script-version">v{{ script.version }}</span>
                  </div>
                  <div class="script-links">
                    <a
                      v-if="script.homepage"
                      :href="script.homepage"
                      target="_blank"
                      class="script-link"
                      @click.prevent="openUrl(script.homepage)"
                    >脚本主页</a>
                    <span v-if="applicableSites(script).length" class="script-sites">
                      适用站点:
                      <a
                        v-for="site in applicableSites(script).slice(0, 3)"
                        :key="site"
                        class="script-link"
                        @click.prevent="openUrl(site)"
                      >{{ site }}</a>
                    </span>
                  </div>
                </div>
              </div>
              <div class="script-actions">
                <label class="toggle">
                  <input type="checkbox" :checked="script.enabled" @change="toggleScript(script)" />
                  <span class="slider"></span>
                </label>
                <button class="delete-btn" @click="deleteScript(script)">🗑</button>
              </div>
            </div>
          </div>
        </div>

        <div class="section">
          <h3>关于</h3>
          <div class="about-info">
            <p>WebWrapper v1.0.0</p>
            <p>基于 Tauri 2 构建</p>
            <p>支持油猴脚本注入</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.app {
  min-height: 100vh;
  min-height: 100dvh;
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
  color: #fff;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left);
}

.home {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 60px 20px;
  padding-top: max(60px, env(safe-area-inset-top));
}

.logo-section {
  margin-bottom: 40px;
}

.logo {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 28px;
  font-weight: 700;
}

.logo-icon {
  font-size: 36px;
}

.logo-text {
  background: linear-gradient(90deg, #00d4ff, #7c5afc);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.search-section {
  width: 100%;
  max-width: 600px;
  margin-bottom: 40px;
}

.search-box {
  display: flex;
  align-items: center;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  padding: 8px 16px;
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.search-icon {
  font-size: 18px;
  margin-right: 12px;
}

.search-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: #fff;
  font-size: 16px;
}

.search-input::placeholder {
  color: rgba(255, 255, 255, 0.6);
}

.search-btn {
  background: linear-gradient(90deg, #00d4ff, #7c5afc);
  border: none;
  color: #fff;
  padding: 8px 20px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
}

.bookmarks-section {
  width: 100%;
  max-width: 600px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 20px;
}

.bookmark-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  cursor: pointer;
  padding: 12px;
  border-radius: 12px;
  transition: all 0.2s ease;
}

.bookmark-item:hover {
  background: rgba(255, 255, 255, 0.1);
  transform: scale(1.05);
}

.bookmark-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 20px;
  font-weight: 600;
  margin-bottom: 8px;
}

.bookmark-name {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.8);
  text-align: center;
}

.footer {
  margin-top: 60px;
  color: rgba(255, 255, 255, 0.5);
  font-size: 12px;
}

.settings {
  min-height: 100vh;
  min-height: 100dvh;
  padding-top: env(safe-area-inset-top);
}

.settings-header {
  display: flex;
  align-items: center;
  padding: 16px 24px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.back-btn {
  width: 36px;
  height: 36px;
  border: none;
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
  border-radius: 8px;
  cursor: pointer;
  font-size: 18px;
}

.settings-header h2 {
  flex: 1;
  text-align: center;
  margin: 0;
  font-size: 18px;
}

.spacer {
  width: 36px;
}

.settings-content {
  padding: 24px;
}

.section {
  margin-bottom: 32px;
}

.section h3 {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.5);
  margin-bottom: 16px;
}

.install-box {
  display: flex;
  gap: 12px;
}

.install-input {
  flex: 1;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 8px;
  padding: 12px 16px;
  color: #fff;
  font-size: 14px;
  outline: none;
}

.install-input::placeholder {
  color: rgba(255, 255, 255, 0.5);
}

.install-btn {
  background: linear-gradient(90deg, #00d4ff, #7c5afc);
  border: none;
  color: #fff;
  padding: 12px 24px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
}

.message {
  margin-top: 8px;
  font-size: 13px;
}

.message.error {
  color: #ff6b6b;
}

.message.success {
  color: #51cf66;
}

.empty-state {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 12px;
  padding: 24px;
  text-align: center;
  color: rgba(255, 255, 255, 0.5);
  font-size: 14px;
}

.script-list {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 12px;
  overflow: hidden;
}

.script-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.script-item:last-child {
  border-bottom: none;
}

.script-info {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.script-icon {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  flex-shrink: 0;
  object-fit: contain;
  background: rgba(255, 255, 255, 0.08);
}

.script-icon-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
}

.script-detail {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 4px;
}

.script-title {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.script-name {
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.script-version {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
  flex-shrink: 0;
}

.script-links {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  min-width: 0;
}

.script-sites {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.script-link {
  font-size: 12px;
  color: #00d4ff;
  text-decoration: none;
  cursor: pointer;
}

.script-link:hover {
  text-decoration: underline;
}

.script-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.delete-btn {
  background: transparent;
  border: none;
  color: #ff6b6b;
  cursor: pointer;
  font-size: 18px;
  padding: 4px;
}

.toggle {
  position: relative;
  width: 48px;
  height: 24px;
}

.toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(255, 255, 255, 0.2);
  border-radius: 24px;
  transition: 0.3s;
}

.slider:before {
  position: absolute;
  content: '';
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background: #fff;
  border-radius: 50%;
  transition: 0.3s;
}

input:checked + .slider {
  background: linear-gradient(90deg, #00d4ff, #7c5afc);
}

input:checked + .slider:before {
  transform: translateX(24px);
}

.about-info {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 12px;
  padding: 16px;
}

.about-info p {
  margin: 8px 0;
  font-size: 14px;
  color: rgba(255, 255, 255, 0.7);
}

@media (max-width: 480px) {
  .grid {
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
  }

  .home {
    padding: 40px 16px;
  }

  .install-box {
    flex-direction: column;
  }
}
</style>
