// content.js - In-page credential capture, autofill prompts, and save/update prompts

// Firefox compatibility
if (typeof browser === 'undefined') {
    var browser = chrome;
}

// ===== STATE =====

let lastSeenCredentials = { username: '', password: '', domain: '' };
let listenersAttached   = false;
let autofillRequested   = false;
let pendingUsername     = null;  // Persisted across SPA navigation for multi-step flows
let spaMonitorId        = null;  // setInterval handle for SPA login result monitoring

// ===== DOM HELPERS =====

function findPasswordField() {
    try {
        const direct = document.querySelector('input[type="password"]');
        if (direct) return direct;

        for (const input of document.querySelectorAll('input')) {
            const combined = [
                input.name, input.id, input.placeholder,
                input.getAttribute('aria-label'), input.className,
                input.getAttribute('autocomplete')
            ].join(' ').toLowerCase();
            if (/password|passwd|pwd|pass/.test(combined)) return input;
        }

        for (const el of document.querySelectorAll('*')) {
            if (!el.shadowRoot) continue;
            const sh = el.shadowRoot.querySelector('input[type="password"]');
            if (sh) return sh;
            for (const input of el.shadowRoot.querySelectorAll('input')) {
                const combined = (input.name + input.id).toLowerCase();
                if (/password|passwd|pwd|pass/.test(combined)) return input;
            }
        }
        return null;
    } catch (e) {
        return null;
    }
}

function findUsernameField() {
    try {
        const direct = document.querySelector(
            'input[type="email"], input[autocomplete="username"], input[autocomplete="email"]'
        );
        if (direct) return direct;

        for (const input of document.querySelectorAll('input[type="text"], input:not([type])')) {
            const combined = [
                input.name, input.id, input.placeholder,
                input.getAttribute('aria-label'), input.className,
                input.getAttribute('autocomplete')
            ].join(' ').toLowerCase();
            if (/username|user|email|login|account|signin/.test(combined)) return input;
        }

        for (const el of document.querySelectorAll('*')) {
            if (!el.shadowRoot) continue;
            const sh = el.shadowRoot.querySelector(
                'input[type="email"], input[autocomplete="username"]'
            );
            if (sh) return sh;
            for (const input of el.shadowRoot.querySelectorAll('input[type="text"], input:not([type])')) {
                const combined = (input.name + input.id).toLowerCase();
                if (/username|user|email|login/.test(combined)) return input;
            }
        }
        return null;
    } catch (e) {
        return null;
    }
}

function hasLoginForm() {
    return findPasswordField() !== null || findUsernameField() !== null;
}

// Detect visible error text after a failed login attempt
function hasLoginError() {
    const selectors = [
        '[role="alert"]', '[aria-live]',
        '[class*="error"]', '[class*="alert"]',
        '[class*="invalid"]', '[class*="warning"]', '[id*="error"]'
    ];
    for (const sel of selectors) {
        try {
            for (const el of document.querySelectorAll(sel)) {
                if (el.offsetParent === null) continue; // hidden
                if (/incorrect|invalid|wrong password|failed|denied|unauthorized|try again|not match/i
                        .test(el.textContent)) {
                    return true;
                }
            }
        } catch (e) { /* ignore */ }
    }
    return false;
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str || '';
    return div.innerHTML;
}

// ===== STYLES =====

