// background.js - Handles communication with Grimoire HTTP server

const GRIMOIRE_URL = "http://127.0.0.1:47777";

// Request credentials for a domain
async function getCredentials(domain) {
    try {
        const response = await fetch(GRIMOIRE_URL, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                action: "get_credentials",
                domain: domain
            })
        });
        
        const data = await response.json();
        
        if (data.ok) {
            return {
                username: data.username,
                password: data.password
            };
        } else {
            throw new Error(data.error || "Unknown error");
        }
    } catch (error) {
        throw new Error(`Could not connect to Grimoire: ${error.message}`);
    }
}

// Ping to test connection
async function ping() {
    try {
        const response = await fetch(GRIMOIRE_URL, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                action: "ping"
            })
        });
        
        const data = await response.json();
        return data.ok;
    } catch (error) {
        return false;
    }
}

// Listen for messages from popup or content scripts
browser.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.action === "get_credentials") {
        getCredentials(message.domain)
            .then(credentials => {
                sendResponse({
                    success: true,
                    credentials: credentials
                });
            })
            .catch(error => {
                sendResponse({
                    success: false,
                    error: error.message
                });
            });
        
        return true; // Keep message channel open for async response
    }
    
    if (message.action === "ping") {
        ping()
            .then(success => {
                if (success) {
                    sendResponse({ 
                        success: true, 
                        message: "Connected to Grimoire!" 
                    });
                } else {
                    sendResponse({ 
                        success: false, 
                        error: "Grimoire is not running or locked" 
                    });
                }
            })
            .catch(error => {
                sendResponse({ 
                    success: false, 
                    error: error.message 
                });
            });
        
        return true; // Keep message channel open for async response
    }
});

// Optional: Auto-fill on page load
browser.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
    if (changeInfo.status === 'complete' && tab.url) {
        try {
            const url = new URL(tab.url);
            const domain = url.hostname;
            
            // Skip non-http(s) URLs
            if (!url.protocol.startsWith('http')) {
                return;
            }
            
            getCredentials(domain)
                .then(credentials => {
                    browser.tabs.sendMessage(tabId, {
                        action: "autofill",
                        credentials: credentials
                    }).catch(() => {
                        // Content script not ready yet, that's ok
                    });
                })
                .catch(() => {
                    // No credentials for this domain, that's ok
                });
        } catch (e) {
            // Invalid URL, skip
        }
    }
});
