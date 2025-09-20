// Print management JavaScript
// Handles all print-related functionality

class PrintManager {
    constructor() {
        this.printers = [];
        this.printJobs = [];
        this.selectedFiles = [];
        this.init();
    }

    async init() {
        await this.loadPrinters();
        await this.loadPrintJobs();
        this.setupEventListeners();
        this.setupDropZone();
        this.updateUI();
    }

    // Load available printers
    async loadPrinters() {
        try {
            this.printers = await window.app.apiCall('/printers');
            this.updatePrinterSelect();
        } catch (error) {
            console.error('Failed to load printers:', error);
            window.app.showNotification('Failed to load printers', 'error');
        }
    }

    // Load current print jobs
    async loadPrintJobs() {
        try {
            this.printJobs = await window.app.apiCall('/print/jobs');
            this.updateJobList();
        } catch (error) {
            console.error('Failed to load print jobs:', error);
        }
    }

    // Update printer selection dropdown
    updatePrinterSelect() {
        const select = document.getElementById('printer-select');
        if (!select) return;

        select.innerHTML = '<option value="">Select Printer</option>';

        this.printers.forEach(printer => {
            const option = document.createElement('option');
            option.value = printer.name;
            option.textContent = `${printer.description} (${printer.status})`;
            option.selected = printer.is_default;
            select.appendChild(option);
        });
    }

    // Setup event listeners
    setupEventListeners() {
        // File input handler
        const fileInput = document.getElementById('file-input');
        if (fileInput) {
            fileInput.addEventListener('change', (e) => {
                this.handleFileSelection(Array.from(e.target.files));
            });
        }

        // Print form submission
        const printForm = document.getElementById('print-form');
        if (printForm) {
            printForm.addEventListener('submit', (e) => {
                e.preventDefault();
                this.submitPrintJob();
            });
        }

        // Files dropped event
        document.addEventListener('filesDropped', (e) => {
            if (e.detail.dropZone.id === 'print-drop-zone') {
                this.handleFileSelection(e.detail.files);
            }
        });

        // Clear files button
        const clearBtn = document.getElementById('clear-files');
        if (clearBtn) {
            clearBtn.addEventListener('click', () => {
                this.clearSelectedFiles();
            });
        }

        // Advanced options toggle
        const advancedToggle = document.getElementById('advanced-toggle');
        const advancedOptions = document.getElementById('advanced-options');
        if (advancedToggle && advancedOptions) {
            advancedToggle.addEventListener('click', () => {
                advancedOptions.classList.toggle('hidden');
                advancedToggle.textContent = advancedOptions.classList.contains('hidden')
                    ? 'Show Advanced Options'
                    : 'Hide Advanced Options';
            });
        }
    }

    // Setup drag and drop zone
    setupDropZone() {
        const dropZone = document.getElementById('print-drop-zone');
        if (!dropZone) return;

        dropZone.addEventListener('dragenter', (e) => {
            e.preventDefault();
            dropZone.classList.add('drag-over');
        });

        dropZone.addEventListener('dragleave', (e) => {
            e.preventDefault();
            if (!dropZone.contains(e.relatedTarget)) {
                dropZone.classList.remove('drag-over');
            }
        });

        dropZone.addEventListener('drop', (e) => {
            e.preventDefault();
            dropZone.classList.remove('drag-over');
        });

        // Click to select files
        dropZone.addEventListener('click', () => {
            document.getElementById('file-input').click();
        });
    }

    // Handle file selection
    handleFileSelection(files) {
        const allowedTypes = [
            'application/pdf',
            'image/jpeg',
            'image/png',
            'image/gif',
            'application/msword',
            'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
            'text/plain'
        ];

        const validFiles = files.filter(file => {
            if (!allowedTypes.includes(file.type)) {
                window.app.showNotification(`File type not supported: ${file.name}`, 'warning');
                return false;
            }
            return true;
        });

        this.selectedFiles = [...this.selectedFiles, ...validFiles];
        this.updateFileList();
    }

    // Update selected files display
    updateFileList() {
        const fileList = document.getElementById('selected-files');
        if (!fileList) return;

        if (this.selectedFiles.length === 0) {
            fileList.innerHTML = '<div class="no-files">No files selected</div>';
            return;
        }

        fileList.innerHTML = this.selectedFiles.map((file, index) => `
            <div class="file-item">
                <div class="file-info">
                    <div class="file-name">${file.name}</div>
                    <div class="file-size">${window.app.formatFileSize(file.size)}</div>
                </div>
                <button type="button" class="remove-file" onclick="printManager.removeFile(${index})">
                    ×
                </button>
            </div>
        `).join('');
    }

    // Remove selected file
    removeFile(index) {
        this.selectedFiles.splice(index, 1);
        this.updateFileList();
    }

    // Clear all selected files
    clearSelectedFiles() {
        this.selectedFiles = [];
        this.updateFileList();
        document.getElementById('file-input').value = '';
    }

