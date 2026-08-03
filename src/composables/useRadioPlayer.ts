import { ref, readonly } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import Hls from 'hls.js';
import type { RadioSource } from '../types/radio';

export type PlayerState = 'idle' | 'loading' | 'playing' | 'paused' | 'error';

const audio = new Audio();
audio.preload = 'none';

const currentSource = ref<RadioSource | null>(null);
const state = ref<PlayerState>('idle');
const errorMessage = ref('');
const volume = ref(Number(localStorage.getItem('radio-volume') ?? 0.8));
let hlsInstance: Hls | null = null;

audio.volume = volume.value;

function destroyHls() {
  if (hlsInstance) {
    hlsInstance.destroy();
    hlsInstance = null;
  }
}

audio.addEventListener('play', () => {
  state.value = 'playing';
});

audio.addEventListener('pause', () => {
  state.value = 'paused';
});

audio.addEventListener('waiting', () => {
  state.value = 'loading';
});

audio.addEventListener('playing', () => {
  state.value = 'playing';
  errorMessage.value = '';
});

audio.addEventListener('error', () => {
  state.value = 'error';
  const code = audio.error?.code ?? 0;
  const messages: Record<number, string> = {
    1: '播放被中断',
    2: '网络错误，无法加载音频',
    3: '音频解码失败',
    4: '音频格式不受支持',
  };
  errorMessage.value = messages[code] || '播放出错';
});

async function getProxyUrl(streamUrl: string): Promise<string> {
  return await invoke<string>('proxy_stream_url', {
    url: streamUrl,
    headers: { Referer: 'https://tingfm.net/' },
  });
}

function isCorsOrNetworkError(data: any): boolean {
  if (data.type !== Hls.ErrorTypes.NETWORK_ERROR) return false;
  // response.code 为 0 通常表示请求被阻止（CORS / 网络不可达）
  const responseCode = data.response?.code ?? 0;
  return responseCode === 0 || data.details === 'manifestLoadError';
}

export function useRadioPlayer() {
  async function playWithHls(source: RadioSource, streamUrl: string, useProxy: boolean) {
    destroyHls();
    audio.pause();
    audio.src = '';

    let url = streamUrl;
    if (useProxy) {
      try {
        url = await getProxyUrl(streamUrl);
      } catch (e) {
        errorMessage.value = '代理启动失败';
        state.value = 'error';
        return;
      }
    }

    currentSource.value = source;
    state.value = 'loading';
    errorMessage.value = useProxy ? '正在通过代理加载...' : '';

    hlsInstance = new Hls({
      enableWorker: false,
      lowLatencyMode: false,
      backBufferLength: 60,
    });

    let proxyRetried = useProxy;
    let retrying = false;

    const onError = async (_event: string, data: any) => {
      if (retrying || !data.fatal) return;

      if (!proxyRetried && isCorsOrNetworkError(data)) {
        retrying = true;
        proxyRetried = true;
        errorMessage.value = '直连失败，尝试代理...';

        try {
          const proxyUrl = await getProxyUrl(streamUrl);
          hlsInstance?.destroy();
          hlsInstance = new Hls({
            enableWorker: false,
            lowLatencyMode: false,
            backBufferLength: 60,
          });
          hlsInstance.on(Hls.Events.ERROR, onError);
          hlsInstance.loadSource(proxyUrl);
          hlsInstance.attachMedia(audio);
          await audio.play();
          return;
        } catch (e) {
          // 代理也失败，继续显示错误
        }
      }

      switch (data.type) {
        case Hls.ErrorTypes.NETWORK_ERROR:
          errorMessage.value = '网络错误，无法加载直播流';
          break;
        case Hls.ErrorTypes.MEDIA_ERROR:
          errorMessage.value = '媒体解码错误';
          break;
        default:
          errorMessage.value = '直播流播放失败';
      }
      state.value = 'error';
    };

    hlsInstance.on(Hls.Events.ERROR, onError);
    hlsInstance.loadSource(url);
    hlsInstance.attachMedia(audio);
    audio.play().catch((e) => {
      errorMessage.value = e.message || '播放启动失败';
      state.value = 'error';
    });
  }

  function playDirect(source: RadioSource, streamUrl: string) {
    destroyHls();
    audio.src = streamUrl;
    currentSource.value = source;
    state.value = 'loading';
    errorMessage.value = '';
    audio.play().catch((e) => {
      errorMessage.value = e.message || '播放启动失败';
      state.value = 'error';
    });
  }

  async function play(source: RadioSource, streamIndex = 0) {
    const stream = source.streams[streamIndex];
    if (!stream) {
      errorMessage.value = '没有可用的播放地址';
      state.value = 'error';
      return;
    }

    if (stream.format === 'hls' && Hls.isSupported()) {
      // 优先直连，遇到 CORS/网络错误后自动走代理
      await playWithHls(source, stream.url, false);
    } else {
      playDirect(source, stream.url);
    }
  }

  function togglePlay() {
    if (!currentSource.value) return;
    if (state.value === 'playing') {
      audio.pause();
    } else {
      audio.play().catch(() => {
        // 忽略自动播放策略导致的错误
      });
    }
  }

  function stop() {
    destroyHls();
    audio.pause();
    audio.src = '';
    currentSource.value = null;
    state.value = 'idle';
    errorMessage.value = '';
  }

  function setVolume(value: number) {
    const clamped = Math.max(0, Math.min(1, value));
    volume.value = clamped;
    audio.volume = clamped;
    localStorage.setItem('radio-volume', String(clamped));
  }

  return {
    audio,
    currentSource: readonly(currentSource),
    state: readonly(state),
    errorMessage: readonly(errorMessage),
    volume: readonly(volume),
    play,
    togglePlay,
    stop,
    setVolume,
  };
}