function ensureStyles() {
    if (document.getElementById('grimoire-styles')) return;
    const style = document.createElement('style');
    style.id = 'grimoire-styles';
    style.textContent = `
        @keyframes grm-slideIn  { from { transform:translateX(360px); opacity:0 } to { transform:translateX(0); opacity:1 } }
        @keyframes grm-slideOut { from { transform:translateX(0); opacity:1 } to { transform:translateX(360px); opacity:0 } }

        #grimoire-autofill-prompt,
        #grimoire-save-prompt {
            position: fixed;
            top: 14px;
            right: 14px;
            z-index: 2147483647;
            font-family: 'Courier New', monospace;
            font-size: 11px;
            background: #141414;
            color: #c0c0c0;
            border: 2px solid #3a3a3a;
            border-bottom: 3px solid #060606;
            border-right: 3px solid #060606;
            padding: 13px 15px;
            min-width: 270px;
            max-width: 340px;
            box-shadow: 0 6px 24px rgba(0,0,0,0.85), inset 0 0 18px rgba(0,0,0,0.5);
            animation: grm-slideIn 0.18s ease-out;
            letter-spacing: 0.5px;
        }
        .grm-title {
            font-size: 9px;
            color: #555;
            letter-spacing: 2px;
            text-transform: uppercase;
            margin-bottom: 7px;
            padding-bottom: 5px;
            border-bottom: 1px solid #282828;
        }
        .grm-body {
            color: #9a9a9a;
            margin-bottom: 5px;
            line-height: 1.45;
        }
        .grm-body b { color: #b8b8b8; }
        .grm-user {
            color: #686868;
            font-size: 10px;
            margin-bottom: 10px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }
        .grm-buttons {
            display: flex;
            gap: 8px;
        }
        .grm-btn {
            flex: 1;
            padding: 6px 4px;
            background: #1c1c1c;
            color: #909090;
            border: 1px solid #363636;
            border-bottom: 2px solid #080808;
            border-right: 2px solid #080808;
            font-family: 'Courier New', monospace;
            font-size: 10px;
            letter-spacing: 1px;
            text-transform: uppercase;
            cursor: pointer;
            transition: background 0.08s, color 0.08s;
        }
        .grm-btn:hover { background: #282828; color: #c0c0c0; }
        .grm-btn:active { transform: translate(1px,1px); border-bottom-width: 1px; border-right-width: 1px; }
        .grm-btn-primary { border-color: #484848; color: #b0b0b0; }
        .grm-close {
            position: absolute;
            top: 5px;
            right: 8px;
            background: none;
            border: none;
            color: #484848;
            font-family: 'Courier New', monospace;
            font-size: 14px;
            cursor: pointer;
            line-height: 1;
            padding: 0;
        }
        .grm-close:hover { color: #808080; }

        #grimoire-notification {
            position: fixed;
            bottom: 18px;
            right: 18px;
            z-index: 2147483647;
            font-family: 'Courier New', monospace;
            font-size: 11px;
            letter-spacing: 1px;
            background: #0e0e0e;
            padding: 9px 14px;
            border: 2px solid #252525;
            border-bottom: 3px solid #000;
            border-right: 3px solid #000;
            box-shadow: inset 0 0 16px rgba(0,0,0,0.7);
            animation: grm-slideIn 0.18s ease-out;
        }
    `;
    (document.head || document.documentElement).appendChild(style);
}

function dismissById(id) {
    const el = document.getElementById(id);
    if (!el) return;
    el.style.animation = 'grm-slideOut 0.18s ease-in forwards';
    setTimeout(() => el.remove(), 200);
}

// ===== PROMPT UIs =====

function showAutofillPrompt(domain, credentials) {
    if (!hasLoginForm()) return;  // Nothing to fill
    if (document.getElementById('grimoire-autofill-prompt')) return;
    ensureStyles();

    const user = credentials.username || '';
    const truncUser = user.length > 30 ? user.slice(0, 27) + '...' : user;

    const el = document.createElement('div');
    el.id = 'grimoire-autofill-prompt';
    el.innerHTML = `
        <button class="grm-close" id="grm-af-x">×</button>
        <div class="grm-title">[ GRIMOIRE ]</div>
        <div class="grm-body">Autofill credentials for<br><b>${escapeHtml(domain)}</b>?</div>
        ${truncUser ? `<div class="grm-user">User: ${escapeHtml(truncUser)}</div>` : ''}
        <div class="grm-buttons">
            <button class="grm-btn grm-btn-primary" id="grm-af-yes">[ YES ]</button>
            <button class="grm-btn" id="grm-af-no">[ NO ]</button>
        </div>
    `;
    (document.body || document.documentElement).appendChild(el);

    const dismiss = () => dismissById('grimoire-autofill-prompt');
    document.getElementById('grm-af-yes').onclick = () => { fillCredentials(credentials); dismiss(); };
    document.getElementById('grm-af-no').onclick  = dismiss;
    document.getElementById('grm-af-x').onclick   = dismiss;
    setTimeout(dismiss, 14000);
}

function showSavePrompt(type, domain, username, password) {
    if (document.getElementById('grimoire-save-prompt')) return;
    ensureStyles();

    const isUpdate = type === 'update';
    const user = username || '';
    const truncUser = user.length > 30 ? user.slice(0, 27) + '...' : user;

    const el = document.createElement('div');
    el.id = 'grimoire-save-prompt';
    el.innerHTML = `
        <button class="grm-close" id="grm-sv-x">×</button>
        <div class="grm-title">[ GRIMOIRE ]</div>
        <div class="grm-body">${isUpdate ? 'Update' : 'Save'} credentials for<br><b>${escapeHtml(domain)}</b>?</div>
        ${truncUser ? `<div class="grm-user">User: ${escapeHtml(truncUser)}</div>` : ''}
        <div class="grm-buttons">
            <button class="grm-btn grm-btn-primary" id="grm-sv-yes">[ ${isUpdate ? 'UPDATE' : 'SAVE'} ]</button>
            <button class="grm-btn" id="grm-sv-no">[ DISMISS ]</button>
        </div>
    `;
    (document.body || document.documentElement).appendChild(el);

    const dismiss = () => dismissById('grimoire-save-prompt');
    document.getElementById('grm-sv-yes').onclick = () => {
        browser.runtime.sendMessage({
            action: 'user_confirmed_save',
            domain: domain,
            username: username,
            password: password
        }).catch(() => {});
        dismiss();
    };
    document.getElementById('grm-sv-no').onclick = dismiss;
    document.getElementById('grm-sv-x').onclick  = dismiss;
    setTimeout(dismiss, 22000);
}

