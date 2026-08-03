const axios = require('axios');
const cheerio = require('cheerio');

const BASE_URL = 'https://tingfm.net';
const API_URL = 'https://tingfm.net/wp-json';

const HEADERS = {
    'User-Agent':
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
};

/**
 * 从 HTML 中解析电台列表
 */
function parseRadioList(html) {
    const $ = cheerio.load(html);
    const results = [];

    $('.post-list.radio-list').each((i, el) => {
        const linkEl = $(el).find('h3 a');
        const title = linkEl.text().trim();
        const href = linkEl.attr('href') || '';
        const match = href.match(/\/radio\/(\d+)$/);
        if (!match) return;

        const id = match[1];
        const listensText = $(el).find('.is-pulled-right').text().trim();

        results.push({
            id,
            title,
            artist: listensText || 'TingFM 电台',
        });
    });

    return results;
}

/**
 * 从电台详情页解析元数据
 */
function parseStationDetail(html) {
    const $ = cheerio.load(html);
    const title = $('h1.title').text().trim();
    const logo = $('.station-logo img').attr('src');
    const genre = $('.media-meta a[href^="https://tingfm.net/genre/"]').text().trim();
    const region = $('.media-meta a[href^="https://tingfm.net/region/"]').text().trim();

    return {
        title,
        artwork: logo,
        album: region || genre || 'TingFM',
        artist: region && genre ? `${region} · ${genre}` : region || genre || 'TingFM',
    };
}

module.exports = {
    platform: 'TingFM',
    version: '0.0.1',
    author: 'musicfree-skills',
    description: '听FM 在线广播电台 - 在线收听全国广播电台',
    cacheControl: 'no-cache',
    supportedSearchType: ['music'],

    /**
     * 搜索电台
     */
    async search(query, page, type) {
        if (type !== 'music') {
            return { isEnd: true, data: [] };
        }

        const pageParam = page > 1 ? `&paged=${page}` : '';
        const url = `${BASE_URL}/?s=${encodeURIComponent(query)}${pageParam}`;
        const res = await axios.get(url, { headers: HEADERS });
        const results = parseRadioList(res.data);

        return {
            isEnd: results.length === 0,
            data: results,
        };
    },

    /**
     * 获取电台直播流地址
     */
    async getMediaSource(musicItem, quality) {
        const res = await axios.get(`${API_URL}/query/wndt_streams`, {
            params: { post_id: musicItem.id, in_web: 'true' },
            headers: HEADERS,
        });

        const data = res.data;
        if (
            data.status !== 1 ||
            !data.data ||
            !data.data.streams ||
            data.data.streams.length === 0
        ) {
            throw new Error('无法获取播放链接');
        }

        const stream = data.data.streams[0];
        return {
            url: stream.url,
            headers: { Referer: BASE_URL },
        };
    },

    /**
     * 补全电台信息（封面、地区、分类等）
     */
    async getMusicInfo(musicItem) {
        const res = await axios.get(`${BASE_URL}/radio/${musicItem.id}`, {
            headers: HEADERS,
        });
        return parseStationDetail(res.data);
    },

    /**
     * 通过 URL 导入单个电台
     */
    async importMusicItem(urlLike) {
        const match = urlLike.match(/\/radio\/(\d+)(?:\/|$)/);
        if (!match) {
            throw new Error('无法识别的电台链接格式');
        }
        const id = match[1];
        const res = await axios.get(`${BASE_URL}/radio/${id}`, {
            headers: HEADERS,
        });
        const detail = parseStationDetail(res.data);
        return {
            id,
            title: detail.title,
            artist: detail.artist,
            artwork: detail.artwork,
        };
    },

    /**
     * 排行榜/分类列表
     */
    async getTopLists() {
        return [
            {
                title: '电台分类',
                data: [
                    {
                        id: 'all',
                        title: '全部电台',
                        artwork: 'https://tingfm.net/wp-content/themes/radio-hub/static/images/favicon-64.png',
                    },
                ],
            },
        ];
    },

    /**
     * 排行榜详情
     */
    async getTopListDetail(topListItem, page) {
        const pagePath = page > 1 ? `/page/${page}` : '';
        const url = `${BASE_URL}/radio${pagePath}`;
        const res = await axios.get(url, { headers: HEADERS });
        const results = parseRadioList(res.data);

        return {
            isEnd: results.length === 0,
            musicList: results,
        };
    },
};
