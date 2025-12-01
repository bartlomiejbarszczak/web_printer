// scan.js - Scan page specific functionality


// PAGE STATE
const ScanPage = {
    jobs: [],
    scanners: [],
    jobsRefreshInterval: null,
    isSubmitting: false
};


// INITIALIZATION
document.addEventListener('DOMContentLoaded', () => {
    if (window.location.pathname === '/scan') {
        initializeScanPage();
    }
});

async function initializeScanPage() {
    await loadScanners();
    await loadScanJobs();
    setupScanForm();
    setupRangeInputs();

    // Auto-refresh jobs every 5 seconds
    ScanPage.jobsRefreshInterval = setInterval(loadScanJobs, 5000);
}


// SCANNERS
async function loadScanners() {
    try {
        ScanPage.scanners = await API.get('/scanners');
        displayScanners();
        populateScannerDropdown();
    } catch (error) {
        console.error('Failed to load scanners:', error);
        showScannersError();
    }
}

function displayScanners() {
    const grid = document.getElementById('scanners-grid');
    if (!grid) return;

    if (!ScanPage.scanners.length) {
        grid.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-scanner"></i>
                <h3>No Scanners Available</h3>
                <p>Check SANE service and scanner connections</p>
            </div>
        `;
        return;
    }

    grid.innerHTML = ScanPage.scanners.map(scanner => {
        const displayName = `${scanner.vendor} ${scanner.model}`;
        const deviceName = scanner.name.length > 30
            ? `${scanner.name.substring(0, 30)}...`
            : scanner.name;

        return `
            <div class="scanner-card available">
                <div class="scanner-icon">
                    <i class="fas fa-scanner"></i>
                </div>
                <div class="scanner-info">
                    <h4>${displayName}</h4>
                    <p class="scanner-type">${scanner.device_type || 'Flatbed Scanner'}</p>
                    <p class="scanner-device" style="font-size: 0.75em; color: #888;" title="${scanner.name}">
                        Device: ${deviceName}
                    </p>
                </div>
                <div class="scanner-actions">
                    <button class="btn btn-sm btn-primary" onclick="quickScan('${scanner.name}')">
                        <i class="fas fa-scanner"></i>
                        Scan
                    </button>
                </div>
            </div>
        `;
    }).join('');
}

function showScannersError() {
    const grid = document.getElementById('scanners-grid');
    if (!grid) return;

    grid.innerHTML = `
        <div class="error-state">
            <i class="fas fa-exclamation-triangle"></i>
            <h3>Failed to Load Scanners</h3>
            <p>Check SANE service status</p>
            <button class="btn btn-secondary" onclick="refreshScanners()">
                <i class="fas fa-refresh"></i>
                Try Again
            </button>
        </div>
    `;
}

function populateScannerDropdown() {
    const select = document.getElementById('scan-scanner');
    if (!select) return;

    select.innerHTML = '<option value="">Select Scanner</option>';

    ScanPage.scanners.forEach(scanner => {
        const option = document.createElement('option');
        option.value = scanner.name;
        option.textContent = `${scanner.vendor} ${scanner.model}`;
        select.appendChild(option);
    });
}

async function refreshScanners() {
    const button = event?.target;
    if (button) {
        const originalContent = button.innerHTML;
        button.innerHTML = '<i class="fas fa-spinner fa-spin"></i>';
        button.disabled = true;

        try {
            await loadScanners();
            Toast.success('Scanners refreshed');
        } catch (error) {
            Toast.error('Failed to refresh scanners');
        } finally {
            button.innerHTML = originalContent;
            button.disabled = false;
        }
    } else {
        await loadScanners();
    }
}

function quickScan(scannerName) {
    showScanDialog();
    const scannerSelect = document.getElementById('scan-scanner');
    if (scannerSelect) {
        scannerSelect.value = scannerName;
    }
}


// SCAN JOBS
async function loadScanJobs() {
    try {
        ScanPage.jobs = await API.get('/scan/jobs');
        displayScanJobs();
    } catch (error) {
        console.error('Failed to load scan jobs:', error);
        showJobsError();
    }
}

function displayScanJobs() {
    const tbody = document.getElementById('scan-jobs-tbody');
    if (!tbody) return;

    if (!ScanPage.jobs.length) {
        tbody.innerHTML = `
            <tr>
                <td colspan="8" class="empty-state">
                    <i class="fas fa-scanner"></i>
                    <h3>No Scan Jobs</h3>
                    <p>Start scanning to see jobs here</p>
                </td>
            </tr>
        `;
        return;
    }

    tbody.innerHTML = ScanPage.jobs.map(job => {
        const status = job.status.toLowerCase();
        const filename = job.output_filename || 'Unnamed';

        return `
            <tr class="job-row job-${status}">
                <td>
                    <span class="filename" title="${filename}">
                        ${Utils.truncateFilename(filename, 25)}
                    </span>
                </td>
                <td>
                    <span class="scanner-name">${ScanHelpers.getScannerDisplayName(job.scanner)}</span>
                </td>
                <td>
                    <span class="format-badge format-${job.format}">${job.format.toUpperCase()}</span>
                </td>
                <td>
                    <span class="resolution">${job.resolution} DPI</span>
                </td>
                <td>
                    <span class="status-badge status-${status}">
                        <i class="fas ${ScanHelpers.getStatusIcon(status)}"></i>
                        ${job.status}
                    </span>
                </td>
                <td>
                    <span class="job-time" title="${new Date(job.created_at).toLocaleString()}">
                        ${Utils.formatActivityTime(job.created_at)}
                    </span>
                </td>
                <td>
                    <span class="file-size">
                        ${job.file_size ? Utils.formatFileSize(job.file_size) : '-'}
                    </span>
                </td>
                <td>
                    <div class="job-actions">
                        ${ScanHelpers.getActionButtons(job)}
                    </div>
                </td>
            </tr>
        `;
    }).join('');
}

function showJobsError() {
    const tbody = document.getElementById('scan-jobs-tbody');
    if (!tbody) return;

    tbody.innerHTML = `
        <tr>
            <td colspan="8" class="error-state">
                <i class="fas fa-exclamation-triangle"></i>
                Failed to load scan jobs
                <button class="btn btn-sm btn-secondary" onclick="refreshScanJobs()">Retry</button>
            </td>
        </tr>
    `;
}

async function refreshScanJobs() {
    await loadScanJobs();
    Toast.info('Scan jobs refreshed');
}


// SCAN HELPERS
const ScanHelpers = {
    getScannerDisplayName(deviceName) {
        const scanner = ScanPage.scanners.find(s => s.name === deviceName);

        if (scanner) {
            return `${scanner.vendor} ${scanner.model}`;
        }

        if (deviceName.includes(':')) {
            const parts = deviceName.split(':');
            return parts[0].toUpperCase();
        }

        return deviceName;
    },

    getStatusIcon(status) {
        const icons = {
            'queued': 'fa-clock',
            'scanning': 'fa-spinner fa-spin',
            'processing': 'fa-cog fa-spin',
            'completed': 'fa-check-circle',
            'failed': 'fa-exclamation-circle',
            'cancelled': 'fa-times-circle'
        };
        return icons[status] || 'fa-question-circle';
    },

    getActionButtons(job) {
        const status = job.status.toLowerCase();
        const actions = [];

        // View details button
        actions.push(`
            <button class="btn btn-sm btn-secondary" onclick="viewScanJobDetails('${job.id}')" title="View Details">
                <i class="fas fa-info-circle"></i>
            </button>
        `);

        // Download and preview buttons for completed jobs
        if (status === 'completed' && job.file_available) {
            actions.push(`
                <button class="btn btn-sm btn-success" onclick="downloadScan('${job.id}')" title="Download">
                    <i class="fas fa-download"></i>
                </button>
            `);

            if (['jpeg', 'png', 'tiff'].includes(job.format)) {
                actions.push(`
                    <button class="btn btn-sm btn-info" onclick="previewScan('${job.id}')" title="Preview">
                        <i class="fas fa-eye"></i>
                    </button>
                `);
            }
        }

        // Delete button for completed/failed jobs
        if (['completed', 'failed', 'cancelled'].includes(status)) {
            actions.push(`
                <button class="btn btn-sm btn-danger" onclick="deleteScanJob('${job.id}')" title="Delete Job">
                    <i class="fas fa-trash"></i>
                </button>
            `);
        }

        return actions.join('');
    },

    isImageFile(filename) {
        const ext = filename.split('.').pop().toLowerCase();
        return ['jpg', 'jpeg', 'png', 'tiff', 'tif'].includes(ext);
    }
};


// JOB ACTIONS
async function viewScanJobDetails(jobId) {
    try {
        const job = await API.get(`/scan/jobs/${jobId}`);
        showScanJobDetailsModal(job);
    } catch (error) {
        Toast.error('Failed to load job details');
    }
}

function showScanJobDetailsModal(job) {
    const existingModal = document.getElementById('scan-job-details-modal');
    existingModal?.remove();

    const modal = document.createElement('div');
    modal.id = 'scan-job-details-modal';
    modal.className = 'modal';
    modal.style.display = 'flex';

    const status = job.status.toLowerCase();
    const isCompleted = status === 'completed';
    const fileAvailable = job.file_available;

    const settingsItems = [
        `Format: ${job.format.toUpperCase()}`,
        `Resolution: ${job.resolution} DPI`,
        `Color Mode: ${job.color_mode}`,
        `Page Size: ${job.page_size}`
    ];

    if (job.brightness !== undefined) settingsItems.push(`Brightness: ${job.brightness}`);
    if (job.contrast !== undefined) settingsItems.push(`Contrast: ${job.contrast}`);

    modal.innerHTML = `
        <div class="modal-content">
            <div class="modal-header">
                <h3>Scan Job Details</h3>
                <button class="close-btn" onclick="document.getElementById('scan-job-details-modal').remove()">
                    <i class="fas fa-times"></i>
                </button>
            </div>
            <div class="job-details">
                ${createDetailRow('Filename', `<code>${job.output_filename || 'Unnamed'}</code>`)}
                ${createDetailRow('Scanner', ScanHelpers.getScannerDisplayName(job.scanner))}
                ${createDetailRow('Status', `
                    <span class="status-badge status-${status}">
                        <i class="fas ${ScanHelpers.getStatusIcon(status)}"></i>
                        ${job.status}
                    </span>
                `)}
                ${createDetailRow('Created', new Date(job.created_at).toLocaleString())}
                ${job.completed_at ? createDetailRow('Completed', new Date(job.completed_at).toLocaleString()) : ''}
                ${job.file_size ? createDetailRow('File Size', Utils.formatFileSize(job.file_size)) : ''}
                ${job.error_message ? createDetailRow('Error', `<span class="error-message">${job.error_message}</span>`) : ''}
                ${createDetailRow('Settings', `
                    <ul class="job-options">
                        ${settingsItems.map(item => `<li>${item}</li>`).join('')}
                    </ul>
                `)}
            </div>
            <div class="modal-actions">
                <button class="btn btn-secondary" onclick="document.getElementById('scan-job-details-modal').remove()">Close</button>
                ${isCompleted && fileAvailable ? `
                    <button class="btn btn-success" onclick="downloadScan('${job.id}'); document.getElementById('scan-job-details-modal').remove();">
                        <i class="fas fa-download"></i> Download
                    </button>
                ` : ''}
                ${isCompleted && !fileAvailable ? `
                    <span class="text-muted" style="font-size: 0.9em;">
                        <i class="fas fa-exclamation-triangle"></i> File not available
                    </span>
                ` : ''}
            </div>
        </div>
    `;

    document.body.appendChild(modal);
}

function createDetailRow(label, content) {
    return `
        <div class="detail-row">
            <strong>${label}:</strong>
            <span>${content}</span>
        </div>
    `;
}

async function downloadScan(jobId) {
    try {
        window.open(`/api/scan/download/${jobId}`, '_blank');
        Toast.success('Download started');
    } catch (error) {
        Toast.error('Download failed');
    }
}

async function previewScan(jobId) {
    try {
        const job = await API.get(`/scan/jobs/${jobId}`);

        if (!job) {
            Toast.error('Job not found');
            return;
        }

        if (job.status.toLowerCase() !== 'completed') {
            Toast.error('Scan not completed yet');
            return;
        }

        if (!job.file_available) {
            Toast.error('File is no longer available');
            return;
        }

        showPreviewModal(`/api/scan/download/${jobId}`, job.output_filename || 'scan');
    } catch (error) {
        Toast.error('Preview failed');
    }
}

function showPreviewModal(url, filename) {
    const modal = document.getElementById('preview-modal');
    if (!modal) return;

    const container = document.getElementById('preview-container');
    const downloadBtn = document.getElementById('download-preview-btn');

    if (ScanHelpers.isImageFile(filename)) {
        container.innerHTML = `
            <div class="image-preview">
                <img src="${url}" alt="${filename}" style="max-width: 100%; max-height: 70vh; object-fit: contain;">
            </div>
        `;
    } else {
        container.innerHTML = `
            <div class="file-preview">
                <i class="fas fa-file-pdf fa-4x"></i>
                <h3>${filename}</h3>
                <p>Preview not available for this file type</p>
            </div>
        `;
    }

    if (downloadBtn) {
        downloadBtn.onclick = () => window.open(url, '_blank');
    }

    Modal.show('preview-modal');
}

function closePreviewModal() {
    Modal.hide('preview-modal');
}

async function deleteScanJob(jobId) {
    if (!confirm('Are you sure you want to delete this scan job record?')) return;

    try {
        await API.delete(`/scan/jobs/${encodeURIComponent(jobId)}`);
        Toast.success('Scan job deleted successfully');
        await loadScanJobs();
    } catch (error) {
        Toast.error(`Failed to delete job: ${error.message}`);
    }
}

async function clearCompletedScans() {
    const completedJobs = ScanPage.jobs.filter(job =>
        ['completed', 'failed', 'cancelled'].includes(job.status.toLowerCase())
    );

    if (!completedJobs.length) {
        Toast.info('No completed scans to clear');
        return;
    }

    if (!confirm(`Clear ${completedJobs.length} completed scan job(s)?`)) return;

    let cleared = 0;
    for (const job of completedJobs) {
        try {
            await API.delete(`/scan/jobs/${encodeURIComponent(job.id)}`);
            cleared++;
        } catch (error) {
            console.error(`Failed to delete job ${job.id}:`, error);
        }
    }

    Toast.success(`Cleared ${cleared} completed scan job(s)`);
    await loadScanJobs();
}


// SCAN FORM
function setupScanForm() {
    const form = document.getElementById('scan-form');
    if (!form) return;

    form.addEventListener('submit', handleScanFormSubmit);
}

async function handleScanFormSubmit(e) {
    e.preventDefault();

    if (ScanPage.isSubmitting) return;
    ScanPage.isSubmitting = true;

    const submitBtn = document.getElementById('scan-submit-btn');
    const originalText = submitBtn.innerHTML;
    submitBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Starting Scan...';
    submitBtn.disabled = true;

    try {
        const formData = new FormData(e.target);
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

        const result = await API.post('/scan', scanData);

        Toast.success(`Scan job started: ${Utils.formatJobId(result.job_id)}`);
        closeScanDialog();
        e.target.reset();
        resetRangeInputs();

        await loadScanJobs();
    } catch (error) {
        Toast.error(`Scan failed: ${error.message}`);
    } finally {
        ScanPage.isSubmitting = false;
        submitBtn.innerHTML = originalText;
        submitBtn.disabled = false;
    }
}


// RANGE INPUTS
function setupRangeInputs() {
    const ranges = [
        { input: 'scan-brightness', display: 'brightness-value' },
        { input: 'scan-contrast', display: 'contrast-value' }
    ];

    ranges.forEach(({ input, display }) => {
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

function resetRangeInputs() {
    const ranges = [
        { input: 'scan-brightness', display: 'brightness-value' },
        { input: 'scan-contrast', display: 'contrast-value' }
    ];

    ranges.forEach(({ input, display }) => {
        const rangeInput = document.getElementById(input);
        const displayElement = document.getElementById(display);

        if (rangeInput) {
            rangeInput.value = 0;
            updateSliderBackground(rangeInput);
        }
        if (displayElement) {
            displayElement.textContent = '0';
        }
    });
}


// CLEANUP
window.addEventListener('beforeunload', () => {
    if (ScanPage.jobsRefreshInterval) {
        clearInterval(ScanPage.jobsRefreshInterval);
    }
});