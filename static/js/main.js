// Main JavaScript file for Print/Scan Manager
// Provides core functionality and utilities

class PrintScanManager {
    constructor() {
        this.baseUrl = window.location.origin;
        this.init();
    }

    async init() {
        await this.loadSystemStatus();
        this.setupEventListeners();
        this.setupNavigation();
        this.startStatusPolling();
    }

    // API helper methods
    async apiCall(endpoint, options = {}) {
        try {
            const response = await fetch(`${this.baseUrl}/api${endpoint}`, {
                headers: {
                    'Content-Type': 'application/json',
                    ...options.headers
                },
                ...options
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const contentType = response.headers.get('content-type');
            if (contentType && contentType.includes('application/json')) {
                return await response.json();
            }
            return response;
        } catch (error) {
            console.error('API call failed:', error);
            this.showNotification('API call failed: ' + error.message, 'error');
            throw error;
        }
    }

    async uploadFile(endpoint, formData) {
        try {
            const response = await fetch(`${this.baseUrl}/api${endpoint}`, {
                method: 'POST',
                body: formData
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            return await response.json();
        } catch (error) {
            console.error('File upload failed:', error);
            this.showNotification('File upload failed: ' + error.message, 'error');
            throw error;
        }
    }

    // System status management
    async loadSystemStatus() {
        try {
            const status = await this.apiCall('/system/status');
            this.updateSystemStatusUI(status);
        } catch (error) {
            console.error('Failed to load system status:', error);
        }
    }

    updateSystemStatusUI(status) {
        const statusElement = document.getElementById('system-status');
        if (statusElement) {
            const cupsStatus = status.cups_available ? 'online' : 'offline';
            const saneStatus = status.sane_available ? 'online' : 'offline';

            statusElement.innerHTML = `
                <div class="status-item">
                    <span class="status-label">CUPS:</span>
                    <span class="status-indicator ${cupsStatus}">${cupsStatus}</span>
                </div>
                <div class="status-item">
                    <span class="status-label">SANE:</span>
                    <span class="status-indicator ${saneStatus}">${saneStatus}</span>
                </div>
            `;
        }
    }

    // Navigation management
    setupNavigation() {
        const navLinks = document.querySelectorAll('.nav-link');
        const currentPath = window.location.pathname;

        navLinks.forEach(link => {
            if (link.getAttribute('href') === currentPath) {
                link.classList.add('active');
            }

            link.addEventListener('click', (e) => {
                // Remove active class from all links
                navLinks.forEach(l => l.classList.remove('active'));
                // Add active class to clicked link
                e.target.classList.add('active');
            });
        });
    }

    // Notification system
    showNotification(message, type = 'info') {
        const notification = document.createElement('div');
        notification.className = `notification ${type}`;
        notification.innerHTML = `
            <span class="notification-message">${message}</span>
            <button class="notification-close" onclick="this.parentElement.remove()">×</button>
        `;

        const container = document.getElementById('notifications') || this.createNotificationContainer();
        container.appendChild(notification);

        // Auto-remove after 5 seconds
        setTimeout(() => {
            if (notification.parentElement) {
                notification.remove();
            }
        }, 5000);
    }

    createNotificationContainer() {
        const container = document.createElement('div');
        container.id = 'notifications';
        container.className = 'notifications-container';
        document.body.appendChild(container);
        return container;
    }

    // File management utilities
    formatFileSize(bytes) {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    formatDateTime(dateString) {
        const date = new Date(dateString);
        return date.toLocaleString();
    }

    // Progress bar utilities
    showProgressBar(containerId, progress = 0) {
        const container = document.getElementById(containerId);
        if (!container) return;

        let progressBar = container.querySelector('.progress-bar');
        if (!progressBar) {
            progressBar = document.createElement('div');
            progressBar.className = 'progress-bar';
            progressBar.innerHTML = `
                <div class="progress-fill"></div>
                <div class="progress-text">0%</div>
            `;
            container.appendChild(progressBar);
        }

        this.updateProgressBar(progressBar, progress);
    }

    updateProgressBar(progressBar, progress) {
        const fill = progressBar.querySelector('.progress-fill');
        const text = progressBar.querySelector('.progress-text');

        fill.style.width = `${progress}%`;
        text.textContent = `${Math.round(progress)}%`;
    }

    hideProgressBar(containerId) {
        const container = document.getElementById(containerId);
        if (container) {
            const progressBar = container.querySelector('.progress-bar');
            if (progressBar) {
                progressBar.remove();
            }
        }
    }

    // Job status polling
    startStatusPolling() {
        // Poll job statuses every 2 seconds
        setInterval(() => {
            this.updateJobStatuses();
        }, 2000);
    }

    async updateJobStatuses() {
        try {
            // Update print jobs if on print page
            if (window.location.pathname === '/print' && window.printManager) {
                await window.printManager.updateJobList();
            }

            // Update scan jobs if on scan page
            if (window.location.pathname === '/scan' && window.scanManager) {
                await window.scanManager.updateJobList();
            }
        } catch (error) {
            console.error('Failed to update job statuses:', error);
        }
    }

    // Event listeners
    setupEventListeners() {
        // Handle file drag and drop
        document.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = 'copy';
        });

        document.addEventListener('drop', (e) => {
            e.preventDefault();
            const files = Array.from(e.dataTransfer.files);
            const dropZone = e.target.closest('.drop-zone');

            if (dropZone && files.length > 0) {
                this.handleFileDrop(dropZone, files);
            }
        });

        // Handle responsive menu toggle
        const menuToggle = document.querySelector('.menu-toggle');
        const nav = document.querySelector('.nav');

        if (menuToggle && nav) {
            menuToggle.addEventListener('click', () => {
                nav.classList.toggle('active');
            });
        }
    }

    handleFileDrop(dropZone, files) {
        const event = new CustomEvent('filesDropped', {
            detail: { files, dropZone }
        });
        document.dispatchEvent(event);
    }

    // Utility methods
    debounce(func, wait) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
        };
    }

    // Modal management
    showModal(modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
            modal.classList.add('active');
            document.body.classList.add('modal-open');
        }
    }

    hideModal(modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
            modal.classList.remove('active');
            document.body.classList.remove('modal-open');
        }
    }

    // Initialize modal close handlers
    initializeModals() {
        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('modal-backdrop')) {
                e.target.closest('.modal').classList.remove('active');
                document.body.classList.remove('modal-open');
            }

            if (e.target.classList.contains('modal-close')) {
                e.target.closest('.modal').classList.remove('active');
                document.body.classList.remove('modal-open');
            }
        });

        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                const activeModal = document.querySelector('.modal.active');
                if (activeModal) {
                    activeModal.classList.remove('active');
                    document.body.classList.remove('modal-open');
                }
            }
        });
    }
}

// Initialize the application when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.app = new PrintScanManager();
    window.app.initializeModals();
});

// Export for use in other modules
window.PrintScanManager = PrintScanManager;