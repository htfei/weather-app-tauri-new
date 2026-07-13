(function() {
    'use strict';

    var HOME_URL = '{{WEBWRAPPER_HOME_URL}}';

    function isAppShell() {
        var host = window.location.host;
        var protocol = window.location.protocol;
        return host === 'tauri.localhost' ||
               host === 'localhost:5173' ||
               host === '127.0.0.1:5173' ||
               protocol === 'tauri:' ||
               window.location.href.indexOf('tauri://') === 0;
    }

    if (isAppShell()) return;
    if (document.getElementById('webwrapper-navbar')) return;

    var body = document.body;
    if (!body) return;

    var safeTop = 'env(safe-area-inset-top, 0px)';
    var nb = document.createElement('div');
    nb.id = 'webwrapper-navbar';
    nb.style.cssText =
        'position:fixed;' +
        'top:' + safeTop + ';' +
        'left:0;right:0;' +
        'height:44px;' +
        'background:rgba(255,255,255,0.95);' +
        'border-bottom:1px solid #eee;' +
        'display:flex;' +
        'align-items:center;' +
        'padding:0 8px;' +
        'padding-top:' + safeTop + ';' +
        'z-index:2147483647;' +
        'box-shadow:0 2px 4px rgba(0,0,0,0.1);' +
        'backdrop-filter:blur(10px);' +
        '-webkit-backdrop-filter:blur(10px);';

    function safeNavigate(u) {
        if (window.__webwrapper_navigate__ && typeof window.__webwrapper_navigate__ === 'function') {
            window.__webwrapper_navigate__(u);
        } else {
            window.location.href = u;
        }
    }

    var btnStyle = 'width:32px;height:32px;border:none;background:#f0f0f0;border-radius:6px;cursor:pointer;font-size:16px;margin-right:4px;display:flex;align-items:center;justify-content:center;color:#333;';

    var bb = document.createElement('button'); bb.innerHTML = '←'; bb.style.cssText = btnStyle; bb.title = '后退'; bb.onclick = function() { window.history.back(); };
    var fb = document.createElement('button'); fb.innerHTML = '→'; fb.style.cssText = btnStyle; fb.title = '前进'; fb.onclick = function() { window.history.forward(); };
    var hb = document.createElement('button'); hb.innerHTML = '🏠'; hb.style.cssText = btnStyle + 'margin-right:8px;'; hb.title = '主页'; hb.onclick = function() {
        safeNavigate(HOME_URL);
    };
    var sb = document.createElement('button'); sb.innerHTML = '⚙'; sb.style.cssText = btnStyle + 'margin-right:8px;'; sb.title = '设置'; sb.onclick = function() {
        safeNavigate(HOME_URL + '#settings');
    };

    var ui = document.createElement('input');
    ui.type = 'text';
    ui.value = window.location.href;
    ui.style.cssText = 'flex:1;height:32px;border:1px solid #ddd;border-radius:6px;padding:0 8px;font-size:14px;background:#fff;color:#333;';
    ui.onkeydown = function(e) {
        if (e.key === 'Enter') {
            var u = this.value.trim();
            if (u.indexOf('http') !== 0) u = 'https://' + u;
            safeNavigate(u);
        }
    };

    nb.appendChild(bb);
    nb.appendChild(fb);
    nb.appendChild(hb);
    nb.appendChild(sb);
    nb.appendChild(ui);
    body.style.paddingTop = 'calc(44px + ' + safeTop + ')';
    body.appendChild(nb);

    window.addEventListener('popstate', function() {
        ui.value = window.location.href;
    });
})();