// ===== AUTOFILL =====

function fillCredentials(credentials) {
    const { username, password } = credentials;
    // Use the native setter so React/Vue/Angular synthetic event systems register the change
    const nativeSetter = Object.getOwnPropertyDescriptor(
        window.HTMLInputElement.prototype, 'value'
    ).set;

    let filled = false;

    const userField = findUsernameField();
    if (userField && username) {
        nativeSetter.call(userField, username);
        userField.dispatchEvent(new Event('input',  { bubbles: true }));
        userField.dispatchEvent(new Event('change', { bubbles: true }));
        filled = true;
    }

    const passField = findPasswordField();
    if (passField && password) {
        nativeSetter.call(passField, password);
        passField.dispatchEvent(new Event('input',  { bubbles: true }));
        passField.dispatchEvent(new Event('change', { bubbles: true }));
        filled = true;
    }

    showNotification(filled ? 'Credentials filled' : 'No login fields found', !filled);
}

// ===== NOTIFICATIONS =====

function showNotification(message, isWarning = false) {
    ensureStyles();
    const old = document.getElementById('grimoire-notification');
    if (old) old.remove();

    const n = document.createElement('div');
    n.id = 'grimoire-notification';
    n.style.color = isWarning ? '#606060' : '#888888';
    n.textContent = `[ GRIMOIRE ] ${message}`;
    (document.body || document.documentElement).appendChild(n);

    setTimeout(() => {
        if (!n.parentNode) return;
        n.style.animation = 'grm-slideOut 0.18s ease-in forwards';
        setTimeout(() => n.remove(), 200);
    }, 3200);
}

// ===== SPA LOGIN MONITORING =====

// After a form is submitted in a SPA (no page reload), watch the DOM for
// success (password field vanishes) or failure (error message appears).
function startSpaMonitoring(domain, username, password) {
    if (spaMonitorId) clearInterval(spaMonitorId);

    const start      = Date.now();
    const initialUrl = window.location.href;

    spaMonitorId = setInterval(() => {
        // If the page navigated, let the content_script_ready flow handle it
        if (window.location.href !== initialUrl) {
            clearInterval(spaMonitorId);
            spaMonitorId = null;
            return;
        }

        if (hasLoginError()) {
            console.log('Grimoire: SPA login error detected');
            clearInterval(spaMonitorId);
            spaMonitorId = null;
            return;
        }

        if (!findPasswordField()) {
            console.log('Grimoire: SPA login success (password field gone)');
            clearInterval(spaMonitorId);
            spaMonitorId = null;
            browser.runtime.sendMessage({
                action: 'login_success_spa',
                domain: domain,
                username: username,
                password: password
            }).catch(() => {});
            return;
        }

        if (Date.now() - start > 8000) {
            clearInterval(spaMonitorId);
            spaMonitorId = null;
        }
    }, 300);
}

// ===== CREDENTIAL CAPTURE =====

function captureAndSendCredentials() {
    const domain        = window.location.hostname;
    const usernameField = findUsernameField();
    const passwordField = findPasswordField();

    // Case 1: single-step login — both fields present
    if (usernameField && passwordField && usernameField.value && passwordField.value) {
        const username = usernameField.value;
        const password = passwordField.value;

        if (lastSeenCredentials.username === username &&
            lastSeenCredentials.password === password &&
            lastSeenCredentials.domain   === domain) return;

        lastSeenCredentials = { username, password, domain };

        browser.runtime.sendMessage({
            action: 'credentials_submitted',
            domain: domain, username: username, password: password
        }).catch(() => {});

        pendingUsername = null;
        startSpaMonitoring(domain, username, password);
        return;
    }

    // Case 2: username only (multi-step step 1 — e.g. Google, Microsoft)
    if (usernameField && usernameField.value && !passwordField) {
        const username = usernameField.value;
        pendingUsername = username;
        // Persist in background so it survives page navigation
        browser.runtime.sendMessage({
            action: 'username_captured',
            domain: domain,
            username: username
        }).catch(() => {});
        return;
    }

    // Case 3: password only (multi-step step 2)
    if (passwordField && passwordField.value && !usernameField && pendingUsername) {
        const password = passwordField.value;
        const username = pendingUsername;

        if (lastSeenCredentials.username === username &&
            lastSeenCredentials.password === password &&
            lastSeenCredentials.domain   === domain) return;

        lastSeenCredentials = { username, password, domain };

        browser.runtime.sendMessage({
            action: 'credentials_submitted',
            domain: domain, username: username, password: password
        }).catch(() => {});

        pendingUsername = null;
        startSpaMonitoring(domain, username, password);
    }
}

