// background.js - Credential lifecycle management for Grimoire

// Firefox compatibility
if (typeof browser === 'undefined') {
    var browser = chrome;
}

// Pending credentials waiting for login-success confirmation (keyed by tabId)
let pendingCredentials = new Map();

// Pending usernames for multi-step flows like Google/Microsoft (keyed by tabId)
let pendingUsernames = new Map();

// Dedup guard: avoid re-prompting for credentials saved in the last 5 minutes
let recentlySent = new Map();

function shouldPrompt(domain, username) {
    const key = `${domain}:${username}`;
    const last = recentlySent.get(key);
    return !(last && Date.now() - last < 300000);
}

function markAsSent(domain, username) {
    const key = `${domain}:${username}`;
    recentlySent.set(key, Date.now());
    setTimeout(() => recentlySent.delete(key), 600000);
}

// ===== NATIVE MESSAGING HELPERS =====

async function getCredentials(domain) {
    const response = await browser.runtime.sendNativeMessage(
        'com.grimoire.native',
        { action: 'get_credentials', domain: domain }
    );
    if (response.ok && response.username) {
        return { username: response.username, password: response.password };
    }
    throw new Error(response.error || 'No credentials found');
}

async function saveCredentials(domain, username, password) {
    const response = await browser.runtime.sendNativeMessage(
        'com.grimoire.native',
        { action: 'set_credentials', domain: domain, username: username, password: password }
    );
    if (response.ok) return true;
    throw new Error(response.error || 'Failed to save credentials');
}

async function ping() {
    try {
        const response = await browser.runtime.sendNativeMessage(
            'com.grimoire.native',
            { action: 'ping' }
        );
        return !!response.ok;
    } catch (e) {
        return false;
    }
}

// ===== CORE LOGIC =====

// Compare submitted credentials against stored ones, then send the right prompt.
async function evaluateAndPrompt(tabId, domain, username, password) {
    if (!shouldPrompt(domain, username)) {
        console.log('Grimoire: Skipping duplicate prompt for', domain);
        return;
    }

    let existing = null;
    try {
        existing = await getCredentials(domain);
    } catch (e) {
        // No stored credentials — will prompt to save
    }

    let action;
    if (!existing) {
        action = 'save_prompt';
    } else if (existing.username === username && existing.password === password) {
        console.log('Grimoire: Credentials unchanged, no prompt needed');
        return;
    } else {
        action = 'update_prompt';
    }

    browser.tabs.sendMessage(tabId, {
        action: action,
        domain: domain,
        username: username,
        password: password
    }).catch(err => console.warn('Grimoire: Could not send prompt to tab', tabId, ':', err.message));
}

// ===== MESSAGE HANDLER =====

