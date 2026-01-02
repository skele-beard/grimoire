// popup.js - Handles the popup UI interactions
const statusIndicator = document.getElementById('statusIndicator');
const statusText = document.getElementById('statusText');

function showStatus(message, isActive = false) {
    statusText.textContent = message;
    if (isActive) {
        statusIndicator.classList.add('active');
    } else {
        statusIndicator.classList.remove('active');
    }
}

// Test connection on load
async function checkConnection() {
    showStatus('CHECKING CONNECTION...');
    
    try {
        const response = await browser.runtime.sendMessage({
            action: "ping"
        });
        
        if (response.success) {
            showStatus('CONNECTED', true);
        } else {
            showStatus('DISCONNECTED');
        }
    } catch (error) {
        showStatus('DISCONNECTED');
    }
}

// Test connection button
document.getElementById('testButton').addEventListener('click', async () => {
    showStatus('TESTING...');
    
    try {
        const response = await browser.runtime.sendMessage({
            action: "ping"
        });
        
        if (response.success) {
            showStatus('CONNECTION OK', true);
            setTimeout(() => checkConnection(), 2000);
        } else {
            showStatus('CONNECTION FAILED');
        }
    } catch (error) {
        showStatus('ERROR: ' + error.message.toUpperCase());
    }
});

// Check connection on popup open
checkConnection();ideStatus();
