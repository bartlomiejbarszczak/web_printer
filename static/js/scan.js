// Scan management JavaScript
// Handles all scan-related functionality

class ScanManager {
    constructor() {
        this.scanners = [];
        this.scanJobs = [];
        this.scanSettings = {
            resolution: 300,
            format: 'pdf',
            color_mode: 'color',
            page_size: 'a4',
            brightness: 0,
            contrast: 0
        };
        this.init();
    }

    async init() {
        await this.loadScanners();
        await this.loadScanJobs();
        this.setupEventListeners();
        this.initializeSettings();
        this.updateUI();
    }

    // Load available scanners
    async loadScanners() {
        try {
            this.scanners = await window.app.apiCall('/api/scanners');
            this.updateScannerSelect();
        } catch (error) {
            console.error('Failed to load scanners:', error);
            window.app.showNotification('Failed to load scanners', 'error');
        }
    }

    // Load current scan jobs
    async loadScanJobs() {
        try {
            this.scanJobs = await window.app.apiCall('/scan/jobs');
            this.updateJobList();
        } catch (error) {
            console.error('Failed to load scan jobs:', error);
        }
    }

    // Update scanner selection dropdown
    updateScannerSelect() {
        const select = document.getElementById('scanner-select');
        if (!select) return;

        select.innerHTML = '<option value="">Select Scanner</option>';

        this.scanners.forEach(scanner => {
            const option = document.createElement('option');
            option.value = scanner.name;
            option.textContent = `${scanner.vendor} ${scanner.model}`;
            select.appendChild(option);
        });

        // Select first scanner by default
        if (this.scanners.length > 0 && !select.value) {
            select.value = this.scanners[0].name;
        }
    }

    // Setup event listeners
    setupEventListeners() {
        // Scan form submission
        const scanForm = document.getElementById('scan-form');
        if (scanForm) {
            scanForm.addEventListener('submit', (e) => {
                e.preventDefault();
                this.startScan();
            });
        }

        // Preview scan button
        const previewBtn = document.getElementById('preview-scan');
        if (previewBtn) {
            previewBtn.addEventListener('click', () => {
                this.startPreviewScan();
            });
        }

        // Settings change handlers
        const settingsInputs = document.querySelectorAll('.scan-setting');
        settingsInputs.forEach(input => {
            input.addEventListener('change', (e) => {
                this.updateSetting(e.target.name, e.target.value);
            });
        });

        // Range input display updates
        const rangeInputs = document.querySelectorAll('input[type="range"]');
        rangeInputs.forEach(input => {
            input.addEventListener('input', (e) => {
                const display = document.getElementById(`${e.target.id}-value`);
                if (display) {
                    display.textContent = e.target.value;
                }
            });
        });

        // Preset buttons
        const presetButtons = document.querySelectorAll('.preset-btn');
        presetButtons.forEach(btn => {
            btn.addEventListener('click', (e) => {
                const preset = e.target.dataset.preset;
                this.applyPreset(preset);
            });
        });

        // Advanced settings toggle
        const advancedToggle = document.getElementById('advanced-toggle');
        const advancedSettings = document.getElementById('advanced-settings');
        if (advancedToggle && advancedSettings) {
            advancedToggle.addEventListener('click', () => {
                advancedSettings.classList.toggle('hidden');
                advancedToggle.textContent = advancedSettings.classList.contains('hidden')
                    ? 'Show Advanced Settings'
                    : 'Hide Advanced Settings';
            });
        }
    }

    // Initialize settings from form
    initializeSettings() {
        const form = document.getElementById('scan-form');
        if (!form) return;

        const formData = new FormData(form);
        for (let [key, value] of formData.entries()) {
            if (this.scanSettings.hasOwnProperty(key)) {
                this.scanSettings[key] = value;
            }
        }
    }

