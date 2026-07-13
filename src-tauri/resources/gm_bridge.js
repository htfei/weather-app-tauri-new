(function() {
    'use strict';

    // 每个 Webview 上下文只初始化一次桥接层
    if (window.__webwrapper_bridge__) return;

    // Rust 在运行时会替换 token
    var __WEBWRAPPER_TOKEN__ = '{{WEBWRAPPER_TOKEN}}';

    // 在页面脚本有机会篡改之前，先捕获 Tauri 内部 IPC 引用
    var TAURI_INTERNALS = window.__TAURI_INTERNALS__;
    var TAURI_LEGACY = window.__TAURI__;

    // 允许油猴脚本通过桥接调用的 Rust 命令白名单（严格以 gm_ 开头）
    var GM_COMMAND_WHITELIST = [
        'gm_storage_get',
        'gm_storage_set',
        'gm_storage_delete',
        'gm_xhr_proxy'
    ];

    function isAppShell() {
        var host = window.location.host;
        var protocol = window.location.protocol;
        return host === 'tauri.localhost' ||
               host === 'localhost:5173' ||
               host === '127.0.0.1:5173' ||
               protocol === 'tauri:' ||
               window.location.href.indexOf('tauri://') === 0;
    }

    var APP_SHELL = isAppShell();

    function invokeGm(cmd, args) {
        if (GM_COMMAND_WHITELIST.indexOf(cmd) === -1) {
            return Promise.reject(new Error('GM command not allowed: ' + cmd));
        }
        var invoke = TAURI_INTERNALS && TAURI_INTERNALS.invoke;
        if (!invoke && TAURI_LEGACY && TAURI_LEGACY.invoke) {
            invoke = TAURI_LEGACY.invoke;
        }
        if (typeof invoke !== 'function') {
            return Promise.reject(new Error('Tauri invoke not available'));
        }
        return invoke(cmd, args || {});
    }

    function hasGrant(grants, name) {
        if (!grants || grants.length === 0) return false;
        if (grants.indexOf('none') !== -1) return false;
        var alt = name.indexOf('_') !== -1
            ? name.replace(/_/g, '.')
            : name.replace(/\./g, '_');
        return grants.indexOf(name) !== -1 || grants.indexOf(alt) !== -1;
    }

    function buildGmApis(grants, scriptMeta) {
        var apis = {};

        if (hasGrant(grants, 'GM_getValue')) {
            apis.GM_getValue = function(key, defaultValue) {
                return invokeGm('gm_storage_get', { key: key, defaultValue: defaultValue });
            };
        }

        if (hasGrant(grants, 'GM_setValue')) {
            apis.GM_setValue = function(key, value) {
                return invokeGm('gm_storage_set', { key: key, value: value });
            };
        }

        if (hasGrant(grants, 'GM_deleteValue')) {
            apis.GM_deleteValue = function(key) {
                return invokeGm('gm_storage_delete', { key: key });
            };
        }

        if (hasGrant(grants, 'GM_listValues')) {
            apis.GM_listValues = function() {
                return Promise.resolve([]);
            };
        }

        if (hasGrant(grants, 'GM_xmlhttpRequest')) {
            apis.GM_xmlhttpRequest = function(details) {
                return invokeGm('gm_xhr_proxy', {
                    url: details.url,
                    method: details.method || 'GET',
                    headers: details.headers || {},
                    data: details.data || null,
                    responseType: details.responseType || 'text'
                }).then(function(response) {
                    if (typeof details.onload === 'function') details.onload(response);
                    return response;
                }).catch(function(error) {
                    if (typeof details.onerror === 'function') details.onerror(error);
                    throw error;
                });
            };
        }

        if (hasGrant(grants, 'GM_addStyle')) {
            apis.GM_addStyle = function(css) {
                var style = document.createElement('style');
                style.textContent = css;
                style.setAttribute('data-webwrapper-style', 'true');
                var parent = document.head || document.documentElement || document.body;
                if (parent) parent.appendChild(style);
                return style;
            };
        }

        if (hasGrant(grants, 'GM_addElement')) {
            apis.GM_addElement = function(tagName, attributes) {
                var el = document.createElement(tagName);
                if (attributes) {
                    for (var key in attributes) {
                        if (Object.prototype.hasOwnProperty.call(attributes, key)) {
                            el.setAttribute(key, attributes[key]);
                        }
                    }
                }
                var parent = document.head || document.documentElement || document.body;
                if (parent) parent.appendChild(el);
                return el;
            };
        }

        if (hasGrant(grants, 'GM_log')) {
            apis.GM_log = function() {
                console.log.apply(console, arguments);
            };
        }

        // 常用但本地环境下暂以 stub 实现的 API
        apis.GM_registerMenuCommand = function() {};
        apis.GM_unregisterMenuCommand = function() {};
        apis.GM_openInTab = function(url) { window.open(url, '_blank'); return { close: function() {} }; };
        apis.GM_notification = function() {};
        apis.GM_setClipboard = function() {};
        apis.GM_getResourceText = function() { return ''; };
        apis.GM_getResourceURL = function() { return ''; };
        apis.GM_download = function() {};

        var info = {
            script: scriptMeta || { name: 'WebWrapper', version: '1.0.0' },
            scriptHandler: 'WebWrapper',
            version: '1.0.0'
        };
        apis.GM_info = info;
        apis.unsafeWindow = window;

        // GM.* 别名
        apis.GM = { info: info };
        if (apis.GM_getValue) apis.GM.getValue = apis.GM_getValue;
        if (apis.GM_setValue) apis.GM.setValue = apis.GM_setValue;
        if (apis.GM_deleteValue) apis.GM.deleteValue = apis.GM_deleteValue;
        if (apis.GM_listValues) apis.GM.listValues = apis.GM_listValues;
        if (apis.GM_xmlhttpRequest) apis.GM.xmlHttpRequest = apis.GM_xmlhttpRequest;
        if (apis.GM_addStyle) apis.GM.addStyle = apis.GM_addStyle;
        if (apis.GM_addElement) apis.GM.addElement = apis.GM_addElement;
        if (apis.GM_log) apis.GM.log = apis.GM_log;
        apis.GM.registerMenuCommand = apis.GM_registerMenuCommand;
        apis.GM.unregisterMenuCommand = apis.GM_unregisterMenuCommand;
        apis.GM.openInTab = apis.GM_openInTab;
        apis.GM.notification = apis.GM_notification;
        apis.GM.setClipboard = apis.GM_setClipboard;

        return apis;
    }

    function defineGlobal(name, value) {
        try {
            Object.defineProperty(window, name, {
                value: value,
                writable: false,
                configurable: false
            });
        } catch (e) {
            try { window[name] = value; } catch (_) {}
        }
    }

    // 仅暴露无敏感操作的全局辅助对象
    defineGlobal('unsafeWindow', window);
    defineGlobal('GM_info', {
        script: { name: 'WebWrapper', version: '1.0.0' },
        scriptHandler: 'WebWrapper',
        version: '1.0.0'
    });

    var executedScripts = {};

    function loadRequires(requires, callback) {
        if (!requires || requires.length === 0) {
            callback();
            return;
        }
        var loaded = 0;
        var done = function() {
            loaded++;
            if (loaded >= requires.length) callback();
        };

        function loadOne(src, attempt) {
            attempt = attempt || 1;
            var s = document.createElement('script');
            s.src = src;
            s.onload = function() {
                console.log('[WebWrapper] 依赖加载成功:', src);
                done();
            };
            s.onerror = function() {
                console.warn('[WebWrapper] 依赖加载失败 (尝试 ' + attempt + '):', src);
                if (attempt < 3) {
                    setTimeout(function() { loadOne(src, attempt + 1); }, 1000 * attempt);
                } else {
                    console.error('[WebWrapper] 依赖最终加载失败:', src);
                    done();
                }
            };
            s.onabort = s.onerror;

            // 5 秒超时重试
            var timer = setTimeout(function() {
                console.warn('[WebWrapper] 依赖加载超时 (尝试 ' + attempt + '):', src);
                s.src = '';
                if (s.parentNode) s.parentNode.removeChild(s);
                if (attempt < 3) {
                    loadOne(src, attempt + 1);
                } else {
                    console.error('[WebWrapper] 依赖最终加载超时:', src);
                    done();
                }
            }, 5000);

            s.onload = function() {
                clearTimeout(timer);
                console.log('[WebWrapper] 依赖加载成功:', src);
                done();
            };

            var parent = document.head || document.documentElement;
            if (parent) parent.appendChild(s);
        }

        requires.forEach(function(src) { loadOne(src, 1); });
    }

    function executeScript(token, config) {
        if (token !== __WEBWRAPPER_TOKEN__) {
            console.warn('[WebWrapper] 拒绝执行脚本：token 不匹配');
            return;
        }

        var id = config.id || 'unknown';
        if (executedScripts[id]) return;
        executedScripts[id] = true;

        var runAt = config.runAt || 'document-end';
        var code = config.code || '';
        var requires = config.requires || [];
        var grants = config.grants || [];
        var name = config.name || '未命名脚本';
        var scriptMeta = { name: name, version: config.version || '1.0' };

        var run = function() {
            loadRequires(requires, function() {
                var apis = buildGmApis(grants, scriptMeta);
                var apiKeys = Object.keys(apis);
                var apiValues = apiKeys.map(function(k) { return apis[k]; });

                var escapedName = name.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
                var wrappedCode =
                    '(function() {\n' +
                    '  "use strict";\n' +
                    '  try {\n' +
                    '    /* @grant: ' + (grants.join(', ') || 'none') + ' */\n' +
                    code + '\n' +
                    '  } catch(e) {\n' +
                    '    console.error("[WebWrapper] 脚本执行错误 [' + escapedName + ']:", e);\n' +
                    '  }\n' +
                    '})();';

                try {
                    var args = apiKeys.concat([wrappedCode]);
                    var sandbox = Function.prototype.constructor.apply(null, args);
                    sandbox.apply(null, apiValues);
                } catch (e) {
                    console.error('[WebWrapper] 脚本包装失败 [' + name + ']:', e);
                }
            });
        };

        if (runAt === 'document-start') {
            run();
        } else if (runAt === 'document-idle') {
            if (document.readyState === 'complete') {
                setTimeout(run, 0);
            } else {
                window.addEventListener('load', function() { setTimeout(run, 0); });
            }
        } else {
            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', run);
            } else {
                run();
            }
        }
    }

    function executeScripts(token, configs) {
        if (token !== __WEBWRAPPER_TOKEN__) return;
        if (!configs || !configs.length) return;
        configs.forEach(function(c) { executeScript(token, c); });
    }

    window.__webwrapper_bridge__ = {
        executeScript: executeScript,
        executeScripts: executeScripts
    };

    // 拦截 target="_blank" 链接，在当前 Webview 中打开
    document.addEventListener('click', function(e) {
        var t = e.target;
        while (t && t.nodeType === 1 && t.tagName !== 'A') t = t.parentNode;
        if (t && t.tagName === 'A') {
            var href = t.getAttribute('href');
            var target = t.getAttribute('target');
            if (href && href !== '#' && href.indexOf('javascript:') !== 0) {
                if (target === '_blank' || target === '_new') {
                    e.preventDefault();
                    if (href.indexOf('http') !== 0) {
                        href = new URL(href, window.location.origin + window.location.pathname).href;
                    }
                    window.location.href = href;
                }
            }
        }
    }, true);

    // 在外部网页中移除 Tauri 内部 IPC 入口，防止第三方脚本直接调用任意 Rust 命令
    if (!APP_SHELL) {
        try { delete window.__TAURI_INTERNALS__; } catch (e) { try { window.__TAURI_INTERNALS__ = undefined; } catch (_) {} }
        try { delete window.__TAURI__; } catch (e) { try { window.__TAURI__ = undefined; } catch (_) {} }
    }
})();