browser.runtime.onMessage.addListener((message, sender, sendResponse) => {

    // ── Content script finished loading ──────────────────────────────────────
    if (message.action === 'content_script_ready') {
        const tabId = sender.tab.id;
        const { hasPasswordField, domain } = message;

        // Priority 1: process credentials pending from a previous page submit
        const pending = pendingCredentials.get(tabId);
        if (pending) {
            const age = Date.now() - pending.timestamp;
            if (age < 10000) {
                if (!hasPasswordField) {
                    // Navigated to a page with no password field → login succeeded
                    console.log('Grimoire: Login success (no password field on new page)');
                    pendingCredentials.delete(tabId);
                    evaluateAndPrompt(tabId, pending.domain, pending.username, pending.password)
                        .catch(e => console.error('Grimoire: evaluateAndPrompt error:', e));
                } else {
                    // Still on a page with a password field → login failed
                    console.log('Grimoire: Login failed (password field still present)');
                    pendingCredentials.delete(tabId);
                }
            } else {
                pendingCredentials.delete(tabId);
            }
            sendResponse({ success: true });
            return true;
        }

        // Priority 2: send back a pending username for multi-step login (step 2)
        const pendingUser = pendingUsernames.get(tabId);
        if (pendingUser && pendingUser.domain === domain && hasPasswordField) {
            console.log('Grimoire: Returning pending username for multi-step login');
            sendResponse({ action: 'pending_username', username: pendingUser.username });
            return true;
        }

        sendResponse({ success: true });
        return true;
    }

    // ── Multi-step login: username captured on step 1 ────────────────────────
    if (message.action === 'username_captured') {
        const tabId = sender.tab.id;
        pendingUsernames.set(tabId, {
            username: message.username,
            domain: message.domain,
            timestamp: Date.now()
        });
        sendResponse({ success: true });
        return true;
    }

    // ── Form submitted with full credentials ─────────────────────────────────
    if (message.action === 'credentials_submitted') {
        const tabId = sender.tab.id;
        pendingCredentials.set(tabId, {
            domain: message.domain,
            username: message.username,
            password: message.password,
            timestamp: Date.now()
        });
        // Full credentials received — clear any pending username
        pendingUsernames.delete(tabId);
        sendResponse({ success: true });
        return true;
    }

    // ── SPA login succeeded without page navigation ──────────────────────────
    if (message.action === 'login_success_spa') {
        const tabId = sender.tab.id;
        console.log('Grimoire: SPA login success for', message.domain);
        evaluateAndPrompt(tabId, message.domain, message.username, message.password)
            .catch(e => console.error('Grimoire: evaluateAndPrompt error:', e));
        sendResponse({ success: true });
        return true;
    }

    // ── User confirmed save/update from in-page prompt ───────────────────────
    if (message.action === 'user_confirmed_save') {
        const { domain, username, password } = message;
        const tabId = sender.tab.id;

        saveCredentials(domain, username, password)
            .then(() => {
                markAsSent(domain, username);
                browser.tabs.sendMessage(tabId, { action: 'credentials_saved' }).catch(() => {});
                sendResponse({ success: true });
            })
            .catch(err => {
                console.error('Grimoire: Save failed:', err);
                browser.tabs.sendMessage(tabId, { action: 'credentials_save_failed' }).catch(() => {});
                sendResponse({ success: false, error: err.message });
            });
        return true;
    }

    // ── Get credentials for a domain (content script pull / popup query) ─────
    if (message.action === 'get_credentials') {
        getCredentials(message.domain)
            .then(creds => sendResponse({ success: true, credentials: creds }))
            .catch(err => sendResponse({ success: false, error: err.message }));
        return true;
    }

    // ── Popup requests immediate autofill for the active tab ─────────────────
    if (message.action === 'popup_autofill') {
        browser.tabs.query({ active: true, currentWindow: true }, tabs => {
            if (!tabs.length) { sendResponse({ success: false, error: 'No active tab' }); return; }
            let domain;
            try { domain = new URL(tabs[0].url).hostname; }
            catch (e) { sendResponse({ success: false, error: 'Invalid URL' }); return; }

            getCredentials(domain)
                .then(creds => {
                    // Direct fill — user already expressed intent by clicking the popup button
                    browser.tabs.sendMessage(tabs[0].id, {
                        action: 'autofill',
                        credentials: creds
                    }).catch(() => {});
                    sendResponse({ success: true });
                })
                .catch(err => sendResponse({ success: false, error: err.message }));
        });
        return true;
    }

    // ── Ping ─────────────────────────────────────────────────────────────────
    if (message.action === 'ping') {
        ping()
            .then(ok => {
                if (ok) {
                    sendResponse({ success: true, message: 'Connected to Grimoire!' });
                } else {
                    sendResponse({ success: false, error: 'Grimoire is not running or locked' });
                }
            })
            .catch(err => sendResponse({ success: false, error: err.message }));
        return true;
    }
});

// ===== HOUSEKEEPING =====

setInterval(() => {
    const now = Date.now();
    for (const [id, c] of pendingCredentials) {
        if (now - c.timestamp > 30000) pendingCredentials.delete(id);
    }
    for (const [id, u] of pendingUsernames) {
        if (now - u.timestamp > 120000) pendingUsernames.delete(id);
    }
}, 15000);

browser.tabs.onRemoved.addListener(tabId => {
    pendingCredentials.delete(tabId);
    pendingUsernames.delete(tabId);
});
