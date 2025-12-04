// print.js - Print page specific functionality


// PAGE STATE
const PrintPage = {
    jobs: [],
    printers: [],
    jobsRefreshInterval: null,
    isSubmitting: false
};


// INITIALIZATION
document.addEventListener('DOMContentLoaded', () => {
    if (window.location.pathname === '/print') {
        initializePrintPage();
    }
});

async function initializePrintPage() {
    await loadPrinters();
    await loadPrintJobs();
    setupPrintForm();

    // Auto-refresh jobs every 5 seconds
    PrintPage.jobsRefreshInterval = setInterval(loadPrintJobs, 5000);
}


// PRINTERS
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

    if (!PrintPage.printers.length) {
        grid.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-printer"></i>
                <h3>No Printers Available</h3>
                <p>Check CUPS service and printer connections</p>
            </div>
        `;
        return;
    }

    grid.innerHTML = PrintPage.printers.map(printer => {
        const isAvailable = printer.status === 'idle';
        const cardClass = `printer-card ${printer.is_default ? 'default' : ''} ${isAvailable ? 'available' : 'busy'}`;

        return `
            <div class="${cardClass}">
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
                    <button class="btn btn-sm ${isAvailable ? 'btn-primary' : 'btn-secondary'}" 
                            onclick="quickPrint('${printer.name}')"
                            ${!isAvailable ? 'disabled' : ''}>
                        <i class="fas fa-print"></i>
                        Print
                    </button>
                </div>
            </div>
        `;
    }).join('');
}

function showPrintersError() {
    const grid = document.getElementById('printers-grid');
    if (!grid) return;

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

async function refreshPrinters() {
    const button = event?.target;
    if (button) {
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
    } else {
        await loadPrinters();
    }
}

function quickPrint(printerName) {
    showPrintDialog();
    const printerSelect = document.getElementById('print-printer');
    if (printerSelect) {
        printerSelect.value = printerName;
    }
}


// PRINT JOBS
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

    if (!PrintPage.jobs.length) {
        tbody.innerHTML = `
            <tr>
                <td colspan="8" class="empty-state">
                    <i class="fas fa-print"></i>
                    <h3>No Print Jobs</h3>
                    <p>Start printing to see jobs here</p>
                </td>
            </tr>
        `;
        return;
    }

    tbody.innerHTML = PrintPage.jobs.map(job => {
        const status = job.status.toLowerCase();

        return `
            <tr class="job-row job-${status}">
                <td>
                    <span class="filename" title="${job.filename}">${job.filename}</span>
                </td>
                <td>
                    <span class="printer-name">${PrintHelpers.getPrinterDisplayName(job)}</span>
                </td>
                <td>
                    <span class="copies-count">${job.copies || 1}</span>
                </td>
                <td>
                    <span class="color-mode ${job.color ? 'color-yes' : 'color-no'}">
                        ${job.color ? 'Color' : 'Grayscale'}
                    </span>
                </td>
                <td>
                    <span class="job-time" title="${new Date(job.created_at).toLocaleString()}">
                        ${Utils.formatActivityTime(job.created_at)}
                    </span>
                </td>
                <td>
                    <span class="status-badge status-${status}">
                        <i class="fas ${PrintHelpers.getStatusIcon(status)}"></i>
                        ${job.status}
                    </span>
                </td>
                <td>
                    <div class="progress-container">
                        ${PrintHelpers.getProgressBar(status)}
                    </div>
                </td>
                <td>
                    <div class="job-actions">
                        ${PrintHelpers.getActionButtons(job)}
                    </div>
                </td>
            </tr>
        `;
    }).join('');
}

function showJobsError() {
    const tbody = document.getElementById('jobs-tbody');
    if (!tbody) return;

    tbody.innerHTML = `
        <tr>
            <td colspan="8" class="error-state">
                <i class="fas fa-exclamation-triangle"></i>
                Failed to load jobs
                <button class="btn btn-sm btn-secondary" onclick="refreshJobs()">Retry</button>
            </td>
        </tr>
    `;
}

async function refreshJobs() {
    await loadPrintJobs();
    Toast.info('Jobs refreshed');
}


// JOB HELPERS
const PrintHelpers = {
    getPrinterDisplayName(job) {
        const vendor = job.vendor || "Unknown";
        const model = job.model || "Unknown";

        return vendor + " " + model;
    },

    getStatusIcon(status) {
        const icons = {
            'queued': 'fa-clock',
            'processing': 'fa-spinner fa-spin',
            'printing': 'fa-print',
            'completed': 'fa-check-circle',
            'failed': 'fa-exclamation-circle',
            'cancelled': 'fa-times-circle'
        };
        return icons[status] || 'fa-question-circle';
    },

    getProgressBar(status) {
        const progressMap = {
            'completed': { width: 100, class: '' },
            'failed': { width: 100, class: 'error' },
            'cancelled': { width: 100, class: 'error' },
            'printing': { width: 75, class: 'active' },
            'processing': { width: 25, class: 'active' },
            'queued': { width: 0, class: '' }
        };

        const progress = progressMap[status] || { width: 0, class: '' };
        return `<div class="progress-bar ${progress.class}"><div class="progress-fill" style="width: ${progress.width}%"></div></div>`;
    },

    getActionButtons(job) {
        const status = job.status.toLowerCase();
        const actions = [];

        // View details button
        actions.push(`
            <button class="btn btn-sm btn-secondary" onclick="viewJobDetails('${job.id}')" title="View Details">
                <i class="fas fa-info-circle"></i>
            </button>
        `);

        // Cancel button for active jobs
        if (['queued', 'processing', 'printing'].includes(status)) {
            actions.push(`
                <button class="btn btn-sm btn-danger" onclick="cancelJob('${job.id}')" title="Cancel Job">
                    <i class="fas fa-times"></i>
                </button>
            `);
        }

        // Delete button for completed/failed jobs
        if (['completed', 'failed', 'cancelled'].includes(status)) {
            actions.push(`
                <button class="btn btn-sm btn-danger" onclick="deleteJob('${job.id}')" title="Delete Job">
                    <i class="fas fa-trash"></i>
                </button>
            `);
        }

        return actions.join('');
    }
};


// JOB ACTIONS
async function viewJobDetails(jobId) {
    try {
        const job = await API.get(`/print/jobs/${jobId}`);
        showJobDetailsModal(job);
    } catch (error) {
        Toast.error('Failed to load job details');
    }
}

function showJobDetailsModal(job) {
    // Remove existing modal if present
    const existingModal = document.getElementById('job-details-modal');
    existingModal?.remove();

    const modal = document.createElement('div');
    modal.id = 'job-details-modal';
    modal.className = 'modal';
    modal.style.display = 'flex';

    const status = job.status.toLowerCase();
    const isActive = ['queued', 'processing', 'printing'].includes(status);

    modal.innerHTML = `
        <div class="modal-content">
            <div class="modal-header">
                <h3>Job Details</h3>
                <button class="close-btn" onclick="document.getElementById('job-details-modal').remove()">
                    <i class="fas fa-times"></i>
                </button>
            </div>
            <div class="job-details">
                ${createDetailRow('Job ID', `<code>${job.id}</code>`)}
                ${createDetailRow('Filename', job.filename)}
                ${createDetailRow('Printer', PrintHelpers.getPrinterDisplayName(job))}
                ${createDetailRow('Status', `
                    <span class="status-badge status-${status}">
                        <i class="fas ${PrintHelpers.getStatusIcon(status)}"></i>
                        ${job.status}
                    </span>
                `)}
                ${createDetailRow('Created', new Date(job.created_at).toLocaleString())}
                ${job.completed_at ? createDetailRow('Completed', new Date(job.completed_at).toLocaleString()) : ''}
                ${job.cups_job_id ? createDetailRow('CUPS Job ID', job.cups_job_id) : ''}
                ${job.error_message ? createDetailRow('Error', `<span class="error-message">${job.error_message}</span>`) : ''}
                ${createDetailRow('Options', `
                    <ul class="job-options">
                        <li>Copies: ${job.copies || 1}</li>
                        <li>Pages: ${job.pages || 'All'}</li>
                        <li>Duplex: ${job.duplex ? 'Yes' : 'No'}</li>
                        <li>Color: ${job.color ? 'Yes' : 'No'}</li>
                    </ul>
                `)}
            </div>
            <div class="modal-actions">
                <button class="btn btn-secondary" onclick="document.getElementById('job-details-modal').remove()">Close</button>
                ${isActive ? `
                    <button class="btn btn-danger" onclick="cancelJob('${job.id}'); document.getElementById('job-details-modal').remove();">
                        <i class="fas fa-times"></i> Cancel Job
                    </button>
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

async function cancelJob(jobId) {
    if (!confirm('Are you sure you want to cancel this print job?')) return;

    try {
        await API.post(`/print/jobs/${jobId}/cancel`);
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

    if (!completedJobs.length) {
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


// PRINT FORM
function setupPrintForm() {
    const form = document.getElementById('print-form');
    if (!form) return;

    form.addEventListener('submit', handlePrintFormSubmit);
    setupFileInputValidation();
}

async function handlePrintFormSubmit(e) {
    e.preventDefault();

    if (PrintPage.isSubmitting) return;
    PrintPage.isSubmitting = true;

    const submitBtn = document.getElementById('print-submit-btn');
    const originalText = submitBtn.innerHTML;
    submitBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Printing...';
    submitBtn.disabled = true;

    try {
        const formData = new FormData(e.target);

        // Remove empty pages field to print all pages
        const pagesValue = formData.get('pages');
        if (!pagesValue?.trim()) {
            formData.delete('pages');
        }

        const result = await API.postForm('/print', formData);

        Toast.success(`Print job submitted: ${Utils.formatJobId(result.job_id)}`);
        closePrintDialog();
        e.target.reset();

        await loadPrintJobs();
    } catch (error) {
        Toast.error(`Print failed: ${error.message}`);
    } finally {
        PrintPage.isSubmitting = false;
        submitBtn.innerHTML = originalText;
        submitBtn.disabled = false;
    }
}

function setupFileInputValidation() {
    const fileInput = document.getElementById('print-file');
    if (!fileInput) return;

    fileInput.addEventListener('change', (e) => {
        const file = e.target.files[0];
        if (!file) return;

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
    });
}


// CLEANUP
window.addEventListener('beforeunload', () => {
    if (PrintPage.jobsRefreshInterval) {
        clearInterval(PrintPage.jobsRefreshInterval);
    }
});