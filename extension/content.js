// content.js - Runs on web pages to fill in credentials

console.log('Grimoire content script loaded');

// Listen for messages from background script
browser.runtime.onMessage.addListener((message, sender, sendResponse) => {
    console.log('Grimoire received message:', message);
    
    if (message.action === "autofill" && message.credentials) {
        try {
            fillCredentials(message.credentials);
            sendResponse({ success: true });
        } catch (error) {
            console.error('Grimoire fill error:', error);
            sendResponse({ success: false, error: error.message });
        }
        return true; // Keep channel open for async response
    }
});

function fillCredentials(credentials) {
    const { username, password } = credentials;
    
    console.log('Grimoire attempting to fill credentials');
    
    // Find username/email fields
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
    
    // Find password fields
    const passwordFields = document.querySelectorAll(
        'input[type="password"]'
    );
    
    console.log('Found username fields:', usernameFields.length);
    console.log('Found password fields:', passwordFields.length);
    
    let filled = false;
    
    // Fill in the first matching field of each type
    if (usernameFields.length > 0 && username) {
        const field = usernameFields[0];
        console.log('Filling username field:', field);
        
        // Set value
        field.value = username;
        field.setAttribute('value', username);
        
        // Trigger events for frameworks that listen to them (React, Vue, etc.)
        field.dispatchEvent(new Event('input', { bubbles: true }));
        field.dispatchEvent(new Event('change', { bubbles: true }));
        field.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true }));
        field.dispatchEvent(new KeyboardEvent('keyup', { bubbles: true }));
        field.dispatchEvent(new Event('blur', { bubbles: true }));
        
        filled = true;
    }
    
    if (passwordFields.length > 0 && password) {
        const field = passwordFields[0];
        console.log('Filling password field:', field);
        
        // Set value
        field.value = password;
        field.setAttribute('value', password);
        
        // Trigger events
        field.dispatchEvent(new Event('input', { bubbles: true }));
        field.dispatchEvent(new Event('change', { bubbles: true }));
        field.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true }));
        field.dispatchEvent(new KeyboardEvent('keyup', { bubbles: true }));
        field.dispatchEvent(new Event('blur', { bubbles: true }));
        
        filled = true;
    }
    
    // Show notification if we filled anything
    if (filled) {
        console.log('Grimoire successfully filled credentials');
        showNotification('🔐 Grimoire: Credentials filled');
    } else {
        console.warn('Grimoire could not find fields to fill');
        showNotification('⚠️ No username/password fields found', true);
    }
}

function showNotification(message, isWarning = false) {
    // Check if notification already exists
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
        background: ${isWarning ? '#ff9500' : '#0060df'};
        color: white;
        padding: 15px 20px;
        border-radius: 8px;
        z-index: 2147483647;
        font-family: system-ui, -apple-system, sans-serif;
        font-size: 14px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.3);
        animation: slideIn 0.3s ease-out;
    `;
    
    // Add animation
    const style = document.createElement('style');
    style.textContent = `
        @keyframes slideIn {
            from {
                transform: translateX(400px);
                opacity: 0;
            }
            to {
                transform: translateX(0);
                opacity: 1;
            }
        }
        @keyframes slideOut {
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
    
    // Remove after 3 seconds with animation
    setTimeout(() => {
        notification.style.animation = 'slideOut 0.3s ease-in';
        setTimeout(() => {
            notification.remove();
        }, 300);
    }, 3000);
}
