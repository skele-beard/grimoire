// popup.js - Handles the popup UI interactions

const statusDiv = document.getElementById('status');

function showStatus(message, isError = false) {
    statusDiv.textContent = message;
    statusDiv.className = isError ? 'error' : 'success';
    statusDiv.style.display = 'block';
}

function hideStatus() {
    statusDiv.style.display = 'none';
}

// Test connection button
document.getElementById('ping').addEventListener('click', async () => {
    showStatus('Testing connection...');
    
    try {
        const response = await browser.runtime.sendMessage({
            action: "ping"
        });
        
        if (response.success) {
            showStatus('✓ ' + response.message);
        } else {
            showStatus('✗ ' + response.error, true);
        }
    } catch (error) {
        showStatus('✗ Error: ' + error.message, true);
    }
});

// Fill current page button
document.getElementById('fill').addEventListener('click', async () => {
    showStatus('Getting credentials...');
    
    try {
        // Get current tab
        const tabs = await browser.tabs.query({active: true, currentWindow: true});
        const tab = tabs[0];
        
        if (!tab.url || !tab.url.startsWith('http')) {
            showStatus('✗ Cannot fill credentials on this page', true);
            return;
        }
        
        const url = new URL(tab.url);
        const domain = url.hostname;
        
        // Request credentials
        const response = await browser.runtime.sendMessage({
            action: "get_credentials",
            domain: domain
        });
        
        if (response.success) {
            // Inject content script if needed, then send message
            try {
                // Try to send message first
                await browser.tabs.sendMessage(tab.id, {
                    action: "autofill",
                    credentials: response.credentials
                });
                
                showStatus(`✓ Filled credentials for ${domain}`);
            } catch (error) {
                // Content script might not be loaded, try injecting it
                try {
                    await browser.tabs.executeScript(tab.id, {
                        file: "content.js"
                    });
                    
                    // Wait a moment for script to initialize
                    await new Promise(resolve => setTimeout(resolve, 100));
                    
                    // Try again
                    await browser.tabs.sendMessage(tab.id, {
                        action: "autofill",
                        credentials: response.credentials
                    });
                    
                    showStatus(`✓ Filled credentials for ${domain}`);
                } catch (injectError) {
                    console.error('Injection error:', injectError);
                    showStatus('✗ Could not fill credentials: ' + injectError.message, true);
                }
            }
        } else {
            showStatus('✗ ' + response.error, true);
        }
    } catch (error) {
        showStatus('✗ Error: ' + error.message, true);
    }
});

// Clear status when popup opens
hideStatus();
