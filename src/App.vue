<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface WeatherData {
  location: string
  temperature: number
  description: string
  humidity: number
  wind_speed: number
  feels_like: number
}

const city = ref('')
const weather = ref<WeatherData | null>(null)
const loading = ref(false)
const error = ref('')

const getWeather = async () => {
  if (!city.value.trim()) {
    error.value = '请输入城市名称'
    return
  }
  
  loading.value = true
  error.value = ''
  
  try {
    weather.value = await invoke<WeatherData>('get_weather', { city: city.value })
  } catch (e) {
    error.value = e instanceof Error ? e.message : '获取天气失败'
    weather.value = null
  } finally {
    loading.value = false
  }
}

const getCurrentWeather = async () => {
  loading.value = true
  error.value = ''
  
  try {
    weather.value = await invoke<WeatherData>('get_current_weather')
    if (weather.value) {
      city.value = weather.value.location
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : '获取当前位置天气失败'
    weather.value = null
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  getCurrentWeather()
})
</script>

<template>
  <div class="weather-container">
    <div class="header">
      <h1>🌤️ 天气查询</h1>
      <p class="subtitle">获取实时天气信息</p>
    </div>
    
    <div class="search-box">
      <input
        v-model="city"
        type="text"
        placeholder="输入城市名称（如：北京、上海）"
        @keyup.enter="getWeather"
        :disabled="loading"
      />
      <button @click="getWeather" :disabled="loading">
        <span v-if="loading">查询中...</span>
        <span v-else>🔍 查询</span>
      </button>
    </div>
    
    <div v-if="error" class="error-message">
      {{ error }}
    </div>
    
    <div v-if="weather" class="weather-card">
      <div class="weather-header">
        <h2>{{ weather.location }}</h2>
        <span class="weather-icon">
          {{ weather.temperature > 25 ? '☀️' : weather.temperature > 15 ? '⛅' : '❄️' }}
        </span>
      </div>
      
      <div class="temperature">
        {{ weather.temperature }}°C
      </div>
      
      <div class="description">
        {{ weather.description }}
      </div>
      
      <div class="weather-details">
        <div class="detail-item">
          <span class="icon">💧</span>
          <span class="label">湿度</span>
          <span class="value">{{ weather.humidity }}%</span>
        </div>
        <div class="detail-item">
          <span class="icon">💨</span>
          <span class="label">风速</span>
          <span class="value">{{ weather.wind_speed }} km/h</span>
        </div>
        <div class="detail-item">
          <span class="icon">🌡️</span>
          <span class="label">体感温度</span>
          <span class="value">{{ weather.feels_like }}°C</span>
        </div>
      </div>
    </div>
    
    <div v-if="!weather && !loading && !error" class="empty-state">
      <p>👋 欢迎使用天气查询</p>
      <p>请输入城市名称查询天气</p>
    </div>
  </div>
</template>

<style scoped>
.weather-container {
  background: rgba(255, 255, 255, 0.95);
  border-radius: 20px;
  padding: 30px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.15);
}

.header {
  text-align: center;
  margin-bottom: 30px;
}

.header h1 {
  font-size: 28px;
  color: #333;
  margin-bottom: 8px;
}

.subtitle {
  color: #666;
  font-size: 14px;
}

.search-box {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;
}

.search-box input {
  flex: 1;
  padding: 12px 16px;
  border: 2px solid #e0e0e0;
  border-radius: 10px;
  font-size: 16px;
  outline: none;
  transition: border-color 0.3s;
}

.search-box input:focus {
  border-color: #667eea;
}

.search-box input:disabled {
  background: #f5f5f5;
}

.search-box button {
  padding: 12px 24px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border: none;
  border-radius: 10px;
  font-size: 16px;
  cursor: pointer;
  transition: transform 0.2s, opacity 0.2s;
}

.search-box button:hover:not(:disabled) {
  transform: translateY(-2px);
}

.search-box button:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.error-message {
  background: #ffebee;
  color: #c62828;
  padding: 12px;
  border-radius: 8px;
  margin-bottom: 20px;
  text-align: center;
}

.weather-card {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 16px;
  padding: 25px;
  color: white;
}

.weather-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.weather-header h2 {
  font-size: 24px;
  font-weight: 600;
}

.weather-icon {
  font-size: 40px;
}

.temperature {
  font-size: 56px;
  font-weight: 300;
  margin-bottom: 8px;
}

.description {
  font-size: 18px;
  opacity: 0.9;
  margin-bottom: 25px;
}

.weather-details {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 15px;
}

.detail-item {
  text-align: center;
  background: rgba(255, 255, 255, 0.15);
  padding: 12px 8px;
  border-radius: 10px;
}

.detail-item .icon {
  font-size: 24px;
  display: block;
  margin-bottom: 5px;
}

.detail-item .label {
  font-size: 12px;
  opacity: 0.8;
  display: block;
}

.detail-item .value {
  font-size: 14px;
  font-weight: 600;
  display: block;
  margin-top: 3px;
}

.empty-state {
  text-align: center;
  padding: 40px 20px;
  color: #999;
}

.empty-state p {
  margin-bottom: 8px;
  font-size: 16px;
}
</style>