    // Update a specific setting
    updateSetting(name, value) {
        if (this.scanSettings.hasOwnProperty(name)) {
            // Convert numeric values
            if (name === 'resolution' || name === 'brightness' || name === 'contrast') {
                value = parseInt(value);
            }
            this.scanSettings[name] = value;
        }
    }

    // Apply scan presets
    applyPreset(preset) {
        const presets = {
            document: {
                resolution: 300,
                format: 'pdf',
                color_mode: 'grayscale',
                page_size: 'a4',
                brightness: 0,
                contrast: 10
            },
            photo: {
                resolution: 600,
                format: 'jpg',
                color_mode: 'color',
                page_size: 'a4',
                brightness: 0,
                contrast: 0
            },
            text: {
                resolution: 400,
                format: 'pdf',
                color_mode: 'lineart',
                page_size: 'a4',
                brightness: 0,
                contrast: 20
            },
            draft: {
                resolution: 150,
                format: 'pdf',
                color_mode: 'grayscale',
                page_size: 'a4',
                brightness: 0,
                contrast: 0
            }
        };

        if (presets[preset]) {
            this.scanSettings = { ...presets[preset] };
            this.updateFormFromSettings();
            window.app.showNotification(`Applied ${preset} preset`, 'info');
        }
    }

    // Update form inputs from settings
    updateFormFromSettings() {
        Object.keys(this.scanSettings).forEach(key => {
            const input = document.querySelector(`[name="${key}"]`);
            if (input) {
                input.value = this.scanSettings[key];

                // Update range display
                if (input.type === 'range') {
                    const display = document.getElementById(`${input.id}-value`);
                    if (display) {
                        display.textContent = this.scanSettings[key];
                    }
                }
            }
        });
    }

    // Start a scan job
    async startScan() {
        const scanner = document.getElementById('scanner-select').value;
        if (!scanner) {
            window.app.showNotification('Please select a scanner', 'warning');
            return;
        }

        // Update settings from form
        this.collectSettingsFromForm();

        const scanData = {
            scanner: scanner,
            ...this.scanSettings
        };

        try {
            window.app.showProgressBar('scan-progress', 0);

            const result = await window.app.apiCall('/scan', {
                method: 'POST',
                body: JSON.stringify(scanData)
            });

            window.app.showNotification(`Scan started! Job ID: ${result.job_id}`, 'success');

            // Start polling for this job
            this.pollScanJob(result.job_id);

            // Refresh job list
            await this.loadScanJobs();

        } catch (error) {
            console.error('Scan failed:', error);
            window.app.showNotification('Failed to start scan', 'error');
        } finally {
            window.app.hideProgressBar('scan-progress');
        }
    }

    // Start a preview scan (lower resolution, quick)
    async startPreviewScan() {
        const scanner = document.getElementById('scanner-select').value;
        if (!scanner) {
            window.app.showNotification('Please select a scanner', 'warning');
            return;
        }

        // Create preview settings (lower resolution for speed)
        const previewSettings = {
            ...this.scanSettings,
            resolution: 150,
            format: 'jpg'
        };

        const scanData = {
            scanner: scanner,
            ...previewSettings
        };

        try {
            window.app.showProgressBar('preview-progress', 0);

            const result = await window.app.apiCall('/scan', {
                method: 'POST',
                body: JSON.stringify(scanData)
            });

            window.app.showNotification('Preview scan started', 'info');

            // Poll for preview completion
            this.pollScanJob(result.job_id, true);

        } catch (error) {
            console.error('Preview scan failed:', error);
            window.app.showNotification('Failed to start preview scan', 'error');
        } finally {
            window.app.hideProgressBar('preview-progress');
        }
    }

    // Collect settings from form inputs
    collectSettingsFromForm() {
        const form = document.getElementById('scan-form');
        if (!form) return;

        const formData = new FormData(form);
        for (let [key, value] of formData.entries()) {
            if (this.scanSettings.hasOwnProperty(key)) {
                // Convert numeric values
                if (key === 'resolution' || key === 'brightness' || key === 'contrast') {
                    value = parseInt(value);
                }
                this.scanSettings[key] = value;
            }
        }
    }

