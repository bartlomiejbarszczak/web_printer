// main.js - Core functionality and utilities

// Global state
const AppState = {
    systemStatus: null,
    printers: [],
    scanners: [],
    refreshInterval: null
};

// API helper functions
const API = {
    async get(endpoint) {
        try {
            const response = await fetch(`/api${endpoint}`);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();

            // Handle ApiResponse wrapper
            if (result.success !== undefined) {
                if (!result.success) {
                    throw new Error(result.message || 'API request failed');
                }
                return result.data;
            }

            return result;
        } catch (error) {
            console.error(`GET ${endpoint} failed:`, error);
            throw error;
        }
    },

    async post(endpoint, data) {
        try {
            const response = await fetch(`/api${endpoint}`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(data)
            });

            if (!response.ok) {
                const errorData = await response.json().catch(() => ({}));
                throw new Error(errorData.message || `HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();

            // Handle ApiResponse wrapper
            if (result.success !== undefined) {
                if (!result.success) {
                    throw new Error(result.message || 'API request failed');
                }
                return result.data;
            }

            return result;
        } catch (error) {
            console.error(`POST ${endpoint} failed:`, error);
            throw error;
        }
    },

    async postForm(endpoint, formData) {
        try {
            const response = await fetch(`/api${endpoint}`, {
                method: 'POST',
                body: formData
            });

            if (!response.ok) {
                const errorData = await response.json().catch(() => ({}));
                throw new Error(errorData.message || `HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();

            // Handle ApiResponse wrapper
            if (result.success !== undefined) {
                if (!result.success) {
                    throw new Error(result.message || 'API request failed');
                }
                return result.data;
            }

            return result;
        } catch (error) {
            console.error(`POST ${endpoint} failed:`, error);
            throw error;
        }
    },

    async delete(endpoint) {
        try {
            const response = await fetch(`/api${endpoint}`, {
                method: 'DELETE'
            });

            if (!response.ok) {
                const errorData = await response.json().catch(() => ({}));
                throw new Error(errorData.message || `HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();

            // Handle ApiResponse wrapper
            if (result.success !== undefined) {
                if (!result.success) {
                    throw new Error(result.message || 'API request failed');
                }
                return result.data;
            }

            return result;
        } catch (error) {
            console.error(`DELETE ${endpoint} failed:`, error);
            throw error;
        }
    }
};

// Toast notification system
const Toast = {
    container: null,

    init() {
        this.container = document.getElementById('toast-container') || this.createContainer();
    },

    createContainer() {
        const container = document.createElement('div');
        container.id = 'toast-container';
        container.className = 'toast-container';
        container.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            z-index: 10000;
        `;
        document.body.appendChild(container);
        return container;
    },

    show(message, type = 'info', duration = 5000) {
        if (!this.container) this.init();

        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;

        const icon = this.getIcon(type);
        toast.innerHTML = `
            <div class="toast-content">
                <i class="fas ${icon}"></i>
                <span class="toast-message">${message}</span>
                <button class="toast-close" onclick="this.parentElement.parentElement.remove()">
                    <i class="fas fa-times"></i>
                </button>
            </div>
        `;

        // Add styles
        toast.style.cssText = `
            background: ${this.getColor(type)};
            color: white;
            padding: 12px 16px;
            border-radius: 8px;
            margin-bottom: 10px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
            animation: slideIn 0.3s ease;
            display: flex;
            align-items: center;
            min-width: 300px;
        `;

        this.container.appendChild(toast);

        if (duration > 0) {
            setTimeout(() => {
                if (toast.parentNode) {
                    toast.style.animation = 'slideOut 0.3s ease';
                    setTimeout(() => toast.remove(), 300);
                }
            }, duration);
        }
    },

    getIcon(type) {
        const icons = {
            success: 'fa-check-circle',
            error: 'fa-exclamation-circle',
            warning: 'fa-exclamation-triangle',
            info: 'fa-info-circle'
        };
        return icons[type] || icons.info;
    },

    getColor(type) {
        const colors = {
            success: '#10b981',
            error: '#ef4444',
            warning: '#f59e0b',
            info: '#3b82f6'
        };
        return colors[type] || colors.info;
    },

    success(message, duration) { this.show(message, 'success', duration); },
    error(message, duration) { this.show(message, 'error', duration); },
    warning(message, duration) { this.show(message, 'warning', duration); },
    info(message, duration) { this.show(message, 'info', duration); }
};

// Utility functions
const Utils = {
    formatFileSize(bytes) {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    },

    formatDate(timestamp) {
        const date = new Date(timestamp * 1000);
        return date.toLocaleString();
    },

    formatJobId(id) {
        return id.substring(0, 8) + '...';
    },

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
};

// Modal management
const Modal = {
    show(modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
            modal.style.display = 'flex';
            document.body.style.overflow = 'hidden';

            // Focus management
            const firstInput = modal.querySelector('input, select, textarea, button');
            if (firstInput) firstInput.focus();
        }
    },

    hide(modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
            modal.style.display = 'none';
            document.body.style.overflow = '';
        }
    },

    hideAll() {
        const modals = document.querySelectorAll('.modal');
        modals.forEach(modal => {
            modal.style.display = 'none';
        });
        document.body.style.overflow = '';
    }
};

// System status management
async function updateSystemStatus() {
    try {
        const status = await API.get('/system/status');
        AppState.systemStatus = status;

        // Update status indicators
        updateStatusIndicator('cups-status', status.cups_available);
        updateStatusIndicator('sane-status', status.sane_available);

        // Update dashboard stats if on main page
        if (window.location.pathname === '/') {
            updateDashboardStats(status);
            await updateRecentActivity();
        }

    } catch (error) {
        console.error('Failed to update system status:', error);
        Toast.error('Failed to update system status');
    }
}

function updateStatusIndicator(elementId, isAvailable) {
    const indicator = document.getElementById(elementId);
    if (indicator) {
        const circle = indicator.querySelector('i');
        circle.className = isAvailable ? 'fas fa-circle status-online' : 'fas fa-circle status-offline';
        indicator.title = isAvailable ? 'Service Available' : 'Service Unavailable';
    }
}

function updateDashboardStats(status) {
    const elements = {
        'active-prints': status.active_print_jobs || 0,
        'active-scans': status.active_scan_jobs || 0,
        'total-printers': AppState.printers.length,
        'total-scanners': AppState.scanners.length,
        'disk-space': status.disk_space_mb ? `${status.disk_space_mb} MB` : 'Unknown',
        'uptime': status.uptime_str || 'Unknown'
    };

    Object.entries(elements).forEach(([id, value]) => {
        const element = document.getElementById(id);
        if (element) element.textContent = value;
    });
}


// Load initial data
async function loadInitialData() {
    try {
        await updateSystemStatus();

        // Load printers and scanners
        if (AppState.systemStatus?.cups_available) {
            AppState.printers = await API.get('/printers');
        }

        if (AppState.systemStatus?.sane_available) {
            AppState.scanners = await API.get('/scanners');
        }

        // Update UI if on dashboard
        if (window.location.pathname === '/') {
            updateDashboardStats(AppState.systemStatus);
            await updateRecentActivity();
        }

    } catch (error) {
        console.error('Failed to load initial data:', error);
        Toast.error('Failed to load initial data');
    }
}

// Update recent activity
async function updateRecentActivity() {
    try {
        const recentJobs = await API.get('/system/get-recent');
        displayRecentActivity(recentJobs);
    } catch (error) {
        console.error('Failed to load recent activity:', error);
    }
}

function displayRecentActivity(jobs) {
    const container = document.getElementById('recent-activity');
    if (!container) return;

    if (!jobs || jobs.length === 0) {
        container.innerHTML = `
            <div class="activity-placeholder">
                <i class="fas fa-clock"></i>
                <p>No recent activity</p>
            </div>
        `;
        return;
    }

    container.innerHTML = jobs.map(job => {
        const isPrint = job.Print !== undefined;
        const jobData = isPrint ? job.Print : job.Scan;
        const icon = isPrint ? 'print' : 'scanner';
        const type = isPrint ? 'Print' : 'Scan';
        const name = truncateFilename(isPrint ? jobData.filename : (jobData.output_filename || 'Scan'), 20);

        return `
            <div class="activity-item">
                <div class="activity-icon ${icon}">
<!--                    <i class="fas fa-${icon}"></i>-->
                    <i class="fas fa-print"></i>
                </div>
                <div class="activity-content">
                    <div class="activity-title">${type}: ${name}</div>
                    <div class="activity-time">${formatActivityTime(jobData.completed_at || jobData.created_at)}</div>
                </div>
                <span class="status-badge status-${jobData.status.toLowerCase()}">
                    ${jobData.status}
                </span>
            </div>
        `;
    }).join('');
}

function truncateFilename(filename, maxLength = 20) {
    if (!filename || filename.length <= maxLength) return filename;

    const ext = filename.split('.').pop();
    const name = filename.substring(0, filename.lastIndexOf('.'));

    return name.substring(0, maxLength - ext.length - 4) + '...'
}

function formatActivityTime(timestamp) {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins} min ago`;
    if (diffHours < 24) return `${diffHours} hours ago`;
    return date.toLocaleDateString();
}

// Printer maintenance functions
async function performNozzleCheck() {
    const btn = document.getElementById('nozzle-check-btn');
    if (!btn) return;

    const originalContent = btn.innerHTML;
    btn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Checking...';
    btn.disabled = true;

    try {
        const result = await API.post('/system/nozzle/check', {});

        if (result === 'true' || result === true) {
            Toast.success('Nozzle check completed successfully! Check your printer output.');
        } else {
            Toast.warning('Nozzle check command sent, but status unclear. Check your printer.');
        }
    } catch (error) {
        Toast.error(`Nozzle check failed: ${error.message}`);
    } finally {
        btn.innerHTML = originalContent;
        btn.disabled = false;
    }
}

async function performNozzleClean() {
    if (!confirm('This will clean the printer nozzles. Continue?')) return;

    const btn = document.getElementById('nozzle-clean-btn');
    if (!btn) return;

    const originalContent = btn.innerHTML;
    btn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Cleaning...';
    btn.disabled = true;

    try {
        const result = await API.post('/system/nozzle/clean', {});

        if (result === 'true' || result === true) {
            Toast.success('Nozzle cleaning completed successfully!');
        } else {
            Toast.warning('Nozzle cleaning command sent, but status unclear.');
        }
    } catch (error) {
        Toast.error(`Nozzle cleaning failed: ${error.message}`);
    } finally {
        btn.innerHTML = originalContent;
        btn.disabled = false;
    }
}

// Print dialog functions (for dashboard)
function showPrintDialog() {
    const modal = document.getElementById('print-modal');
    if (modal) {
        // Populate printer dropdown
        populatePrinterSelect('print-printer');
        Modal.show('print-modal');
    }
}

function closePrintDialog() {
    Modal.hide('print-modal');
    document.getElementById('print-form')?.reset();
}

function populatePrinterSelect(selectId) {
    const select = document.getElementById(selectId);
    if (!select) return;

    select.innerHTML = '<option value="">Default Printer</option>';

    AppState.printers.forEach(printer => {
        const option = document.createElement('option');
        option.value = printer.name;
        option.textContent = `${printer.name}${printer.is_default ? ' (Default)' : ''}`;
        if (printer.is_default) option.selected = true;
        select.appendChild(option);
    });
}

// Scan dialog functions (for dashboard)
function showScanDialog() {
    const modal = document.getElementById('scan-modal');
    if (modal) {
        // Populate scanner dropdown
        populateScannerSelect('scan-scanner');
        Modal.show('scan-modal');
    }
}

function closeScanDialog() {
    Modal.hide('scan-modal');
    document.getElementById('scan-form')?.reset();
}

function populateScannerSelect(selectId) {
    const select = document.getElementById(selectId);
    if (!select) return;

    select.innerHTML = '<option value="">Select Scanner</option>';

    AppState.scanners.forEach(scanner => {
        const option = document.createElement('option');
        option.value = scanner.name; // Keep the actual device name for API
        // Display human-readable vendor and model
        const displayName = `${scanner.vendor} ${scanner.model}`;
        option.textContent = displayName;
        select.appendChild(option);
    });
}

// Event handlers for dashboard forms
function setupDashboardForms() {
    // Print form handler
    const printForm = document.getElementById('print-form');
    if (printForm) {
        printForm.addEventListener('submit', async (e) => {
            e.preventDefault();

            const formData = new FormData(printForm);

            // Remove pages field if it's empty to let the printer print all pages
            const pagesValue = formData.get('pages');
            if (!pagesValue || pagesValue.trim() === '') {
                formData.delete('pages');
            }

            try {
                const result = await API.postForm('/print', formData);
                Toast.success('Print job submitted successfully');
                closePrintDialog();

                // Redirect to print page to see job
                setTimeout(() => {
                    window.location.href = '/print';
                }, 1000);

            } catch (error) {
                Toast.error(error.message);
            }
        });
    }

    // Scan form handler
    const scanForm = document.getElementById('scan-form');
    if (scanForm) {
        scanForm.addEventListener('submit', async (e) => {
            e.preventDefault();

            const formData = new FormData(scanForm);
            const scanData = Object.fromEntries(formData);

            // Convert numeric fields
            scanData.resolution = parseInt(scanData.resolution);
            if (scanData.brightness) scanData.brightness = parseInt(scanData.brightness);
            if (scanData.contrast) scanData.contrast = parseInt(scanData.contrast);

            try {
                const result = await API.post('/scan', scanData);
                Toast.success('Scan job started successfully');
                closeScanDialog();

                // Redirect to scan page to see job
                setTimeout(() => {
                    window.location.href = '/scan';
                }, 1000);

            } catch (error) {
                Toast.error(error.message);
            }
        });
    }
}

// Initialize everything when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    // Initialize toast system
    Toast.init();

    // Add CSS for toast animations if not present
    if (!document.querySelector('#toast-styles')) {
        const style = document.createElement('style');
        style.id = 'toast-styles';
        style.textContent = `
            @keyframes slideIn {
                from { transform: translateX(100%); opacity: 0; }
                to { transform: translateX(0); opacity: 1; }
            }
            @keyframes slideOut {
                from { transform: translateX(0); opacity: 1; }
                to { transform: translateX(100%); opacity: 0; }
            }
            .toast-content {
                display: flex;
                align-items: center;
                gap: 8px;
                width: 100%;
            }
            .toast-message {
                flex: 1;
            }
            .toast-close {
                background: none;
                border: none;
                color: inherit;
                cursor: pointer;
                padding: 4px;
            }
            .status-online { color: #10b981; }
            .status-offline { color: #ef4444; }
        `;
        document.head.appendChild(style);
    }

    // Setup modal click handlers
    document.addEventListener('click', (e) => {
        if (e.target.classList.contains('modal')) {
            Modal.hideAll();
        }
    });

    // Setup escape key handler
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            Modal.hideAll();
        }
    });

    // Load initial data
    loadInitialData();

    // Setup dashboard forms if on main page
    if (window.location.pathname === '/') {
        setupDashboardForms();
    }

    // Start periodic updates
    AppState.refreshInterval = setInterval(updateSystemStatus, 5000); // Every 5 seconds
});

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    if (AppState.refreshInterval) {
        clearInterval(AppState.refreshInterval);
    }
});