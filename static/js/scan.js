// scan.js - Scan page specific functionality

// Page state
const ScanPage = {
    jobs: [],
    scanners: [],
    scanFiles: [],
    jobsRefreshInterval: null,
    isSubmitting: false
};

// Initialize scan page
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

    // Start auto-refresh for jobs
    ScanPage.jobsRefreshInterval = setInterval(loadScanJobs, 5000); // Every 5 seconds
}

// Load and display scanners
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

    if (ScanPage.scanners.length === 0) {
        grid.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-scanner"></i>
                <h3>No Scanners Available</h3>
                <p>Check SANE service and scanner connections</p>
            </div>
        `;
        return;
    }

    grid.innerHTML = ScanPage.scanners.map(scanner => `
        <div class="scanner-card available">
            <div class="scanner-icon">
                <i class="fas fa-scanner"></i>
            </div>
            <div class="scanner-info">
                <h4>${scanner.vendor} ${scanner.model}</h4>
                <p class="scanner-type">${scanner.device_type || 'Flatbed Scanner'}</p>
                <p class="scanner-device" style="font-size: 0.75em; color: #888;" title="${scanner.name}">
                    Device: ${scanner.name.length > 30 ? scanner.name.substring(0, 30) + '...' : scanner.name}
                </p>
            </div>
            <div class="scanner-actions">
                <button class="btn btn-sm btn-primary" onclick="quickScan('${scanner.name}')">
                    <i class="fas fa-scanner"></i>
                    Scan
                </button>
            </div>
        </div>
    `).join('');
}

function showScannersError() {
    const grid = document.getElementById('scanners-grid');
    if (grid) {
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

// Helper function to get scanner display name from device name
function getScannerDisplayName(deviceName) {
    const scanner = ScanPage.scanners.find(s => s.name === deviceName);

    if (scanner) {
        return `${scanner.vendor} ${scanner.model}`;
    }

    if (deviceName.includes(':')) {
        const parts = deviceName.split(':');
        return parts[0].toUpperCase();
    }
    return deviceName;
}

// Load and display scan jobs
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

    if (ScanPage.jobs.length === 0) {
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

    tbody.innerHTML = ScanPage.jobs.map(job => `
        <tr class="job-row job-${job.status.toLowerCase()}">
            <td>
                <span class="filename" title="${job.output_filename || 'Unnamed'}">${truncateFilename(job.output_filename || 'Unnamed', 25)}</span>
            </td>
            <td>
                <span class="scanner-name">${getScannerDisplayName(job.scanner)}</span>
            </td>
            <td>
                <span class="format-badge format-${job.format}">${job.format.toUpperCase()}</span>
            </td>
            <td>
                <span class="resolution">${job.resolution} DPI</span>
            </td>
            <td>
                <span class="status-badge status-${job.status.toLowerCase()}">
                    <i class="fas ${getScanStatusIcon(job.status)}"></i>
                    ${job.status}
                </span>
            </td>
            <td>
                <span class="job-time" title="${new Date(job.created_at).toLocaleString()}">
                    ${timeAgo(job.created_at)}
                </span>
            </td>
            <td>
                <span class="file-size">
                    ${job.file_size ? Utils.formatFileSize(job.file_size) : '-'}
                </span>
            </td>
            <td>
                <div class="job-actions">
                    ${getScanJobActions(job)}
                </div>
            </td>
        </tr>
    `).join('');
}

function showJobsError() {
    const tbody = document.getElementById('scan-jobs-tbody');
    if (tbody) {
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
}

function getScanStatusIcon(status) {
    const icons = {
        'queued': 'fa-clock',
        'scanning': 'fa-spinner fa-spin',
        'processing': 'fa-cog fa-spin',
        'completed': 'fa-check-circle',
        'failed': 'fa-exclamation-circle',
        'cancelled': 'fa-times-circle'
    };
    return icons[status.toLowerCase()] || 'fa-question-circle';
}

function getScanJobActions(job) {
    const status = job.status.toLowerCase();
    let actions = [];

    // View details button
    actions.push(`
        <button class="btn btn-sm btn-secondary" onclick="viewScanJobDetails('${job.id}')" title="View Details">
            <i class="fas fa-info-circle"></i>
        </button>
    `);

    // Download button (only for completed jobs where file is available)
    if (status === 'completed' && job.file_available) {
        actions.push(`
            <button class="btn btn-sm btn-success" onclick="downloadScan('${job.id}')" title="Download">
                <i class="fas fa-download"></i>
            </button>
        `);

        // Preview button (for image formats)
        if (['jpeg', 'png', 'tiff'].includes(job.format)) {
            actions.push(`
                <button class="btn btn-sm btn-info" onclick="previewScan('${job.id}')" title="Preview">
                    <i class="fas fa-eye"></i>
            </button>
        `);
        }
    }

    // Delete button (for completed/failed jobs)
    if (['completed', 'failed', 'cancelled'].includes(status)) {
        actions.push(`
            <button class="btn btn-sm btn-danger" onclick="deleteScanJob('${job.id}')" title="Delete Job">
                <i class="fas fa-trash"></i>
            </button>
        `);
    }

    return actions.join('');
}

function truncateFilename(filename, maxLength = 20) {
    if (!filename || filename.length <= maxLength) return filename;

    const ext = filename.split('.').pop();
    const name = filename.substring(0, filename.lastIndexOf('.'));

    return name.substring(0, maxLength - ext.length - 4) + '...'
}

function isImageFile(filename) {
    const ext = filename.split('.').pop().toLowerCase();
    return ['jpg', 'jpeg', 'png', 'tiff', 'tif'].includes(ext);
}

// Setup scan form
function setupScanForm() {
    const form = document.getElementById('scan-form');
    if (!form) return;

    form.addEventListener('submit', async (e) => {
        e.preventDefault();

        if (ScanPage.isSubmitting) return;
        ScanPage.isSubmitting = true;

        const submitBtn = document.getElementById('scan-submit-btn');
        const originalText = submitBtn.innerHTML;
        submitBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Starting Scan...';
        submitBtn.disabled = true;

        try {
            const formData = new FormData(form);
            const scanData = Object.fromEntries(formData);

            // Convert numeric fields
            scanData.resolution = parseInt(scanData.resolution);
            if (scanData.brightness) scanData.brightness = parseInt(scanData.brightness);
            if (scanData.contrast) scanData.contrast = parseInt(scanData.contrast);

            const result = await API.post('/scan', scanData);

            Toast.success(`Scan job started: ${result.job_id.substring(0, 8)}...`);
            closeScanDialog();
            form.reset();
            resetRangeInputs();

            await loadScanJobs();

        } catch (error) {
            Toast.error(`Scan failed: ${error.message}`);
        } finally {
            ScanPage.isSubmitting = false;
            submitBtn.innerHTML = originalText;
            submitBtn.disabled = false;
        }
    });
}

// Setup range input listeners
function setupRangeInputs() {
    const brightnessRange = document.getElementById('scan-brightness');
    const contrastRange = document.getElementById('scan-contrast');
    const brightnessValue = document.getElementById('brightness-value');
    const contrastValue = document.getElementById('contrast-value');

    if (brightnessRange && brightnessValue) {
        brightnessRange.addEventListener('input', (e) => {
            brightnessValue.textContent = e.target.value;
        });
    }

    if (contrastRange && contrastValue) {
        contrastRange.addEventListener('input', (e) => {
            contrastValue.textContent = e.target.value;
        });
    }
}

function resetRangeInputs() {
    const brightnessRange = document.getElementById('scan-brightness');
    const contrastRange = document.getElementById('scan-contrast');
    const brightnessValue = document.getElementById('brightness-value');
    const contrastValue = document.getElementById('contrast-value');

    if (brightnessRange) {
        brightnessRange.value = 0;
        if (brightnessValue) brightnessValue.textContent = '0';
    }

    if (contrastRange) {
        contrastRange.value = 0;
        if (contrastValue) contrastValue.textContent = '0';
    }
}

function timeAgo(dateString) {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    return `${diffDays}d ago`;
}

// Action functions
async function refreshScanners() {
    const button = event.target;
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
}

async function refreshScanJobs() {
    await loadScanJobs();
    Toast.info('Scan jobs refreshed');
}

function quickScan(scannerName) {
    showScanDialog();
    const scannerSelect = document.getElementById('scan-scanner');
    if (scannerSelect) {
        scannerSelect.value = scannerName;
    }
}

async function viewScanJobDetails(jobId) {
    try {
        const job = await API.get(`/scan/jobs/${jobId}`);
        showScanJobDetailsModal(job);
    } catch (error) {
        Toast.error('Failed to load job details');
    }
}

function showScanJobDetailsModal(job) {
    // Remove existing modal if present
    const existingModal = document.getElementById('scan-job-details-modal');
    if (existingModal) existingModal.remove();

    // Create modal
    const modal = document.createElement('div');
    modal.id = 'scan-job-details-modal';
    modal.className = 'modal';
    modal.style.display = 'flex';

    modal.innerHTML = `
        <div class="modal-content">
            <div class="modal-header">
                <h3>Scan Job Details</h3>
                <button class="close-btn" onclick="document.getElementById('scan-job-details-modal').remove()">
                    <i class="fas fa-times"></i>
                </button>
            </div>
            <div class="job-details">
                <div class="detail-row">
                    <strong>Filename:</strong>
                    <code>${job.output_filename || 'Unnamed'}</code>
                </div>
                <div class="detail-row">
                    <strong>Scanner:</strong>
                    <span>${getScannerDisplayName(job.scanner)}</span>
                </div>
                <div class="detail-row">
                    <strong>Status:</strong>
                    <span class="status-badge status-${job.status.toLowerCase()}">
                        <i class="fas ${getScanStatusIcon(job.status)}"></i>
                        ${job.status}
                    </span>
                </div>
                <div class="detail-row">
                    <strong>Created:</strong>
                    <span>${new Date(job.created_at).toLocaleString()}</span>
                </div>
                ${job.completed_at ? `
                <div class="detail-row">
                    <strong>Completed:</strong>
                    <span>${new Date(job.completed_at).toLocaleString()}</span>
                </div>
                ` : ''}
                ${job.file_size ? `
                <div class="detail-row">
                    <strong>File Size:</strong>
                    <span>${Utils.formatFileSize(job.file_size)}</span>
                </div>
                ` : ''}
                ${job.error_message ? `
                <div class="detail-row">
                    <strong>Error:</strong>
                    <span class="error-message">${job.error_message}</span>
                </div>
                ` : ''}
                <div class="detail-row">
                    <strong>Settings:</strong>
                    <ul class="job-options">
                        <li>Format: ${job.format.toUpperCase()}</li>
                        <li>Resolution: ${job.resolution} DPI</li>
                        <li>Color Mode: ${job.color_mode}</li>
                        <li>Page Size: ${job.page_size}</li>
                        ${job.brightness !== undefined ? `<li>Brightness: ${job.brightness}</li>` : ''}
                        ${job.contrast !== undefined ? `<li>Contrast: ${job.contrast}</li>` : ''}
                    </ul>
                </div>
            </div>
            <div class="modal-actions">
                <button class="btn btn-secondary" onclick="document.getElementById('scan-job-details-modal').remove()">Close</button>
                ${job.status.toLowerCase() === 'completed' && job.file_available ? `
                <button class="btn btn-success" onclick="downloadScan('${job.id}'); document.getElementById('scan-job-details-modal').remove();">
                    <i class="fas fa-download"></i> Download
                </button>
                ` : ''}
                ${job.status.toLowerCase() === 'completed' && !job.file_available ? `
                <span class="text-muted" style="font-size: 0.9em;">
                    <i class="fas fa-exclamation-triangle"></i> File not available
                </span>
                ` : ''}
            </div>
        </div>
    `;

    document.body.appendChild(modal);
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
        if (job && job.status.toLowerCase() === 'completed' && job.file_available) {
            showPreviewModal(`/api/scan/download/${jobId}`, job.output_filename || 'scan');
        } else if (job && !job.file_available) {
            Toast.error('File is no longer available');
        } else {
            Toast.error('Cannot preview this scan');
        }
    } catch (error) {
        Toast.error('Preview failed');
    }
}

function showPreviewModal(url, filename) {
    const modal = document.getElementById('preview-modal');
    if (!modal) return;

    const container = document.getElementById('preview-container');
    const downloadBtn = document.getElementById('download-preview-btn');

    if (isImageFile(filename)) {
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

    downloadBtn.onclick = () => {
        window.open(url, '_blank');
    };

    Modal.show('preview-modal');
}

function closePreviewModal() {
    Modal.hide('preview-modal');
}

async function deleteScanJob(jobId) {
    if (!confirm('Are you sure you want to delete this scan job record?')) return;

    try {
        await API.delete(`/scan/jobs/${encodeURIComponent(jobId)}`)
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

    if (completedJobs.length === 0) {
        Toast.info('No completed scans to clear');
        return;
    }

    if (!confirm(`Clear ${completedJobs.length} completed scan job(s)?`)) return;

    Toast.info('Clear completed scans not fully implemented yet');
}

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    if (ScanPage.jobsRefreshInterval) {
        clearInterval(ScanPage.jobsRefreshInterval);
    }
});