/**
 * 基于 tingfm-plugin.js 生成电台目录 JSON
 *
 * 用法：
 *   node tools/generate-tingfm-catalog.js
 *
 * 输出：
 *   public/radio-catalog.json
 */

const fs = require('fs');
const path = require('path');

const pluginPath = path.resolve(__dirname, './tingfm-plugin.cjs');
const plugin = require(pluginPath);

const OUTPUT_PATH = path.resolve(__dirname, '../public/radio-catalog.json');
const MAX_SOURCES = 60;

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

function extractRegion(artistText) {
    if (!artistText) return 'TingFM';
    const parts = artistText.split('·').map((s) => s.trim());
    return parts[parts.length - 1] || 'TingFM';
}

async function collectSources() {
    const allSources = [];
    const seenIds = new Set();

    console.log('开始获取 TingFM 电台列表...');

    const topListItem = { id: 'all', title: '全部电台' };
    let page = 1;
    let isEnd = false;

    while (!isEnd && allSources.length < MAX_SOURCES) {
        console.log(`  获取第 ${page} 页...`);
        const detail = await plugin.getTopListDetail(topListItem, page);
        const list = detail.musicList || [];

        if (!list.length) {
            isEnd = true;
            break;
        }

        for (const item of list) {
            if (seenIds.has(item.id)) continue;
            seenIds.add(item.id);

            try {
                await sleep(800);
                const info = await plugin.getMusicInfo(item).catch(() => ({
                    title: item.title,
                    artwork: item.artwork,
                    album: item.album || 'TingFM',
                    artist: item.artist || 'TingFM',
                }));
                await sleep(800);

                const media = await plugin.getMediaSource(item, 'standard');
                await sleep(800);

                const url = media?.url;
                if (!url) {
                    console.warn(`    [跳过] ${item.title}: 无播放地址`);
                    continue;
                }

                const format = url.toLowerCase().includes('.m3u8') ? 'hls' : 'direct';

                const artistText = info.artist || item.artist || 'TingFM';

                allSources.push({
                    id: `tingfm_${item.id}`,
                    name: info.title || item.title,
                    logo: info.artwork || item.artwork || '',
                    category: info.album || 'TingFM',
                    region: extractRegion(artistText),
                    description: `${artistText} - TingFM 在线广播`,
                    streams: [
                        {
                            url,
                            format,
                            quality: 'standard',
                        },
                    ],
                });

                console.log(`    [成功] ${info.title || item.title}`);
            } catch (e) {
                console.warn(`    [失败] ${item.title}: ${e.message}`);
                if (e.message && e.message.includes('429')) {
                    console.log('    触发限流，等待 5 秒后继续...');
                    await sleep(5000);
                }
            }

            if (allSources.length >= MAX_SOURCES) break;
        }

        isEnd = detail.isEnd || list.length === 0;
        page++;
        if (!isEnd) await sleep(1000);
    }

    console.log(`共收集 ${allSources.length} 个电台`);
    return allSources;
}

async function main() {
    try {
        const sources = await collectSources();

        const catalog = {
            version: plugin.version || '1.0.0',
            platform: plugin.platform || 'TingFM',
            updatedAt: new Date().toISOString(),
            sources,
        };

        fs.writeFileSync(OUTPUT_PATH, JSON.stringify(catalog, null, 2), 'utf-8');
        console.log(`\n目录已保存到: ${OUTPUT_PATH}`);
    } catch (e) {
        console.error('生成目录失败:', e.message);
        process.exit(1);
    }
}

main();
