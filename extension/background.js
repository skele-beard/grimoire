// Request credentials for a domain
async function getCredentials(domain) {
    try {
        const response = await browser.runtime.sendNativeMessage(
            "com.grimoire.native",
            {
                action: "get_credentials",
                domain: domain
            }
        );
        
        if (response.ok) {
            return {
                username: response.username,
                password: response.password
            };
        } else {
            throw new Error(response.error || "Unknown error");
        }
    } catch (error) {
        throw new Error(`Could not connect to Grimoire: ${error.message}`);
    }
}

// Send new credentials to Grimoire for a domain
async function sendCredentials(domain, username, password) {
    try {
        const response = await browser.runtime.sendNativeMessage(
            "com.grimoire.native",
            {
                action: "set_credentials",
                domain: domain,
                username: username,
                password: password
            }
        );
        
        if (response.ok) {
            return {
                success: true,
                message: response.message || "Credentials saved successfully"
            };
        } else {
            throw new Error(response.error || "Unknown error");
        }
    } catch (error) {
        throw new Error(`Could not connect to Grimoire: ${error.message}`);
    }
}

// Ping to test connection
async function ping() {
    try {
        const response = await browser.runtime.sendNativeMessage(
            "com.grimoire.native",
            {
                action: "ping"
            }
        );
        return response.ok;
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
        
        return true;
    }
    
    if (message.action === "send_credentials") {
        sendCredentials(message.domain, message.username, message.password)
            .then(result => {
                sendResponse({
                    success: true,
                    message: result.message
                });
            })
            .catch(error => {
                sendResponse({
                    success: false,
                    error: error.message
                });
            });
        
        return true;
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
        
        return true;
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
