// print.js - Print page specific functionality

// Page state
const PrintPage = {
    jobs: [],
    printers: [],
    jobsRefreshInterval: null,
    isSubmitting: false
};

// Initialize print page
document.addEventListener('DOMContentLoaded', () => {
    if (window.location.pathname === '/print') {
        initializePrintPage();
    }
});

async function initializePrintPage() {
    await loadPrinters();
    await loadPrintJobs();
    setupPrintForm();

    // Start auto-refresh for jobs
    PrintPage.jobsRefreshInterval = setInterval(loadPrintJobs, 5000); // Every 5 seconds
}

// Load and display printers
async function loadPrinters() {
    try {
        PrintPage.printers = await API.get('/printers');
        displayPrinters();
        populatePrinterDropdown();
    } catch (error) {
        console.error('Failed to load printers:', error);
        showPrintersError();
    }
}

function displayPrinters() {
    const grid = document.getElementById('printers-grid');
    if (!grid) return;

    if (PrintPage.printers.length === 0) {
        grid.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-printer"></i>
                <h3>No Printers Available</h3>
                <p>Check CUPS service and printer connections</p>
            </div>
        `;
        return;
    }

    grid.innerHTML = PrintPage.printers.map(printer => `
        <div class="printer-card ${printer.is_default ? 'default' : ''} ${printer.status === 'idle' ? 'available' : 'busy'}">
            <div class="printer-icon">
                <i class="fas ${printer.is_default ? 'fa-star' : 'fa-printer'}"></i>
            </div>
            <div class="printer-info">
                <h4>${printer.name}</h4>
                <p class="printer-status">
                    <span class="status-dot status-${printer.status}"></span>
                    ${printer.status}${printer.is_default ? ' (Default)' : ''}
                </p>
                <p class="printer-description">${printer.description || 'No description available'}</p>
                <p class="printer-location">${printer.location || 'No location set'}</p>
            </div>
            <div class="printer-actions">
                <button class="btn btn-sm ${printer.status === 'idle' ? 'btn-primary' : 'btn-secondary'}" 
                        onclick="quickPrint('${printer.name}')"
                        ${printer.status !== 'idle' ? 'disabled' : ''}>
                    <i class="fas fa-print"></i>
                    Print
                </button>
            </div>
        </div>
    `).join('');
}

function showPrintersError() {
    const grid = document.getElementById('printers-grid');
    if (grid) {
        grid.innerHTML = `
            <div class="error-state">
                <i class="fas fa-exclamation-triangle"></i>
                <h3>Failed to Load Printers</h3>
                <p>Check CUPS service status</p>
                <button class="btn btn-secondary" onclick="refreshPrinters()">
                    <i class="fas fa-refresh"></i>
                    Try Again
                </button>
            </div>
        `;
    }
}

function populatePrinterDropdown() {
    const select = document.getElementById('print-printer');
    if (!select) return;

    select.innerHTML = '<option value="">Default Printer</option>';

    PrintPage.printers.forEach(printer => {
        const option = document.createElement('option');
        option.value = printer.name;
        option.textContent = `${printer.name}${printer.is_default ? ' (Default)' : ''}`;
        if (printer.is_default) option.selected = true;
        select.appendChild(option);
    });
}

// Load and display print jobs
async function loadPrintJobs() {
    try {
        PrintPage.jobs = await API.get('/print/jobs');
        displayPrintJobs();
    } catch (error) {
        console.error('Failed to load print jobs:', error);
        showJobsError();
    }
}

function displayPrintJobs() {
    const tbody = document.getElementById('jobs-tbody');
    if (!tbody) return;

    if (PrintPage.jobs.length === 0) {
        tbody.innerHTML = `
            <tr>
                <td colspan="7" class="empty-state">
                    <i class="fas fa-print"></i>
                    <h3>No Print Jobs</h3>
                    <p>Start printing to see jobs here</p>
                </td>
            </tr>
        `;
        return;
    }

    tbody.innerHTML = PrintPage.jobs.map(job => `
        <tr class="job-row job-${job.status.toLowerCase()}">
            <td>
                <code class="job-id" title="${job.id}">${Utils.formatJobId(job.id)}</code>
            </td>
            <td>
                <span class="filename" title="${job.filename}">${job.filename}</span>
            </td>
            <td>
                <span class="printer-name">${job.printer}</span>
            </td>
            <td>
                <span class="status-badge status-${job.status.toLowerCase()}">
                    <i class="fas ${getStatusIcon(job.status)}"></i>
                    ${job.status}
                </span>
            </td>
            <td>
                <span class="job-time" title="${new Date(job.created_at).toLocaleString()}">
                    ${timeAgo(job.created_at)}
                </span>
            </td>
            <td>
                <div class="progress-container">
                    ${getJobProgress(job)}
                </div>
            </td>
            <td>
                <div class="job-actions">
                    ${getJobActions(job)}
                </div>
            </td>
        </tr>
    `).join('');
}

function showJobsError() {
    const tbody = document.getElementById('jobs-tbody');
    if (tbody) {
        tbody.innerHTML = `
            <tr>
                <td colspan="7" class="error-state">
                    <i class="fas fa-exclamation-triangle"></i>
                    Failed to load jobs
                    <button class="btn btn-sm btn-secondary" onclick="refreshJobs()">Retry</button>
                </td>
            </tr>
        `;
    }
}

function getStatusIcon(status) {
    const icons = {
        'queued': 'fa-clock',
        'processing': 'fa-spinner fa-spin',
        'printing': 'fa-print',
        'completed': 'fa-check-circle',
        'failed': 'fa-exclamation-circle',
        'cancelled': 'fa-times-circle'
    };
    return icons[status.toLowerCase()] || 'fa-question-circle';
}

function getJobProgress(job) {
    const status = job.status.toLowerCase();

    if (status === 'completed') {
        return '<div class="progress-bar"><div class="progress-fill" style="width: 100%"></div></div>';
    } else if (status === 'failed' || status === 'cancelled') {
        return '<div class="progress-bar error"><div class="progress-fill" style="width: 100%"></div></div>';
    } else if (status === 'printing') {
        return '<div class="progress-bar active"><div class="progress-fill" style="width: 75%"></div></div>';
    } else if (status === 'processing') {
        return '<div class="progress-bar active"><div class="progress-fill" style="width: 25%"></div></div>';
    } else {
        return '<div class="progress-bar"><div class="progress-fill" style="width: 0%"></div></div>';
    }
}

function getJobActions(job) {
    const status = job.status.toLowerCase();
    let actions = [];

    // View details button
    actions.push(`
        <button class="btn btn-sm btn-secondary" onclick="viewJobDetails('${job.id}')" title="View Details">
            <i class="fas fa-info-circle"></i>
        </button>
    `);

    // Cancel button (only for active jobs)
    if (['queued', 'processing', 'printing'].includes(status)) {
        actions.push(`
            <button class="btn btn-sm btn-danger" onclick="cancelJob('${job.id}')" title="Cancel Job">
                <i class="fas fa-times"></i>
            </button>
        `);
    }

    // Delete button (for completed/failed jobs)
    if (['completed', 'failed', 'cancelled'].includes(status)) {
        actions.push(`
            <button class="btn btn-sm btn-danger" onclick="deleteJob('${job.id}')" title="Delete Job">
                <i class="fas fa-trash"></i>
            </button>
        `);
    }

    return actions.join('');
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

// Setup print form
function setupPrintForm() {
    const form = document.getElementById('print-form');
    if (!form) return;

    form.addEventListener('submit', async (e) => {
        e.preventDefault();

        if (PrintPage.isSubmitting) return;
        PrintPage.isSubmitting = true;

        const submitBtn = document.getElementById('print-submit-btn');
        const originalText = submitBtn.innerHTML;
        submitBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Printing...';
        submitBtn.disabled = true;

        try {
            const formData = new FormData(form);
            const result = await API.postForm('/print', formData);

            Toast.success(`Print job submitted: ${result.job_id.substring(0, 8)}...`);
            closePrintDialog();
            form.reset();

            await loadPrintJobs();

        } catch (error) {
            Toast.error(`Print failed: ${error.message}`);
        } finally {
            PrintPage.isSubmitting = false;
            submitBtn.innerHTML = originalText;
            submitBtn.disabled = false;
        }
    });

    // File input validation
    const fileInput = document.getElementById('print-file');
    if (fileInput) {
        fileInput.addEventListener('change', (e) => {
            const file = e.target.files[0];
            if (file) {
                const maxSize = 50 * 1024 * 1024; // 50MB
                if (file.size > maxSize) {
                    Toast.error('File size must be less than 50MB');
                    fileInput.value = '';
                    return;
                }

                const allowedTypes = [
                    'application/pdf',
                    'application/msword',
                    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
                    'text/plain',
                    'image/jpeg',
                    'image/png'
                ];

                if (!allowedTypes.includes(file.type)) {
                    Toast.error('Unsupported file type. Please use PDF, DOC, DOCX, TXT, JPG, or PNG.');
                    fileInput.value = '';
                }
            }
        });
    }
}

// Action functions
async function refreshPrinters() {
    const button = event.target;
    const originalContent = button.innerHTML;
    button.innerHTML = '<i class="fas fa-spinner fa-spin"></i>';
    button.disabled = true;

    try {
        await loadPrinters();
        Toast.success('Printers refreshed');
    } catch (error) {
        Toast.error('Failed to refresh printers');
    } finally {
        button.innerHTML = originalContent;
        button.disabled = false;
    }
}

async function refreshJobs() {
    await loadPrintJobs();
    Toast.info('Jobs refreshed');
}

function quickPrint(printerName) {
    showPrintDialog();
    const printerSelect = document.getElementById('print-printer');
    if (printerSelect) {
        printerSelect.value = printerName;
    }
}

async function viewJobDetails(jobId) {
    try {
        const job = await API.get(`/print/jobs/${jobId}`);

        // Create and show job details modal
        showJobDetailsModal(job);

    } catch (error) {
        Toast.error('Failed to load job details');
    }
}

function showJobDetailsModal(job) {
    // Remove existing modal if present
    const existingModal = document.getElementById('job-details-modal');
    if (existingModal) existingModal.remove();

    // Create modal
    const modal = document.createElement('div');
    modal.id = 'job-details-modal';
    modal.className = 'modal';
    modal.style.display = 'flex';

    modal.innerHTML = `
        <div class="modal-content">
            <div class="modal-header">
                <h3>Job Details</h3>
                <button class="close-btn" onclick="document.getElementById('job-details-modal').remove()">
                    <i class="fas fa-times"></i>
                </button>
            </div>
            <div class="job-details">
                <div class="detail-row">
                    <strong>Job ID:</strong>
                    <code>${job.id}</code>
                </div>
                <div class="detail-row">
                    <strong>Filename:</strong>
                    <span>${job.filename}</span>
                </div>
                <div class="detail-row">
                    <strong>Printer:</strong>
                    <span>${job.printer}</span>
                </div>
                <div class="detail-row">
                    <strong>Status:</strong>
                    <span class="status-badge status-${job.status.toLowerCase()}">
                        <i class="fas ${getStatusIcon(job.status)}"></i>
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
                ${job.cups_job_id ? `
                <div class="detail-row">
                    <strong>CUPS Job ID:</strong>
                    <span>${job.cups_job_id}</span>
                </div>
                ` : ''}
                ${job.error_message ? `
                <div class="detail-row">
                    <strong>Error:</strong>
                    <span class="error-message">${job.error_message}</span>
                </div>
                ` : ''}
                <div class="detail-row">
                    <strong>Options:</strong>
                    <ul class="job-options">
                        <li>Copies: ${job.copies || 1}</li>
                        <li>Pages: ${job.pages || 'All'}</li>
                        <li>Duplex: ${job.duplex ? 'Yes' : 'No'}</li>
                        <li>Color: ${job.color ? 'Yes' : 'No'}</li>
                    </ul>
                </div>
            </div>
            <div class="modal-actions">
                <button class="btn btn-secondary" onclick="document.getElementById('job-details-modal').remove()">Close</button>
                ${['queued', 'processing', 'printing'].includes(job.status.toLowerCase()) ? `
                <button class="btn btn-danger" onclick="cancelJob('${job.id}'); document.getElementById('job-details-modal').remove();">
                    <i class="fas fa-times"></i> Cancel Job
                </button>
                ` : ''}
            </div>
        </div>
    `;

    document.body.appendChild(modal);
}

async function cancelJob(jobId) {
    if (!confirm('Are you sure you want to cancel this print job?')) return;

    try {
        await API.post(`/print/jobs/${jobId}`);
        Toast.success('Print job cancelled');
        await loadPrintJobs();
    } catch (error) {
        Toast.error(`Failed to cancel job: ${error.message}`);
    }
}

async function deleteJob(jobId) {
    if (!confirm('Are you sure you want to delete this print job record?')) return;

    try {
        await API.delete(`/print/jobs/${jobId}`);
        Toast.success('Print job deleted');
        await loadPrintJobs();
    } catch (error) {
        Toast.error(`Failed to delete job: ${error.message}`);
    }
}

async function clearCompletedJobs() {
    const completedJobs = PrintPage.jobs.filter(job =>
        ['completed', 'failed', 'cancelled'].includes(job.status.toLowerCase())
    );

    if (completedJobs.length === 0) {
        Toast.info('No completed jobs to clear');
        return;
    }

    if (!confirm(`Clear ${completedJobs.length} completed job(s)?`)) return;

    let cleared = 0;
    for (const job of completedJobs) {
        try {
            await API.delete(`/print/jobs/${job.id}`);
            cleared++;
        } catch (error) {
            console.error(`Failed to delete job ${job.id}:`, error);
        }
    }

    Toast.success(`Cleared ${cleared} completed job(s)`);
    await loadPrintJobs();
}

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    if (PrintPage.jobsRefreshInterval) {
        clearInterval(PrintPage.jobsRefreshInterval);
    }
});