// ===== AUTOFILL REQUEST (pull-based) =====

// Called once a login form is detected. Asks the background for stored credentials
// and shows the prompt if any are found.
function requestAutofillIfNeeded() {
    if (autofillRequested) return;
    autofillRequested = true;

    const domain = window.location.hostname;
    browser.runtime.sendMessage({ action: 'get_credentials', domain: domain })
        .then(response => {
            if (response && response.success && response.credentials) {
                showAutofillPrompt(domain, response.credentials);
            }
        })
        .catch(() => {});
}

// ===== FORM MONITORING =====

function setupFormMonitoring() {
    if (listenersAttached) return;
    listenersAttached = true;

    // Pull autofill credentials now that we've confirmed a login form exists
    requestAutofillIfNeeded();

    // Standard form submit
    document.addEventListener('submit', () => captureAndSendCredentials(), true);

    // SPA-style login button clicks
    document.addEventListener('click', event => {
        const target = event.target;
        const button = target.tagName === 'BUTTON'
            ? target
            : target.closest && target.closest('button');

        if (!button && target.type !== 'submit') return;

        const text = (button || target).textContent.toLowerCase();
        if (text.includes('log in') || text.includes('sign in') ||
            text.includes('login')  || text.includes('signin')  ||
            (button || target).type === 'submit') {
            setTimeout(() => captureAndSendCredentials(), 100);
        }
    }, true);

    // Enter key in any login field
    document.addEventListener('keydown', event => {
        if (event.key !== 'Enter') return;
        const t = event.target;
        if (t.type === 'password' || t.type === 'text' || t.type === 'email') {
            setTimeout(() => captureAndSendCredentials(), 100);
        }
    }, true);
}

function watchForLoginForms() {
    if (hasLoginForm()) {
        setupFormMonitoring();
        return;
    }

    if (!document.body) {
        setTimeout(watchForLoginForms, 100);
        return;
    }

    const observer = new MutationObserver(() => {
        if (hasLoginForm() && !listenersAttached) {
            setupFormMonitoring();
        }
    });
    observer.observe(document.body, {
        childList: true, subtree: true,
        attributes: true, attributeFilter: ['type', 'class', 'id']
    });

    // Interval fallback for 30 s
    let ticks = 0;
    const timer = setInterval(() => {
        if (hasLoginForm() && !listenersAttached) {
            setupFormMonitoring();
            clearInterval(timer);
        }
        if (++ticks > 120) clearInterval(timer);
    }, 250);
}

// ===== MESSAGE LISTENER =====

browser.runtime.onMessage.addListener((message, sender, sendResponse) => {

    // Autofill prompt (with user acceptance) — triggered by pull request result
    if (message.action === 'autofill_prompt' && message.credentials) {
        showAutofillPrompt(message.domain, message.credentials);
        sendResponse({ success: true });
        return true;
    }

    // Direct autofill — used by the popup button (user already accepted)
    if (message.action === 'autofill' && message.credentials) {
        fillCredentials(message.credentials);
        sendResponse({ success: true });
        return true;
    }

    if (message.action === 'save_prompt') {
        showSavePrompt('save', message.domain, message.username, message.password);
        sendResponse({ success: true });
        return true;
    }

    if (message.action === 'update_prompt') {
        showSavePrompt('update', message.domain, message.username, message.password);
        sendResponse({ success: true });
        return true;
    }

    if (message.action === 'credentials_saved') {
        showNotification('Credentials saved');
        return true;
    }

    if (message.action === 'credentials_save_failed') {
        showNotification('Failed to save credentials', true);
        return true;
    }
});

// ===== STARTUP =====

function reportStartupState() {
    const domain         = window.location.hostname;
    const hasPasswordField = findPasswordField() !== null;

    browser.runtime.sendMessage({
        action: 'content_script_ready',
        domain: domain,
        hasPasswordField: hasPasswordField
    }).then(response => {
        // Background may send back a pending username for multi-step login step 2
        if (response && response.action === 'pending_username') {
            console.log('Grimoire: Received pending username from background:', response.username);
            pendingUsername = response.username;
        }
    }).catch(() => {});
}

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        reportStartupState();
        watchForLoginForms();
    });
} else {
    reportStartupState();
    watchForLoginForms();
}
