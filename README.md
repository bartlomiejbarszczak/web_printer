[//]: # (# Print & Scan Manager)

[//]: # ()
[//]: # (<div align="center">)

[//]: # ()
[//]: # (![Rust]&#40;https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white&#41;)

[//]: # (![Actix Web]&#40;https://img.shields.io/badge/actix--web-000000?style=for-the-badge&logo=actix&logoColor=white&#41;)

[//]: # (![SQLite]&#40;https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white&#41;)

[//]: # (![Raspberry Pi]&#40;https://img.shields.io/badge/-RaspberryPi-C51A4A?style=for-the-badge&logo=Raspberry-Pi&#41;)

[//]: # ()
[//]: # (A modern, web-based print and scan management system designed for Raspberry Pi Zero 2W, transforming your USB-only printer/scanner into a WiFi-accessible device.)

[//]: # ()
[//]: # ([Features]&#40;#features&#41; • [Screenshots]&#40;#screenshots&#41; • [Installation]&#40;#installation&#41; • [Usage]&#40;#usage&#41; • [Architecture]&#40;#architecture&#41;)

[//]: # ()
[//]: # (</div>)

[//]: # (---)

## Print & Scan Manager

Print & Scan Manager is a lightweight, high-performance web application that provides network access to USB printers and scanners through a Raspberry Pi. Built with Rust and Actix-web, it offers a responsive, modern interface for managing print jobs, scanning documents, and performing printer maintenance - all accessible from any device on your home network.

### Why This Project?

Many home printers and scanners lack WiFi connectivity, limiting their usability in modern home networks. This project solves that problem by:

- Converting USB-only devices into network-accessible resources
- Providing a clean, modern web interface accessible from any device
- Managing print and scan jobs with persistent storage
- Offering printer maintenance capabilities (nozzle cleaning/checking)
- Running efficiently on low-power Raspberry Pi Zero 2W hardware

## Features

### Print Management
- **Upload & Print** - Support for PDF, DOC, DOCX, TXT, JPG, and PNG files
- **Multiple Printers** - Manage and select from multiple connected printers
- **Advanced Options** - Configure copies, page ranges, duplex, color mode, and paper size
- **Job Tracking** - Real-time monitoring of print job status and progress
- **Job History** - View and manage completed print jobs

### Scan Management
- **High-Quality Scanning** - Support for resolutions from 150 to 1200 DPI
- **Multiple Formats** - Output to PDF, JPEG, PNG, or TIFF
- **Preview & Download** - Preview scanned images before downloading
- **Custom Settings** - Adjust color mode, brightness, contrast, and page size
- **File Management** - Organize and download scanned documents

### System Features
- **Printer Maintenance** - Nozzle cleaning and checking via ESCPUTIL
- **Real-time Status** - Monitor CUPS and SANE service availability
- **System Information** - View disk space and uptime statistics
- **Recent Activity** - Track recent print and scan operations
- **SQLite Database** - Persistent job history and metadata storage



## Technology Stack

- **Backend**: Rust with Actix-web framework
- **Database**: SQLite with sqlx
- **Frontend**: Vanilla JavaScript, HTML5, CSS3
- **System Integration**:
    - CUPS (Common Unix Printing System)
    - SANE (Scanner Access Now Easy)
    - ESCPUTIL (Epson printer utility)

## Installation

### Prerequisites

```bash
# Update system packages
sudo apt update && sudo apt upgrade -y

# Install CUPS
sudo apt install -y cups cups-client

# Install SANE
sudo apt install -y sane sane-utils

# Install ESCPUTIL (for Epson printers) - necessary for maintenance
sudo apt install -y escputil

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Printer Setup

1. Connect your printer via USB to the Raspberry Pi
2. Configure CUPS:
    ```bash
    # Add your user to the lpadmin group
    sudo usermod -aG lpadmin $USER
    
    # Access CUPS web interface at http://raspberry-pi-ip:631
    # Add and configure your printer
    ```

3. Configure SANE:
    ```bash
    # Test scanner detection
    scanimage -L
    
    # Verify scanner is detected
    sane-find-scanner
    ```

### Application Setup

1. Clone the repository:
    ```bash
    git clone https://github.com/bartlomiejbarszczak/web_printer.git
    cd web_printer
    ```

2. Build the application:
    ```bash
    cargo build --release
    ```

3. Run the application:
    ```bash
    ./target/release/web_printer
    ```

The application will be accessible at `http://raspberry-pi-ip:8080`

### Optional: Automated deploy on Raspberry PI with cross compile

1. Create a script (e.g., deploy_pi.sh)

2. Fill and add the following content
    ```bash
    #!/bin/bash
    PI_IP="<RaspberryPi IP address>"
    PI_USER="<PI username>"
    PROGRAM_NAME="web_printer"
    PI_DIRECTORY="Web" # or different directory name
    TARGET="aarch64-unknown-linux-gnu" # PI Zero 2W target
    
    echo "Cross-compiling for $TARGET..."
    cross build --target $TARGET --release
    
    echo "Copying executable to Pi..."
    scp target/$TARGET/release/$PROGRAM_NAME $PI_USER@$PI_IP:~/$PI_DIRECTORY
    
    echo "Copying static & templates files (css, images, js, html) to Pi..."
    scp -r static/ templates/ $PI_USER@$PI_IP:~/$PI_DIRECTORY
    
    echo "Setting executable permissions..."
    ssh $PI_USER@$PI_IP "chmod +x ~/$PI_DIRECTORY/$PROGRAM_NAME"
    
    echo "Done"
    ```
3. Make the script executable
4. Run docker
5. Run the script

### Optional: Run as System Service

1. Create a systemd service file:
    ```bash
    sudo nano /etc/systemd/system/print-scan-manager.service
    ```

2. Add the following content:
    ```ini
    [Unit]
    Description=Print & Scan Manager
    After=network.target cups.service
    
    [Service]
    Type=simple
    User=pi
    WorkingDirectory=/home/pi/print-scan-manager
    ExecStart=/home/pi/print-scan-manager/target/release/print-scan-manager
    Restart=always
    RestartSec=10
    
    [Install]
    WantedBy=multi-user.target
    ```

3. Enable and start the service:
    ```bash
    sudo systemctl daemon-reload
    sudo systemctl enable print-scan-manager
    sudo systemctl start print-scan-manager
    ```

## Usage

### Printing a Document

1. Navigate to the Print page
2. Click "New Print Job"
3. Select your file (PDF, DOC, DOCX, TXT, JPG, PNG)
4. Configure options:
    - Choose printer
    - Set number of copies
    - Specify page range (optional)
    - Enable duplex printing (if supported)
    - Choose color or monochrome
    - Select paper size
5. Click "Start Printing"

### Scanning a Document

1. Navigate to the Scan page
2. Click "New Scan"
3. Configure scan settings:
    - Select scanner
    - Choose resolution (150-1200 DPI)
    - Select output format (PDF, JPEG, PNG, TIFF)
    - Set color mode
    - Adjust brightness and contrast
    - Specify page size
4. Click "Start Scan"
5. Download or preview the scanned document once complete

### Printer Maintenance

From the dashboard:
- **Nozzle Check**: Prints a test pattern to verify nozzle condition
- **Clean Nozzles**: Performs a cleaning cycle to clear clogged nozzles


##  Configuration

### Database Optimizations

The application includes Pi Zero 2W-specific optimizations:
- WAL mode for better concurrency
- Optimized cache size
- Indexed queries for faster lookups


## Screenshots

### Dashboard
![Dashboard](images/dashboard.png)
*Main dashboard showing all key features: print, scan, maintenance, recent activity, and system info*

### Print Management
![Print Management](images/print.png)
*Complete print job management*

### Scan Management
![Scan Management](images/scan.png)
*Comprehensive scan interface*


## Project Structure

```
print-scan-manager/
├── src/
│   ├── main.rs                 # Application entry point
│   ├── database/
│   │   ├── mod.rs              # Database initialization
│   │   └── migrations.rs       # Database schema migrations
│   ├── handlers/
│   │   ├── mod.rs              # Handler utilities
│   │   ├── print.rs            # Print job handlers
│   │   ├── scan.rs             # Scan job handlers
│   │   └── system.rs           # System & maintenance handlers
│   ├── services/
│   │   ├── mod.rs              # Service layer
│   │   ├── cups.rs             # CUPS integration
│   │   ├── sane.rs             # SANE integration
│   │   └── escputil.rs         # ESCPUTIL integration
│   └── models/
│       ├── mod.rs              # Model definitions
│       ├── print_job.rs        # Print job model
│       └── scan_job.rs         # Scan job model
├── static/
│   ├── css/                    # Stylesheets
│   └── js/                     # Frontend JavaScript
├── templates/                  # HTML templates
├── uploads/                    # Temporary print file storage
├── scans/                      # Scanned document storage
└── data/                       # SQLite database location
```

## API Endpoints

### Print Endpoints
- `GET /api/printers` - List available printers
- `POST /api/print` - Submit print job
- `GET /api/print/jobs` - List all print jobs
- `GET /api/print/jobs/{id}` - Get specific print job
- `POST /api/print/jobs/{id}` - Cancel print job
- `DELETE /api/print/jobs/{id}` - Delete print job record

### Scan Endpoints
- `GET /api/scanners` - List available scanners
- `POST /api/scan` - Start scan job
- `GET /api/scan/jobs` - List all scan jobs
- `GET /api/scan/jobs/{id}` - Get specific scan job
- `GET /api/scan/download/{id}` - Download scanned file
- `DELETE /api/scan/jobs/{id}` - Delete scan job record
- `DELETE /api/scan/remove/{id}` - Delete scanned file

### System Endpoints
- `GET /api/system/status` - Get system status
- `GET /api/system/get-recent` - Get recent activity
- `POST /api/system/nozzle/check` - Perform nozzle check
- `POST /api/system/nozzle/clean` - Clean printer nozzles

## Future Development

### Planned Features

- [ ] **Cancel In-Progress Scans** - Ability to abort scanning operations
- [ ] **Job Queue System** 
    - Print job queuing
    - Scan job queuing
- [ ] **Enhanced Database Schema**
    - Store printer vendor and model information
    - Store scanner vendor and model information
- [ ] **Multi-page Scanning**

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Actix-web](https://actix.rs/) - Fast, pragmatic, Rust web framework
- [CUPS](https://www.cups.org/) - Common Unix Printing System
- [SANE](http://www.sane-project.org/) - Scanner Access Now Easy
- [SQLx](https://github.com/launchbadge/sqlx) - Rust SQL toolkit
- Inspired by the need for simple, accessible home printing solutions