    // Poll scan job status
    async pollScanJob(jobId, isPreview = false) {
        const pollInterval = setInterval(async () => {
            try {
                const job = await window.app.apiCall(`/scan/jobs/${jobId}`);

                if (job.status === 'completed') {
                    clearInterval(pollInterval);

                    if (isPreview) {
                        this.showPreview(job);
                        window.app.showNotification('Preview scan completed', 'success');
                    } else {
                        window.app.showNotification('Scan completed successfully', 'success');
                    }

                    await this.loadScanJobs();

                } else if (job.status === 'failed') {
                    clearInterval(pollInterval);
                    window.app.showNotification('Scan failed: ' + (job.error || 'Unknown error'), 'error');
                    await this.loadScanJobs();
                }

                // Update progress if available
                if (job.progress !== undefined) {
                    const progressId = isPreview ? 'preview-progress' : 'scan-progress';
                    window.app.updateProgressBar(
                        document.querySelector(`#${progressId} .progress-bar`),
                        job.progress
                    );
                }

            } catch (error) {
                console.error('Error polling scan job:', error);
                clearInterval(pollInterval);
            }
        }, 1000);

        // Stop polling after 5 minutes
        setTimeout(() => {
            clearInterval(pollInterval);
        }, 300000);
    }

    // Show preview image
    showPreview(job) {
        const previewContainer = document.getElementById('scan-preview');
        if (!previewContainer) return;

        previewContainer.innerHTML = `
            <div class="preview-header">
                <h3>Scan Preview</h3>
                <button class="btn btn-secondary btn-sm" onclick="scanManager.closePreview()">Close</button>
            </div>
            <div class="preview-image">
                <img src="/api/scan/download/${job.id}" alt="Scan Preview" />
            </div>
            <div class="preview-actions">
                <button class="btn btn-primary" onclick="scanManager.acceptPreview()">Accept & Scan</button>
                <button class="btn btn-secondary" onclick="scanManager.adjustSettings()">Adjust Settings</button>
            </div>
        `;

        previewContainer.classList.remove('hidden');
    }

    // Close preview
    closePreview() {
        const previewContainer = document.getElementById('scan-preview');
        if (previewContainer) {
            previewContainer.classList.add('hidden');
            previewContainer.innerHTML = '';
        }
    }

    // Accept preview and start full scan
    acceptPreview() {
        this.closePreview();
        this.startScan();
    }

    // Adjust settings (close preview and show settings)
    adjustSettings() {
        this.closePreview();
        const advancedSettings = document.getElementById('advanced-settings');
        if (advancedSettings && advancedSettings.classList.contains('hidden')) {
            document.getElementById('advanced-toggle').click();
        }
    }

    // Update scan jobs list
    async updateJobList() {
        try {
            this.scanJobs = await window.app.apiCall('/scan/jobs');
            this.renderJobList();
        } catch (error) {
            console.error('Failed to update job list:', error);
        }
    }

