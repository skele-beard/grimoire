console.log('Grimoire: Content script loaded');

// Firefox compatibility
if (typeof browser === 'undefined') {
    var browser = chrome;
}

// Store last seen credentials to avoid duplicates
let lastSeenCredentials = { username: '', password: '', domain: '' };
let listenersAttached = false;

// Store username from multi-step login flows (e.g., Google, Microsoft)
let pendingUsername = null;

// ===== HELPER FUNCTIONS =====

// Find password fields (including in shadow DOM and custom elements)
function findPasswordField() {
    try {
        // Check regular DOM first
        let passwordField = document.querySelector('input[type="password"]');
        if (passwordField) {
            console.log('Grimoire: Found password field in regular DOM');
            return passwordField;
        }
        
        // Check for inputs with password-related attributes using regex
        const allInputs = document.querySelectorAll('input');
        for (const input of allInputs) {
            // Check various attributes for "password" (case-insensitive)
            const name = input.getAttribute('name') || '';
            const id = input.getAttribute('id') || '';
            const placeholder = input.getAttribute('placeholder') || '';
            const ariaLabel = input.getAttribute('aria-label') || '';
            const className = input.getAttribute('class') || '';
            const autocomplete = input.getAttribute('autocomplete') || '';
            
            // Combine all attributes into one string to search
            const combined = (name + id + placeholder + ariaLabel + className + autocomplete).toLowerCase();
            
            if (/password|passwd|pwd|pass/i.test(combined)) {
                console.log('Grimoire: Found password field by regex match:', input);
                return input;
            }
        }
        
        // Check inside shadow roots
        const elementsWithShadow = document.querySelectorAll('*');
        for (const el of elementsWithShadow) {
            if (el.shadowRoot) {
                const shadowPassword = el.shadowRoot.querySelector('input[type="password"]');
                if (shadowPassword) {
                    console.log('Grimoire: Found password field in shadow DOM');
                    return shadowPassword;
                }
                
                // Also check shadow DOM inputs with regex
                const shadowInputs = el.shadowRoot.querySelectorAll('input');
                for (const input of shadowInputs) {
                    const name = input.getAttribute('name') || '';
                    const id = input.getAttribute('id') || '';
                    const combined = (name + id).toLowerCase();
                    
                    if (/password|passwd|pwd|pass/i.test(combined)) {
                        console.log('Grimoire: Found password field in shadow DOM by regex');
                        return input;
                    }
                }
            }
        }
        
        return null;
    } catch (error) {
        console.error('Grimoire: Error in findPasswordField:', error);
        return null;
    }
}

// Find username fields (including in shadow DOM and custom elements)
function findUsernameField() {
    try {
        // Standard username/email fields
        let usernameField = document.querySelector(
            'input[type="email"], ' +
            'input[autocomplete="username"], ' +
            'input[autocomplete="email"]'
        );
        if (usernameField) return usernameField;
        
        // Check all text inputs with regex for username-related terms
        const allInputs = document.querySelectorAll('input[type="text"], input:not([type])');
        for (const input of allInputs) {
            const name = input.getAttribute('name') || '';
            const id = input.getAttribute('id') || '';
            const placeholder = input.getAttribute('placeholder') || '';
            const ariaLabel = input.getAttribute('aria-label') || '';
            const className = input.getAttribute('class') || '';
            const autocomplete = input.getAttribute('autocomplete') || '';
            
            // Combine all attributes
            const combined = (name + id + placeholder + ariaLabel + className + autocomplete).toLowerCase();
            
            // Match username, email, login, user, etc.
            if (/username|user|email|login|account|signin/i.test(combined)) {
                console.log('Grimoire: Found username field by regex match:', input);
                return input;
            }
        }
        
        // Check inside shadow roots
        const elementsWithShadow = document.querySelectorAll('*');
        for (const el of elementsWithShadow) {
            if (el.shadowRoot) {
                const shadowUsername = el.shadowRoot.querySelector(
                    'input[type="email"], input[autocomplete="username"]'
                );
                if (shadowUsername) return shadowUsername;
                
                // Also check shadow DOM inputs with regex
                const shadowInputs = el.shadowRoot.querySelectorAll('input[type="text"], input:not([type])');
                for (const input of shadowInputs) {
                    const name = input.getAttribute('name') || '';
                    const id = input.getAttribute('id') || '';
                    const combined = (name + id).toLowerCase();
                    
                    if (/username|user|email|login/i.test(combined)) {
                        return input;
                    }
                }
            }
        }
        
        return null;
    } catch (error) {
        console.error('Grimoire: Error in findUsernameField:', error);
        return null;
    }
}

