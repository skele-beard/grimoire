// popup.js - Popup UI logic

// Firefox compatibility
if (typeof browser === 'undefined') {
    var browser = chrome;
}

const statusIndicator = document.getElementById('statusIndicator');
const statusText      = document.getElementById('statusText');
const domainText      = document.getElementById('domainText');
const credStatus      = document.getElementById('credentialStatus');
const fillButton      = document.getElementById('fillButton');

function showStatus(message, isActive = false) {
    statusText.textContent = message;
    statusIndicator.classList.toggle('active', isActive);
}

async function checkConnection() {
    showStatus('CHECKING...');
    try {
        const response = await browser.runtime.sendMessage({ action: 'ping' });
        showStatus(response.success ? 'CONNECTED' : 'DISCONNECTED', response.success);
    } catch (e) {
        showStatus('DISCONNECTED');
    }
}

async function loadCurrentTabInfo() {
    domainText.textContent = '—';
    credStatus.textContent = '—';
    fillButton.disabled    = true;

    let tabs;
    try {
        tabs = await browser.tabs.query({ active: true, currentWindow: true });
    } catch (e) {
        credStatus.textContent = 'ERROR';
        return;
    }

    if (!tabs.length) return;

    let domain;
    try {
        domain = new URL(tabs[0].url).hostname;
    } catch (e) {
        domainText.textContent = 'N/A';
        credStatus.textContent = 'NOT AN HTTP PAGE';
        return;
    }

    domainText.textContent = domain || '—';
    credStatus.textContent = 'CHECKING...';

    try {
        const response = await browser.runtime.sendMessage({
            action: 'get_credentials',
            domain: domain
        });

        if (response.success && response.credentials && response.credentials.username) {
            const user = response.credentials.username;
            const display = user.length > 24 ? user.slice(0, 21) + '...' : user;
            credStatus.textContent = display;
            fillButton.disabled = false;
        } else {
            credStatus.textContent = 'NONE';
        }
    } catch (e) {
        credStatus.textContent = 'NONE';
    }
}

document.getElementById('testButton').addEventListener('click', async () => {
    showStatus('TESTING...');
    try {
        const response = await browser.runtime.sendMessage({ action: 'ping' });
        if (response.success) {
            showStatus('CONNECTION OK', true);
            setTimeout(checkConnection, 2000);
        } else {
            showStatus('CONNECTION FAILED');
        }
    } catch (e) {
        showStatus('ERROR');
    }
});

fillButton.addEventListener('click', async () => {
    fillButton.disabled    = true;
    fillButton.textContent = '[ FILLING... ]';
    try {
        const response = await browser.runtime.sendMessage({ action: 'popup_autofill' });
        if (response && response.success) {
            window.close();
        } else {
            credStatus.textContent    = 'FILL FAILED';
            fillButton.textContent    = '[ AUTOFILL ]';
            fillButton.disabled       = false;
        }
    } catch (e) {
        credStatus.textContent = 'FILL FAILED';
        fillButton.textContent = '[ AUTOFILL ]';
        fillButton.disabled    = false;
    }
});

// Initialize
checkConnection();
loadCurrentTabInfo();