    // Render jobs list in UI
    renderJobList() {
        const jobsList = document.getElementById('scan-jobs-list');
        if (!jobsList) return;

        if (this.scanJobs.length === 0) {
            jobsList.innerHTML = '<div class="no-jobs">No scan jobs</div>';
            return;
        }

        jobsList.innerHTML = this.scanJobs.map(job => `
            <div class="job-item ${job.status}" data-job-id="${job.id}">
                <div class="job-header">
                    <div class="job-title">Scan Job #${job.id}</div>
                    <div class="job-status status-${job.status}">${job.status}</div>
                </div>
                <div class="job-details">
                    <div class="job-info">
                        <span>Scanner: ${job.scanner}</span>
                        <span>Resolution: ${job.resolution}dpi</span>
                        <span>Format: ${job.format.toUpperCase()}</span>
                        <span>Mode: ${job.color_mode}</span>
                    </div>
                    <div class="job-time">${window.app.formatDateTime(job.created_at)}</div>
                </div>
                ${job.progress !== undefined && job.status === 'processing' ?
            `<div class="job-progress">
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: ${job.progress}%"></div>
                            <div class="progress-text">${Math.round(job.progress)}%</div>
                        </div>
                    </div>` : ''
        }
                <div class="job-actions">
                    ${job.status === 'completed' ?
            `<button class="btn btn-primary btn-sm" onclick="scanManager.downloadScan('${job.id}')">Download</button>
                         <button class="btn btn-secondary btn-sm" onclick="scanManager.viewScan('${job.id}')">View</button>` :
            ''
        }
                    ${job.status === 'processing' ?
            `<button class="btn btn-danger btn-sm" onclick="scanManager.cancelJob('${job.id}')">Cancel</button>` :
            ''
        }
                    ${job.status === 'completed' || job.status === 'failed' ?
            `<button class="btn btn-secondary btn-sm" onclick="scanManager.removeJobFromList('${job.id}')">Remove</button>` :
            ''
        }
                </div>
            </div>
        `).join('');
    }

    // Download scanned document
    downloadScan(jobId) {
        const downloadUrl = `/api/scan/download/${jobId}`;
        const link = document.createElement('a');
        link.href = downloadUrl;
        link.download = `scan_${jobId}`;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);