// ===== MESSAGE HANDLERS =====

// Listen for messages from background script
browser.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.action === "autofill" && message.credentials) {
        try {
            fillCredentials(message.credentials);
            sendResponse({ success: true });
        } catch (error) {
            console.error('Grimoire: Fill error:', error);
            sendResponse({ success: false, error: error.message });
        }
        return true;
    }
    
    if (message.action === "credentials_saved") {
        showNotification('Grimoire: Credentials saved!', false);
        return true;
    }
    
    if (message.action === "credentials_discarded") {
        showNotification('Grimoire: Login failed, credentials not saved', true);
        return true;
    }
    
    if (message.action === "credentials_save_failed") {
        showNotification('Grimoire: Failed to save credentials', true);
        return true;
    }
});

// ===== CORE FUNCTIONS =====

// Report startup state to background script
function reportStartupState() {
    const domain = window.location.hostname;
    const hasPasswordField = findPasswordField() !== null;
    
    console.log('Grimoire: Reporting startup state');
    console.log('  Domain:', domain);
    console.log('  Has password field:', hasPasswordField);
    
    browser.runtime.sendMessage({
        action: "content_script_ready",
        domain: domain,
        hasPasswordField: hasPasswordField
    }).then(response => {
        if (response && response.action) {
            console.log('Grimoire: Background response:', response.action);
        }
    }).catch(error => {
        console.error('Grimoire: Error reporting startup state:', error);
    });
}

// Capture and send credentials immediately on form submission
function captureAndSendCredentials() {
    const domain = window.location.hostname;
    
    const usernameField = findUsernameField();
    const passwordField = findPasswordField();
    
    // Case 1: Both username and password present (single-step login)
    if (usernameField && passwordField && usernameField.value && passwordField.value) {
        const username = usernameField.value;
        const password = passwordField.value;
        
        // Check if these are the same credentials we just sent
        if (lastSeenCredentials.username === username && 
            lastSeenCredentials.password === password && 
            lastSeenCredentials.domain === domain) {
            return;
        }
        
        console.log('Grimoire: Form submitted with both username and password');
        
        // Update last seen
        lastSeenCredentials = { username, password, domain };
        
        // Send to background script IMMEDIATELY (before redirect kills us)
        browser.runtime.sendMessage({
            action: "credentials_submitted",
            domain: domain,
            username: username,
            password: password
        }).catch(error => {
            console.error('Grimoire: Error sending to background:', error);
        });
        
        // Clear pending username since we've sent complete credentials
        pendingUsername = null;
        return;
    }
    
    // Case 2: Only username present (multi-step login - step 1)
    if (usernameField && usernameField.value && !passwordField) {
        const username = usernameField.value;
        console.log('Grimoire: Username submitted (multi-step login):', username);
        
        // Store username for when password screen appears
        pendingUsername = username;
        return;
    }
    
    // Case 3: Only password present (multi-step login - step 2)
    if (passwordField && passwordField.value && !usernameField && pendingUsername) {
        const password = passwordField.value;
        const username = pendingUsername;
        
        console.log('Grimoire: Password submitted for pending username:', username);
        
        // Check if these are the same credentials we just sent
        if (lastSeenCredentials.username === username && 
            lastSeenCredentials.password === password && 
            lastSeenCredentials.domain === domain) {
            return;
        }
        
        // Update last seen
        lastSeenCredentials = { username, password, domain };
        
        // Send complete credentials to background script
        browser.runtime.sendMessage({
            action: "credentials_submitted",
            domain: domain,
            username: username,
            password: password
        }).catch(error => {
            console.error('Grimoire: Error sending to background:', error);
        });
        
        // Clear pending username
        pendingUsername = null;
        return;
    }
}

