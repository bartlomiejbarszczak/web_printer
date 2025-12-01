// main.js - Core functionality and utilities

// Global state
const AppState = {
    systemStatus: null,
    printers: [],
    scanners: [],
    uptime_ms: 0,
};

let eventSource = null;
let currentJobQueue = [];
let queueTimeUpdateInterval = null;
let uptimeUpdateInterval = null;


// API HELPER
const API = {
    request: async function (endpoint, options = {}) {
        try {
            const response = await fetch(`/api${endpoint}`, options);

            if (!response.ok) {
                const errorData = await response.json().catch(() => ({}));
                throw new Error(errorData.message || `HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();

            if (result.success !== undefined) {
                if (!result.success) {
                    throw new Error(result.message || 'API request failed');
                }
                return result.data;
            }

            return result;
        } catch (error) {
            console.error(`${options.method || 'GET'} ${endpoint} failed:`, error);
            throw error;
        }
    },

    get(endpoint) {
        return this.request(endpoint);
    },

    post(endpoint, data) {
        return this.request(endpoint, {
            method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(data)
        });
    },

    postForm(endpoint, formData) {
        return this.request(endpoint, {
            method: 'POST', body: formData
        });
    },

    delete(endpoint) {
        return this.request(endpoint, {method: 'DELETE'});
    }
};


// TOAST NOTIFICATION SYSTEM
const Toast = {
    container: null,

    init() {
        if (!this.container) {
            this.container = this.createContainer();
            this.injectStyles();
        }
    },

    createContainer() {
        const container = document.createElement('div');
        container.id = 'toast-container';
        container.className = 'toast-container';
        document.body.appendChild(container);
        return container;
    },

    injectStyles() {
        if (document.querySelector('#toast-styles')) return;

        const style = document.createElement('style');
        style.id = 'toast-styles';
        style.textContent = `
            .toast-container {
                position: fixed;
                top: 20px;
                right: 20px;
                z-index: 10000;
            }
            @keyframes slideIn {
                from { transform: translateX(100%); opacity: 0; }
                to { transform: translateX(0); opacity: 1; }
            }
            @keyframes slideOut {
                from { transform: translateX(0); opacity: 1; }
                to { transform: translateX(100%); opacity: 0; }
            }
            .toast {
                padding: 12px 16px;
                border-radius: 8px;
                margin-bottom: 10px;
                box-shadow: 0 4px 12px rgba(0,0,0,0.15);
                animation: slideIn 0.3s ease;
                min-width: 300px;
                color: white;
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
    },

    show(message, type = 'info', duration = 5000) {
        if (!this.container) this.init();

        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;
        toast.style.background = this.getColor(type);

        toast.innerHTML = `
            <div class="toast-content">
                <i class="fas ${this.getIcon(type)}"></i>
                <span class="toast-message">${message}</span>
                <button class="toast-close" onclick="this.parentElement.parentElement.remove()">
                    <i class="fas fa-times"></i>
                </button>
            </div>
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
            success: '#10b981', error: '#ef4444', warning: '#f59e0b', info: '#3b82f6'
        };
        return colors[type] || colors.info;
    },

    success(message, duration) {
        this.show(message, 'success', duration);
    }, error(message, duration) {
        this.show(message, 'error', duration);
    }, warning(message, duration) {
        this.show(message, 'warning', duration);
    }, info(message, duration) {
        this.show(message, 'info', duration);
    }
};


// UTILITY FUNCTIONS
const Utils = {
    formatFileSize(bytes) {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
    },

    // formatDate(timestamp) {
    //     return new Date(timestamp * 1000).toLocaleString();
    // },
    //
    // formatJobId(id) {
    //     return `${id.substring(0, 8)}...`;
    // },
    //
    // debounce(func, wait) {
    //     let timeout;
    //     return function executedFunction(...args) {
    //         clearTimeout(timeout);
    //         timeout = setTimeout(() => func(...args), wait);
    //     };
    // },

    truncateFilename(filename, maxLength = 20) {
        if (!filename || filename.length <= maxLength) return filename;

        const lastDotIndex = filename.lastIndexOf('.');
        if (lastDotIndex === -1) {
            return `${filename.substring(0, maxLength - 3)}...`;
        }

        const ext = filename.substring(lastDotIndex);
        const name = filename.substring(0, lastDotIndex);
        const availableLength = maxLength - ext.length - 3;

        return `${name.substring(0, availableLength)}...${ext}`;
    },

    formatDuration(ms) {
        const seconds = Math.floor(ms / 1000);
        const minutes = Math.floor(seconds / 60);
        const hours = Math.floor(minutes / 60);
        const days = Math.floor(hours / 24);

        if (days > 0) {
            return `${days}d ${hours % 24}h ${minutes % 60}m`;
        } else if (hours > 0) {
            return `${hours}h ${minutes % 60}m`;
        } else if (minutes > 0) {
            return `${minutes}m ${seconds % 60}s`;
        } else {
            return `${seconds}s`;
        }
    },

    formatActivityTime(timestamp) {
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
};


// MODAL MANAGEMENT
const Modal = {
    show(modalId) {
        const modal = document.getElementById(modalId);
        if (!modal) return;

        modal.style.display = 'flex';
        document.body.style.overflow = 'hidden';

        // Focus first interactive element
        const firstInput = modal.querySelector('input, select, textarea, button');
        firstInput?.focus();
    },

    hide(modalId) {
        const modal = document.getElementById(modalId);
        if (!modal) return;

        modal.style.display = 'none';
        document.body.style.overflow = '';
    },

    hideAll() {
        document.querySelectorAll('.modal').forEach(modal => {
            modal.style.display = 'none';
        });
        document.body.style.overflow = '';
    }
};


// SYSTEM STATUS
async function updateSystemStatus() {
    try {
        const status = await API.get('/system/status');
        AppState.systemStatus = status;
        AppState.uptime_ms = status.uptime_ms;

        updateStatusIndicator('cups-status', status.cups_available);
        updateStatusIndicator('sane-status', status.sane_available);

        if (window.location.pathname === '/') {
            updateDashboardStats(status);
        }
    } catch (error) {
        console.error('Failed to update system status:', error);
        Toast.error('Failed to update system status');
    }
}

function updateStatusIndicator(elementId, isAvailable) {
    const indicator = document.getElementById(elementId);
    if (!indicator) return;

    const circle = indicator.querySelector('i');
    if (circle) {
        circle.className = `fas fa-circle ${isAvailable ? 'status-online' : 'status-offline'}`;
    }
    indicator.title = isAvailable ? 'Service Available' : 'Service Unavailable';
}

function updateDashboardStats(status) {
    const updates = {
        'active-prints': status.active_print_jobs || 0,
        'active-scans': status.active_scan_jobs || 0,
        'total-printers': AppState.printers.length,
        'total-scanners': AppState.scanners.length,
        'disk-space': status.disk_space_mb ? `${status.disk_space_mb} MB` : 'Unknown',
        'uptime': Utils.formatDuration(status.uptime_ms) || 'Unknown'
    };

    Object.entries(updates).forEach(([id, value]) => {
        const element = document.getElementById(id);
        if (element) element.textContent = value;
    });

    startUptimeUpdates();
}


// DATA LOADING
async function loadInitialData() {
    try {
        await updateSystemStatus();

        if (AppState.systemStatus?.cups_available) {
            AppState.printers = await API.get('/printers');
        }

        if (AppState.systemStatus?.sane_available) {
            AppState.scanners = await API.get('/scanners');
        }
    } catch (error) {
        console.error('Failed to load initial data:', error);
        Toast.error('Failed to load initial data');
    }
}

async function updateRecentActivity() {
    try {
        const recentJobs = await API.get('/system/recent');
        displayRecentActivity(recentJobs);
    } catch (error) {
        console.error('Failed to load recent activity:', error);
    }
}

function displayRecentActivity(jobs) {
    const container = document.getElementById('recent-activity');
    if (!container) return;

    if (!jobs?.length) {
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
        const type = isPrint ? 'Print' : 'Scan';
        const name = Utils.truncateFilename(isPrint ? jobData.filename : (jobData.output_filename || 'Scan'), 20);

        return `
            <div class="activity-item">
                <div class="activity-icon ${isPrint ? 'print' : 'scan'}">
                    <i class="fas fa-print"></i>
                </div>
                <div class="activity-content">
                    <div class="activity-title">${type}: ${name}</div>
                    <div class="activity-time">${Utils.formatActivityTime(jobData.completed_at || jobData.created_at)}</div>
                </div>
                <span class="status-badge status-${jobData.status.toLowerCase()}">
                    ${jobData.status}
                </span>
            </div>
        `;
    }).join('');
}


// PRINTER MAINTENANCE
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


// PRINT DIALOG
function showPrintDialog() {
    populatePrinterSelect('print-printer');
    Modal.show('print-modal');
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

// SCAN DIALOG
function showScanDialog() {
    populateScannerSelect('scan-scanner');
    Modal.show('scan-modal');
}

function closeScanDialog() {
    Modal.hide('scan-modal');
    const form = document.getElementById('scan-form');
    if (form) {
        form.reset(); // Reset range value displays
        const brightnessValue = document.getElementById('brightness-value');
        const contrastValue = document.getElementById('contrast-value');
        if (brightnessValue) brightnessValue.textContent = '0';
        if (contrastValue) contrastValue.textContent = '0';
    }
}

function populateScannerSelect(selectId) {
    const select = document.getElementById(selectId);
    if (!select) return;

    select.innerHTML = '<option value="">Select Scanner</option>';

    AppState.scanners.forEach(scanner => {
        const option = document.createElement('option');
        option.value = scanner.name;
        option.textContent = `${scanner.vendor} ${scanner.model}`;
        select.appendChild(option);
    });
}

function setupScanRangeInputs() {
    const ranges = [{input: 'scan-brightness', display: 'brightness-value'}, {
        input: 'scan-contrast', display: 'contrast-value'
    }];

    ranges.forEach(({input, display}) => {
        const rangeInput = document.getElementById(input);
        const displayElement = document.getElementById(display);

        if (rangeInput && displayElement) {
            rangeInput.addEventListener('input', (e) => {
                displayElement.textContent = e.target.value;
                updateSliderBackground(e.target);
            });
            updateSliderBackground(rangeInput);
        }
    });
}

function updateSliderBackground(slider) {
    const min = slider.min || 0;
    const max = slider.max || 100;
    const value = slider.value;
    const percentage = ((value - min) / (max - min)) * 100;
    slider.style.background = `linear-gradient(to right, var(--primary-light) 0%, var(--primary-light) ${percentage}%, var(--bg-tertiary) ${percentage}%, var(--bg-tertiary) 100%)`;
}


// FORM HANDLERS
function setupDashboardForms() {
    setupPrintForm();
    setupScanForm();
    setupScanRangeInputs();
}

function setupPrintForm() {
    const printForm = document.getElementById('print-form');
    if (!printForm) return;

    printForm.addEventListener('submit', async (e) => {
        e.preventDefault();

        const formData = new FormData(printForm);

        // Remove pages field if empty to print all pages
        const pagesValue = formData.get('pages');
        if (!pagesValue?.trim()) {
            formData.delete('pages');
        }

        try {
            await API.postForm('/print', formData);
            Toast.success('Print job submitted successfully');
            closePrintDialog();
        } catch (error) {
            Toast.error(error.message);
        }
    });
}

function setupScanForm() {
    const scanForm = document.getElementById('scan-form');
    if (!scanForm) return;

    scanForm.addEventListener('submit', async (e) => {
        e.preventDefault();

        const formData = new FormData(scanForm);
        const scanData = Object.fromEntries(formData);

        // Convert numeric fields
        scanData.resolution = parseInt(scanData.resolution);
        if (scanData.brightness) scanData.brightness = parseInt(scanData.brightness);
        if (scanData.contrast) scanData.contrast = parseInt(scanData.contrast);

        // Handle filename
        if (!scanData.filename?.trim()) {
            delete scanData.filename;
        } else {
            scanData.filename = scanData.filename.trim();
        }

        try {
            await API.post('/scan', scanData);
            Toast.success('Scan job started successfully');
            closeScanDialog();
        } catch (error) {
            Toast.error(error.message);
        }
    });
}


// JOB QUEUE
function displayJobQueue(jobs) {
    currentJobQueue = jobs;
    updateQueueCount(jobs.length);

    const container = document.getElementById('queue-list');
    if (!container) return;

    if (!jobs?.length) {
        container.innerHTML = `
            <div class="queue-placeholder">
                <i class="fas fa-check-circle"></i>
                <p>No jobs in queue</p>
            </div>
        `;
        return;
    }

    container.innerHTML = jobs.map((item, index) => {
        const isPrint = item.Print !== undefined;
        const job = isPrint ? item.Print : item.Scan;
        const type = isPrint ? 'print' : 'scan';
        const isProcessing = ['processing', 'printing', 'scanning'].includes(job.status.toLowerCase());
        const filename = isPrint ? job.filename : (job.output_filename || 'Scan');

        const waitTime = calculateWaitTime(job);
        const processTime = calculateProcessingTime(job);

        return `
            <div class="queue-item ${isProcessing ? 'processing' : 'queued'}" data-job-index="${index}">
                <div class="queue-position ${isProcessing ? 'processing' : ''}">
                    ${isProcessing ? '▶' : `#${index}`}
                </div>
                <div class="queue-job-icon ${type} ${isProcessing ? 'processing' : ''}">
                    <i class="fas fa-print"></i>
                </div>
                <div class="queue-job-info">
                    <div class="queue-job-title">${isPrint ? 'Print' : 'Scan'}: ${filename}</div>
                    <div class="queue-job-subtitle">
                        ${isProcessing ? '<span class="processing-indicator"><i class="fas fa-spinner fa-spin"></i> Processing</span>' : `Status: ${job.status}`}
                    </div>
                </div>
                <div class="queue-job-time">
                    ${isProcessing ? `<span class="queue-time-label">Processing</span>
                           <span class="queue-time-value processing">${processTime}</span>` : `<span class="queue-time-label">Waiting</span>
                           <span class="queue-time-value waiting">${waitTime}</span>`}
                </div>
            </div>
        `;
    }).join('');

    startQueueTimeUpdates();
}

function updateQueueCount(count) {
    const countElement = document.getElementById('queue-count');
    if (countElement) {
        countElement.textContent = count;
        countElement.style.display = count > 0 ? 'inline-block' : 'none';
    }
}

function calculateWaitTime(job) {
    const now = new Date();
    const createdAt = new Date(job.created_at);
    return Utils.formatDuration(now - createdAt);
}

function calculateProcessingTime(job) {
    if (!job.started_at) return '0s';
    const now = new Date();
    const startedAt = new Date(job.started_at);
    return Utils.formatDuration(now - startedAt);
}

function startQueueTimeUpdates() {
    if (queueTimeUpdateInterval) {
        clearInterval(queueTimeUpdateInterval);
    }

    queueTimeUpdateInterval = setInterval(() => {
        const container = document.getElementById('queue-list');
        if (!container || !currentJobQueue?.length) return;

        const queueItems = container.querySelectorAll('.queue-item');

        queueItems.forEach((item) => {
            const jobIndex = parseInt(item.getAttribute('data-job-index'));
            const jobData = currentJobQueue[jobIndex];
            if (!jobData) return;

            const isPrint = jobData.Print !== undefined;
            const job = isPrint ? jobData.Print : jobData.Scan;
            const isProcessing = ['processing', 'printing', 'scanning'].includes(job.status.toLowerCase());

            const timeValueEl = item.querySelector('.queue-time-value');
            if (timeValueEl) {
                timeValueEl.textContent = isProcessing ? calculateProcessingTime(job) : calculateWaitTime(job);
            }
        });
    }, 1000);
}


// UPTIME UPDATES
function startUptimeUpdates() {
    if (uptimeUpdateInterval) {
        clearInterval(uptimeUpdateInterval);
    }

    uptimeUpdateInterval = setInterval(() => {
        const el = document.getElementById('uptime');
        if (el) {
            AppState.uptime_ms += 1000;
            el.textContent = Utils.formatDuration(AppState.uptime_ms);
        }
    }, 1000);
}


// SERVER-SENT EVENTS (SSE)
function initializeSSE() {
    if (eventSource) {
        eventSource.close();
    }

    eventSource = new EventSource("/api/events/stream");

    eventSource.addEventListener('open', () => {
        console.log("SSE connection established");
    });

    eventSource.addEventListener('message', (event) => {
        try {
            const data = JSON.parse(event.data);
            console.log("Received queue update:", data);
            handleSSEMessage(data);
        } catch (error) {
            console.error('Failed to parse SSE message:', error);
        }
    });

    eventSource.addEventListener('error', (error) => {
        console.log("SSE connection error:", error);
        setTimeout(() => {
            console.log('Reconnecting SSE...');
            initializeSSE();
        }, 5000);
    });
}

async function handleSSEMessage(data) {
    await updateRecentActivity();

    switch (data.type) {
        case 'queue_update':
            if (window.location.pathname === '/') {
                displayJobQueue(data.queue);
            }
            break;

        case 'status_update':
            updateStatusFromSSE(data.status);
            break;

        case 'recent_activity':
            console.log("Recent activity update not implemented");
            break;

        default:
            console.log('Unknown SSE message type:', data.type);
    }
}

function updateStatusFromSSE(status) {
    const updates = {
        'active-prints': status.active_prints,
        'active-scans': status.active_scans,
        'disk-space': status.disk_space_mb !== undefined ? `${status.disk_space_mb} MB` : null
    };

    Object.entries(updates).forEach(([id, value]) => {
        if (value !== undefined && value !== null) {
            const el = document.getElementById(id);
            if (el) el.textContent = value;
        }
    });
}


// INITIALIZATION & CLEANUP
document.addEventListener('DOMContentLoaded', () => {
    Toast.init();

    // Setup modal handlers
    document.addEventListener('click', (e) => {
        if (e.target.classList.contains('modal')) {
            Modal.hideAll();
        }
    });

    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            Modal.hideAll();
        }
    });

    // Load initial data
    loadInitialData();

    // Setup dashboard if on main page
    if (window.location.pathname === '/') {
        setupDashboardForms();
    }

    // Initialize SSE
    initializeSSE();
});

window.addEventListener('beforeunload', () => {
    if (queueTimeUpdateInterval) clearInterval(queueTimeUpdateInterval);
    if (uptimeUpdateInterval) clearInterval(uptimeUpdateInterval);
    if (eventSource) eventSource.close();
});