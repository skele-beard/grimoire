// background.js - Uses content script startup to detect successful logins

// Store pending credentials (survives page reloads)
let pendingCredentials = new Map();

// Store recently sent credentials to avoid duplicates
let recentlySent = new Map();

function shouldSendCredentials(domain, username) {
    const key = `${domain}:${username}`;
    const lastSent = recentlySent.get(key);
    
    // Don't resend if we sent these credentials in the last 5 minutes
    if (lastSent && Date.now() - lastSent < 300000) {
        console.log('Grimoire: Skipping duplicate send for', domain, username);
        return false;
    }
    
    return true;
}

function markCredentialsAsSent(domain, username) {
    const key = `${domain}:${username}`;
    recentlySent.set(key, Date.now());
    
    // Clean up old entries after 10 minutes
    setTimeout(() => {
        recentlySent.delete(key);
    }, 600000);
}

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
    // NEW: Content script reports its startup state
    if (message.action === "content_script_ready") {
        const tabId = sender.tab.id;
        const hasPasswordField = message.hasPasswordField;
        const domain = message.domain;
        
        console.log('Grimoire: Content script ready on tab', tabId);
        console.log('  Domain:', domain);
        console.log('  Has password field:', hasPasswordField);
        
        const pending = pendingCredentials.get(tabId);
        
        if (pending) {
            const timeSinceSubmit = Date.now() - pending.timestamp;
            
            // Only process if credentials were submitted recently (within 10 seconds)
            if (timeSinceSubmit < 10000) {
                console.log('Found pending credentials from', timeSinceSubmit, 'ms ago');
                
                // If no password field, login was successful!
                if (!hasPasswordField) {
                    console.log('Grimoire: Login successful (no password field) - saving credentials');
                    
                    // Check if we should send (avoid duplicates)
                    if (!shouldSendCredentials(pending.domain, pending.username)) {
                        console.log('Grimoire: Skipping duplicate credential save');
                        pendingCredentials.delete(tabId);
                        sendResponse({ success: true, action: 'duplicate' });
                        return true;
                    }
                    
                    // Save the credentials via native messaging
                    sendCredentials(pending.domain, pending.username, pending.password)
                        .then(result => {
                            if (result.success) {
                                console.log('Grimoire: Credentials saved successfully');
                                markCredentialsAsSent(pending.domain, pending.username);
                                
                                // Notify content script of success
                                browser.tabs.sendMessage(tabId, {
                                    action: "credentials_saved"
                                }).catch(() => {});
                            }
                        })
                        .catch(error => {
                            console.error('Grimoire: Failed to save credentials:', error);
                        });
                    
                    // Clean up
                    pendingCredentials.delete(tabId);
                    sendResponse({ success: true, action: 'saved' });
                } else {
                    // Still on login page, login failed
                    console.log('Grimoire: Login failed (still has password field) - discarding credentials');
                    pendingCredentials.delete(tabId);
                    
                    // Notify content script
                    browser.tabs.sendMessage(tabId, {
                        action: "credentials_discarded"
                    }).catch(() => {});
                    
                    sendResponse({ success: true, action: 'discarded' });
                }
            } else {
                console.log('Pending credentials too old (', timeSinceSubmit, 'ms), ignoring');
                sendResponse({ success: true, action: 'expired' });
            }
        } else {
            console.log('No pending credentials for this tab');
            sendResponse({ success: true, action: 'none' });
        }
        
        return true;
    }
    
    // Handle credentials submitted
    if (message.action === "credentials_submitted") {
        const tabId = sender.tab.id;
        
        console.log('Grimoire: Storing pending credentials for tab', tabId);
        
        pendingCredentials.set(tabId, {
            domain: message.domain,
            username: message.username,
            password: message.password,
            timestamp: Date.now()
        });
        
        sendResponse({ success: true });
        return true;
    }
    
    // Get credentials for autofill
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
    
    // EXISTING: Direct credential send (keeping for backward compatibility/popup)
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

// Clean up stale pending credentials after 30 seconds
setInterval(() => {
    const now = Date.now();
    for (let [tabId, creds] of pendingCredentials.entries()) {
        if (now - creds.timestamp > 30000) {
            console.log('Grimoire: Cleaning up stale credentials for tab', tabId);
            pendingCredentials.delete(tabId);
        }
    }
}, 15000);

// Clean up when tabs are closed
browser.tabs.onRemoved.addListener((tabId) => {
    if (pendingCredentials.has(tabId)) {
        console.log('Grimoire: Tab closed, cleaning up pending credentials for tab', tabId);
        pendingCredentials.delete(tabId);
    }
});

// EXISTING: Auto-fill on page load
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