// Monitor form submissions
function setupFormMonitoring() {
    if (listenersAttached) return;
    
    console.log('Grimoire: Form monitoring active');
    listenersAttached = true;
    
    // Listen for form submissions
    document.addEventListener('submit', (event) => {
        captureAndSendCredentials();
    }, true);
    
    // Listen for button clicks (SPA-style logins)
    document.addEventListener('click', (event) => {
        const target = event.target;
        
        if (target.type === 'submit' || 
            target.tagName === 'BUTTON' ||
            target.closest('button')) {
            
            const button = target.tagName === 'BUTTON' ? target : target.closest('button');
            const buttonText = button?.textContent?.toLowerCase() || '';
            
            if (buttonText.includes('log in') || 
                buttonText.includes('sign in') || 
                buttonText.includes('login') ||
                buttonText.includes('signin') ||
                button?.type === 'submit') {
                
                setTimeout(() => {
                    captureAndSendCredentials();
                }, 100);
            }
        }
    }, true);
    
    // Monitor Enter key in password fields
    document.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' && event.target.type === 'password') {
            setTimeout(() => {
                captureAndSendCredentials();
            }, 100);
        }
    }, true);
}

// Watch for login forms appearing dynamically
function watchForLoginForms() {
    console.log('Grimoire: Starting to watch for login forms');
    
    // Check immediately
    const passwordField = findPasswordField();
    if (passwordField) {
        console.log('Grimoire: Password field found immediately');
        setupFormMonitoring();
        return;
    }
    
    // Wait for body if it doesn't exist
    if (!document.body) {
        console.log('Grimoire: Body not ready, waiting...');
        setTimeout(watchForLoginForms, 100);
        return;
    }
    
    console.log('Grimoire: Setting up MutationObserver and interval checker');
    
    // Watch for password fields being added (more aggressive monitoring)
    const observer = new MutationObserver((mutations) => {
        const passwordField = findPasswordField();
        if (passwordField && !listenersAttached) {
            console.log('Grimoire: Password field detected by MutationObserver!');
            setupFormMonitoring();
            // Don't disconnect - keep watching in case forms are removed and re-added
        }
    });
    
    observer.observe(document.body, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ['type', 'class', 'id']
    });
    
    // More aggressive fallback interval check - check every 250ms
    let intervalCount = 0;
    const intervalCheck = setInterval(() => {
        intervalCount++;
        const passwordField = findPasswordField();
        
        if (passwordField && !listenersAttached) {
            console.log('Grimoire: Password field detected by interval checker!');
            console.log('Grimoire: Password field element:', passwordField);
            setupFormMonitoring();
            clearInterval(intervalCheck);
        }
        
        // Log periodically to show we're still checking
        if (intervalCount % 20 === 0) {
            console.log('Grimoire: Still watching for password fields... (' + intervalCount + ' checks)');
        }
        
        if (intervalCount > 120) { // Check for 30 seconds (120 * 250ms)
            console.log('Grimoire: Stopped watching after 30 seconds');
            clearInterval(intervalCheck);
        }
    }, 250);
}

// ===== AUTOFILL & UI FUNCTIONS =====