        window.app.showNotification('Download started', 'info');
    }

    // View scanned document (open in new tab)
    viewScan(jobId) {
        const viewUrl = `/api/scan/download/${jobId}`;
        window.open(viewUrl, '_blank');
    }

    // Cancel scan job
    async cancelJob(jobId) {
        if (!confirm('Are you sure you want to cancel this scan job?')) {
            return;
        }

        try {
            await window.app.apiCall(`/scan/jobs/${jobId}`, { method: 'DELETE' });
            window.app.showNotification('Scan job cancelled', 'success');
            await this.loadScanJobs();
        } catch (error) {
            console.error('Failed to cancel job:', error);
            window.app.showNotification('Failed to cancel scan job', 'error');
        }
    }

    // Remove job from display
    removeJobFromList(jobId) {
        const jobElement = document.querySelector(`[data-job-id="${jobId}"]`);
        if (jobElement) {
            jobElement.remove();
        }

        // Also remove from local array
        this.scanJobs = this.scanJobs.filter(job => job.id !== jobId);
    }

    // Get scanner status summary
    getScannerStatusSummary() {
        return {
            available: this.scanners.length,
            total: this.scanners.length
        };
    }

    // Update UI elements
    updateUI() {
        const statusSummary = this.getScannerStatusSummary();
        const statusElement = document.getElementById('scanner-status-summary');

        if (statusElement) {
            statusElement.innerHTML = `
                <div class="status-summary">
                    <div class="status-item">
                        <span class="count">${statusSummary.available}</span>
                        <span class="label">Available</span>
                    </div>
                    <div class="status-item">
                        <span class="count">${this.scanJobs.filter(j => j.status === 'processing').length}</span>
                        <span class="label">Active</span>
                    </div>
                    <div class="status-item">
                        <span class="count">${this.scanJobs.filter(j => j.status === 'completed').length}</span>
                        <span class="label">Completed</span>
                    </div>
                </div>
            `;
        }

        // Update current settings display
        const settingsDisplay = document.getElementById('current-settings');
        if (settingsDisplay) {
            settingsDisplay.innerHTML = `
                <div class="settings-summary">
                    <div class="setting-item">
                        <span class="label">Resolution:</span>
                        <span class="value">${this.scanSettings.resolution}dpi</span>
                    </div>
                    <div class="setting-item">
                        <span class="label">Format:</span>
                        <span class="value">${this.scanSettings.format.toUpperCase()}</span>
                    </div>
                    <div class="setting-item">
                        <span class="label">Color:</span>
                        <span class="value">${this.scanSettings.color_mode}</span>
                    </div>
                    <div class="setting-item">
                        <span class="label">Size:</span>
                        <span class="value">${this.scanSettings.page_size.toUpperCase()}</span>
                    </div>
                </div>
            `;
        }
    }

    // Multi-page scanning support
    async startMultiPageScan() {
        const scanner = document.getElementById('scanner-select').value;
        if (!scanner) {
            window.app.showNotification('Please select a scanner', 'warning');
            return;
        }

        this.multiPageScan = {
            pages: [],
            currentPage: 1,
            scanner: scanner,
            settings: { ...this.scanSettings }
        };

        window.app.showModal('multi-page-modal');
        this.scanNextPage();
    }

    // Scan next page in multi-page sequence
    async scanNextPage() {
        const modal = document.getElementById('multi-page-modal');
        const status = modal.querySelector('.multi-page-status');

        if (status) {
            status.textContent = `Scanning page ${this.multiPageScan.currentPage}...`;
        }

        try {
            const scanData = {
                scanner: this.multiPageScan.scanner,
                ...this.multiPageScan.settings,
                format: 'jpg' // Individual pages as JPG
            };

            const result = await window.app.apiCall('/scan', {
                method: 'POST',
                body: JSON.stringify(scanData)
            });

            // Wait for completion
            const job = await this.waitForScanCompletion(result.job_id);
            this.multiPageScan.pages.push(job);
            this.multiPageScan.currentPage++;

            // Update UI
            this.updateMultiPageUI();

        } catch (error) {
            console.error('Multi-page scan failed:', error);
            window.app.showNotification('Failed to scan page', 'error');
        }
    }

    // Wait for scan completion
    async waitForScanCompletion(jobId) {
        return new Promise((resolve, reject) => {
            const pollInterval = setInterval(async () => {
                try {
                    const job = await window.app.apiCall(`/scan/jobs/${jobId}`);

                    if (job.status === 'completed') {
                        clearInterval(pollInterval);
                        resolve(job);
                    } else if (job.status === 'failed') {
                        clearInterval(pollInterval);
                        reject(new Error('Scan failed'));
                    }
                } catch (error) {
                    clearInterval(pollInterval);
                    reject(error);
                }
            }, 1000);
        });
    }

    // Update multi-page scan UI
    updateMultiPageUI() {
        const modal = document.getElementById('multi-page-modal');
        const pagesList = modal.querySelector('.pages-list');
        const status = modal.querySelector('.multi-page-status');

        if (pagesList) {
            pagesList.innerHTML = this.multiPageScan.pages.map((page, index) => `
                <div class="page-item">
                    <img src="/api/scan/download/${page.id}" alt="Page ${index + 1}" />
                    <span>Page ${index + 1}</span>
                </div>
            `).join('');
        }

        if (status) {
            status.textContent = `${this.multiPageScan.pages.length} pages scanned. Insert next page or finish.`;
        }
    }

    // Finish multi-page scan and create PDF
    async finishMultiPageScan() {
        try {
            const pageIds = this.multiPageScan.pages.map(p => p.id);

            // Call API to combine pages into PDF
            const result = await window.app.apiCall('/scan/combine', {
                method: 'POST',
                body: JSON.stringify({
                    pages: pageIds,
                    format: 'pdf',
                    title: `Multi-page scan ${new Date().toISOString()}`
                })
            });

            window.app.showNotification('Multi-page scan completed!', 'success');
            window.app.hideModal('multi-page-modal');

            await this.loadScanJobs();

        } catch (error) {
            console.error('Failed to combine pages:', error);
            window.app.showNotification('Failed to create multi-page document', 'error');
        }
    }

    // Refresh all data
    async refresh() {
        await this.loadScanners();
        await this.loadScanJobs();
        this.updateUI();
        window.app.showNotification('Scan manager refreshed', 'info');
    }
}

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    if (window.location.pathname === '/scan') {
        window.scanManager = new ScanManager();
    }
});