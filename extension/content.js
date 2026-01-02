console.log('Grimoire: Content script loaded');

// Firefox compatibility
if (typeof browser === 'undefined') {
    var browser = chrome;
}

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
});

// Store last seen credentials to avoid duplicates
let lastSeenCredentials = { username: '', password: '', domain: '' };
let listenersAttached = false;

// Capture and send credentials
function captureAndSendCredentials() {
    const domain = window.location.hostname;
    
    const usernameField = document.querySelector(
        'input[type="email"], ' +
        'input[type="text"][name*="user" i], ' +
        'input[type="text"][name*="email" i], ' +
        'input[type="text"][name*="login" i], ' +
        'input[type="text"][id*="user" i], ' +
        'input[type="text"][id*="email" i], ' +
        'input[type="text"][id*="login" i], ' +
        'input[autocomplete="username"], ' +
        'input[autocomplete="email"]'
    );
    
    const passwordField = document.querySelector('input[type="password"]');
    
    if (usernameField && passwordField && usernameField.value && passwordField.value) {
        const username = usernameField.value;
        const password = passwordField.value;
        
        // Check if these are the same credentials we just sent
        if (lastSeenCredentials.username === username && 
            lastSeenCredentials.password === password && 
            lastSeenCredentials.domain === domain) {
            return;
        }
        
        console.log('Grimoire: Capturing credentials for', domain);
        showNotification('Grimoire: Saving credentials...', false);
        
        // Update last seen
        lastSeenCredentials = { username, password, domain };
        
        // Send to background script
        browser.runtime.sendMessage({
            action: "send_credentials",
            domain: domain,
            username: username,
            password: password
        }).then(response => {
            if (response.success) {
                console.log('Grimoire: Credentials saved successfully');
                showNotification('Grimoire: Credentials saved!', false);
            } else {
                console.error('Grimoire: Failed to save:', response.error);
                showNotification('Failed to save credentials', true);
            }
        }).catch(error => {
            console.error('Grimoire: Error sending credentials:', error);
            showNotification('Error saving credentials', true);
        });
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
    // Check immediately
    const passwordField = document.querySelector('input[type="password"]');
    if (passwordField) {
        setupFormMonitoring();
        return;
    }
    
    // Wait for body if it doesn't exist
    if (!document.body) {
        setTimeout(watchForLoginForms, 100);
        return;
    }
    
    // Watch for password fields being added
    const observer = new MutationObserver(() => {
        const passwordField = document.querySelector('input[type="password"]');
        if (passwordField && !listenersAttached) {
            setupFormMonitoring();
            observer.disconnect();
        }
    });
    
    observer.observe(document.body, {
        childList: true,
        subtree: true
    });
    
    // Fallback interval check
    let intervalCount = 0;
    const intervalCheck = setInterval(() => {
        intervalCount++;
        const passwordField = document.querySelector('input[type="password"]');
        if (passwordField && !listenersAttached) {
            setupFormMonitoring();
            clearInterval(intervalCheck);
            observer.disconnect();
        }
        if (intervalCount > 60) {
            clearInterval(intervalCheck);
        }
    }, 500);
}

// Initialize
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', watchForLoginForms);
} else {
    watchForLoginForms();
}

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
