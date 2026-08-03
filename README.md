# TingFM Radio

基于 Tauri 2 + Vue 3 的跨平台网络电台播放器，支持自定义 JSON 电台目录。

## 功能

- 播放 HLS / 直连音频流
- 电台搜索、分类筛选、收藏
- 本地目录管理与远程 URL 更新
- 本地流代理：自动处理 CORS / Referer 限制

## 目录格式

`public/radio-catalog.json`：

```json
{
  "version": "1.0.0",
  "platform": "TingFM",
  "updatedAt": "2026-08-03T12:00:00Z",
  "sources": [
    {
      "id": "tingfm_123",
      "name": "电台名称",
      "logo": "https://...",
      "category": "分类",
      "region": "地区",
      "description": "描述",
      "streams": [
        {
          "url": "https://.../stream.m3u8",
          "format": "hls"
        }
      ]
    }
  ]
}
```

## 开发

```bash
npm install
npm run tauri dev
```

## 从 TingFM 生成目录

```bash
node tools/generate-tingfm-catalog.cjs
```

> 注意：频繁请求可能导致 TingFM 限流（429），请适当调整脚本中的延迟。

## 构建

```bash
npm run tauri build
```