function fillCredentials(credentials) {
    const { username, password } = credentials;
    
    const usernameFields = document.querySelectorAll(
        'input[type="email"], ' +
        'input[type="text"][name*="user" i], ' +
        'input[type="text"][name*="email" i], ' +
        'input[type="text"][name*="login" i], ' +
        'input[type="text"][id*="user" i], ' +
        'input[type="text"][id*="email" i], ' +
        'input[type="text"][id*="login" i], ' +
        'input[autocomplete="username"], ' +
        'input[autocomplete="email"], ' +
        'input[name="login"], ' +
        'input[id="login_field"]'
    );
    
    const passwordFields = document.querySelectorAll('input[type="password"]');
    
    let filled = false;
    
    if (usernameFields.length > 0 && username) {
        const field = usernameFields[0];
        field.value = username;
        field.setAttribute('value', username);
        field.dispatchEvent(new Event('input', { bubbles: true }));
        field.dispatchEvent(new Event('change', { bubbles: true }));
        field.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true }));
        field.dispatchEvent(new KeyboardEvent('keyup', { bubbles: true }));
        field.dispatchEvent(new Event('blur', { bubbles: true }));
        filled = true;
    }
    
    if (passwordFields.length > 0 && password) {
        const field = passwordFields[0];
        field.value = password;
        field.setAttribute('value', password);
        field.dispatchEvent(new Event('input', { bubbles: true }));
        field.dispatchEvent(new Event('change', { bubbles: true }));
        field.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true }));
        field.dispatchEvent(new KeyboardEvent('keyup', { bubbles: true }));
        field.dispatchEvent(new Event('blur', { bubbles: true }));
        filled = true;
    }
    
    if (filled) {
        console.log('Grimoire: Credentials filled');
        showNotification('Grimoire: Credentials filled');
    } else {
        console.log('Grimoire: No fields found to fill');
        showNotification('No username/password fields found', true);
    }
}

function showNotification(message, isWarning = false) {
    if (document.getElementById('grimoire-notification')) {
        return;
    }
    
    const notification = document.createElement('div');
    notification.id = 'grimoire-notification';
    notification.textContent = message;
    notification.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        background: #0f0f0f;
        color: ${isWarning ? '#a0a0a0' : '#909090'};
        padding: 12px 16px;
        border: 2px solid #2a2a2a;
        border-bottom: 3px solid #000000;
        border-right: 3px solid #000000;
        z-index: 2147483647;
        font-family: 'Courier New', monospace;
        font-size: 11px;
        box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.8);
        animation: slideInRetro 0.2s ease-out;
        letter-spacing: 1px;
    `;
    
    const style = document.createElement('style');
    style.textContent = `
        @keyframes slideInRetro {
            from {
                transform: translateX(400px);
                opacity: 0;
            }
            to {
                transform: translateX(0);
                opacity: 1;
            }
        }
        @keyframes slideOutRetro {
            from {
                transform: translateX(0);
                opacity: 1;
            }
            to {
                transform: translateX(400px);
                opacity: 0;
            }
        }
    `;
    if (!document.getElementById('grimoire-notification-style')) {
        style.id = 'grimoire-notification-style';
        document.head.appendChild(style);
    }
    
    document.body.appendChild(notification);
    
    setTimeout(() => {
        notification.style.animation = 'slideOutRetro 0.2s ease-in';
        setTimeout(() => {
            notification.remove();
        }, 200);
    }, 3000);
}

// ===== INITIALIZATION =====

console.log('Grimoire: Initializing, document.readyState =', document.readyState);

if (document.readyState === 'loading') {
    console.log('Grimoire: Waiting for DOMContentLoaded');
    document.addEventListener('DOMContentLoaded', () => {
        console.log('Grimoire: DOMContentLoaded fired');
        reportStartupState();
        watchForLoginForms();
    });
} else {
    console.log('Grimoire: Document already loaded, starting immediately');
    // Report startup state immediately
    reportStartupState();
    // Then start watching for forms
    watchForLoginForms();
}