    // Submit print job
    async submitPrintJob() {
        if (this.selectedFiles.length === 0) {
            window.app.showNotification('Please select files to print', 'warning');
            return;
        }

        const formData = new FormData();
        const printer = document.getElementById('printer-select').value;
        const copies = document.getElementById('copies').value || 1;
        const pages = document.getElementById('pages').value;
        const duplex = document.getElementById('duplex').checked;
        const color = document.getElementById('color').checked;

        // Add files
        this.selectedFiles.forEach(file => {
            formData.append('file', file);
        });

        // Add print options
        if (printer) formData.append('printer', printer);
        formData.append('copies', copies);
        if (pages) formData.append('pages', pages);
        formData.append('duplex', duplex);
        formData.append('color', color);

        try {
            window.app.showProgressBar('print-progress', 0);

            const result = await window.app.uploadFile('/print', formData);

            window.app.showNotification(`Print job submitted successfully! Job ID: ${result.job_id}`, 'success');
            this.clearSelectedFiles();

            // Reset form
            document.getElementById('print-form').reset();
            this.updatePrinterSelect(); // Restore default selection

            // Refresh job list
            await this.loadPrintJobs();

        } catch (error) {
            console.error('Print job failed:', error);
            window.app.showNotification('Failed to submit print job', 'error');
        } finally {
            window.app.hideProgressBar('print-progress');
        }
    }

    // Update print jobs list
    async updateJobList() {
        try {
            this.printJobs = await window.app.apiCall('/print/jobs');
            this.renderJobList();
        } catch (error) {
            console.error('Failed to update job list:', error);
        }
    }

    // Render jobs list in UI
    renderJobList() {
        const jobsList = document.getElementById('jobs-list');
        if (!jobsList) return;

        if (this.printJobs.length === 0) {
            jobsList.innerHTML = '<div class="no-jobs">No print jobs</div>';
            return;
        }

        jobsList.innerHTML = this.printJobs.map(job => `
            <div class="job-item ${job.status}" data-job-id="${job.id}">
                <div class="job-header">
                    <div class="job-title">${job.document_name}</div>
                    <div class="job-status status-${job.status}">${job.status}</div>
                </div>
                <div class="job-details">
                    <div class="job-info">
                        <span>Printer: ${job.printer}</span>
                        <span>Copies: ${job.copies}</span>
                        <span>Pages: ${job.pages || 'All'}</span>
                    </div>
                    <div class="job-time">${window.app.formatDateTime(job.created_at)}</div>
                </div>
                <div class="job-actions">
                    ${job.status === 'pending' || job.status === 'processing' ?
            `<button class="btn btn-danger btn-sm" onclick="printManager.cancelJob('${job.id}')">Cancel</button>` :
            ''
        }
                    ${job.status === 'completed' || job.status === 'failed' ?
            `<button class="btn btn-secondary btn-sm" onclick="printManager.removeJobFromList('${job.id}')">Remove</button>` :
            ''
        }
                </div>
            </div>
        `).join('');
    }

    // Cancel a print job
    async cancelJob(jobId) {
        if (!confirm('Are you sure you want to cancel this print job?')) {
            return;
        }

        try {
            await window.app.apiCall(`/print/jobs/${jobId}`, { method: 'DELETE' });
            window.app.showNotification('Print job cancelled', 'success');
            await this.loadPrintJobs();
        } catch (error) {
            console.error('Failed to cancel job:', error);
            window.app.showNotification('Failed to cancel print job', 'error');
        }
    }

    // Remove job from display (for completed/failed jobs)
    removeJobFromList(jobId) {
        const jobElement = document.querySelector(`[data-job-id="${jobId}"]`);
        if (jobElement) {
            jobElement.remove();
        }

        // Also remove from local array
        this.printJobs = this.printJobs.filter(job => job.id !== jobId);
    }

    // Get printer status summary
    getPrinterStatusSummary() {
        const online = this.printers.filter(p => p.status === 'idle' || p.status === 'printing').length;
        const offline = this.printers.filter(p => p.status === 'offline').length;

        return { online, offline, total: this.printers.length };
    }

    // Update UI elements
    updateUI() {
        const statusSummary = this.getPrinterStatusSummary();
        const statusElement = document.getElementById('printer-status-summary');

        if (statusElement) {
            statusElement.innerHTML = `
                <div class="status-summary">
                    <div class="status-item">
                        <span class="count">${statusSummary.online}</span>
                        <span class="label">Online</span>
                    </div>
                    <div class="status-item">
                        <span class="count">${statusSummary.offline}</span>
                        <span class="label">Offline</span>
                    </div>
                    <div class="status-item">
                        <span class="count">${statusSummary.total}</span>
                        <span class="label">Total</span>
                    </div>
                </div>
            `;
        }
    }

    // Refresh all data
    async refresh() {
        await this.loadPrinters();
        await this.loadPrintJobs();
        this.updateUI();
        window.app.showNotification('Print manager refreshed', 'info');
    }
}

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    if (window.location.pathname === '/print') {
        window.printManager = new PrintManager();
    }
});