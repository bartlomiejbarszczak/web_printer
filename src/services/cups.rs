use tokio::process::Command;
use crate::models::{Printer, PrintJob};
use crate::services::command_exists;
use crate::capitalize;

pub struct CupsService;

impl CupsService {
    pub fn new() -> Self {
        Self
    }

    /// Check if CUPS is available and running
    pub async fn is_available(&self) -> bool {
        command_exists("lpstat")
    }

    /// Get list of available printers
    pub async fn get_printers(&self) -> Result<Vec<Printer>, String> {
        let output = Command::new("lpstat")
            .args(["-p", "-d"])
            .output()
            .await
            .map_err(|e| format!("Failed to execute lpstat: {}", e))?;

        if !output.status.success() {
            return Err("CUPS service not available".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut printers = Vec::new();
        let mut default_printer = None;

        // Parse lpstat output
        for line in stdout.lines() {
            if line.starts_with("printer ") {
                // "printer EPSON_L3110_Series is idle.  enabled since <DATE>"
                if let Some(name_part) = line.strip_prefix("printer ") {
                    if let Some(space_pos) = name_part.find(' ') {
                        let name = name_part[..space_pos].to_string();
                        let status = if line.contains("idle") {
                            "idle"
                        } else if line.contains("printing") {
                            "printing"
                        } else if line.contains("stopped") {
                            "stopped"
                        } else {
                            "unknown"
                        }.to_string();

                        let parts = name.split("_").collect::<Vec<&str>>().iter().map(|x| {capitalize!(x)}).collect::<Vec<String>>();
                        let vendor = parts.get(0).map_or_else(|| "Unknown".to_string(), |x| x.to_string());
                        let model = parts.get(1..).map_or_else(|| "Unknown".to_string(), |x| x.join(" "));

                        printers.push(Printer {
                            name: name.clone(),
                            vendor,
                            model,
                            description: self.get_printer_description(&name).await.unwrap_or(name.clone()),
                            status,
                            location: self.get_printer_location(&name).await,
                            is_default: false,
                        });
                    }
                }
            } else if line.starts_with("system default destination: ") {
                default_printer = line.strip_prefix("system default destination: ")
                    .map(|s| s.trim().to_string());
            } else if line.starts_with("lpstat: ") {
                return Err("No printer is available".to_string())
            }
        }

        // Mark default printer
        if let Some(default_name) = default_printer {
            for printer in &mut printers {
                if printer.name == default_name {
                    printer.is_default = true;
                    break;
                }
            }
        }

        Ok(printers)
    }

    /// Get printer description from CUPS
    async fn get_printer_description(&self, printer_name: &str) -> Option<String> {
        let output = Command::new("lpstat")
            .args(["-p", printer_name, "-v"])
            .output()
            .await
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("usb:") {
                    return Some(format!("USB Printer [{}]", printer_name));
                } else if line.contains("device-uri") {
                    return Some(format!("{} [description: {}]", printer_name, line));
                }
            }
        }

        None
    }

    /// Get printer location from CUPS
    async fn get_printer_location(&self, printer_name: &str) -> Option<String> {
        let output = Command::new("lpstat")
            .args(["-p", printer_name, "-l"])
            .output()
            .await
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("Location:") {
                    return Some(line.to_string());
                }
            }
        }

        None
    }

    /// Submit a print job to CUPS
    pub async fn submit_print_job(&self, job: &PrintJob, file_path: &str) -> Result<i32, String> {
        let mut cmd = Command::new("lp");

        cmd.args(["-d", &job.printer]);

        // Number of copies
        if job.copies > 1 {
            cmd.args(["-n", &job.copies.to_string()]);
        }

        // Page range if specified
        if let Some(ref pages) = job.pages {
            if pages.len() > 0 {
                cmd.args(["-P", pages]);
            }
        }

        // Duplex option
        if job.duplex {
            cmd.args(["-o", "sides=two -sided-long-edge"]);
        }

        // Set color
        let color_mode = match job.color {
            true => "COLOR".to_string(),
            false => "MONO".to_string()
        };
        cmd.args(["-o", format!("Ink={color_mode}").as_str()]);

        // Set media size
        let media_size = capitalize!(job.page_size.to_string());
        cmd.args(["-o", format!("PageSize={media_size}").as_str()]);

        // Add the file to print
        cmd.arg(file_path);

        let output = cmd.output().await
            .map_err(|e| format!("Failed to execute lp command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Print job failed: {}", stderr));
        }

        // Parse job ID from output
        let stdout = String::from_utf8_lossy(&output.stdout);

        //request id is EPSON_L3110_Series-66 (1 file(s))
        if let Some(job_id_str) = stdout.split_whitespace().nth(3) {
            let id = job_id_str.split('-').last().unwrap().parse::<i32>();
            log::info!("Request is like: {job_id_str}");
            match id {
                Ok(id) => Ok(id),
                Err(e) => {
                    log::error!("Failed to parse job id: {}", e);
                    Err(format!("Prase ID error: {}", e))},
            }
        } else {
            Err("No job ID returned".to_string())
        }
    }


    
    /// Get status of a specific print job
    pub async fn get_job_status(&self, job_id: i32) -> Result<String, String> {
        let active_output = Command::new("lpstat")
            .args(["-o"])
            .output()
            .await
            .map_err(|e| format!("Failed to execute lpstat: {}", e))?;

        if active_output.status.success() {
            let stdout = String::from_utf8_lossy(&active_output.stdout);
            for line in stdout.lines() {
                log::info!("{}", line);
                if let Some(job_part) = line.split_whitespace().next() {
                    if let Some(parsed_job_id) = job_part.split('-').last() {
                        if parsed_job_id == job_id.to_string() {
                            return Ok("active".to_string());
                        }
                    }
                }
            }
        }

        let completed_output = Command::new("lpstat")
            .args(["-W", "completed", "-o"])
            .output()
            .await
            .map_err(|e| format!("Failed to execute lpstat: {}", e))?;

        if !completed_output.status.success() {
            return Err("Failed to get completed job status".to_string());
        }

        let stdout = String::from_utf8_lossy(&completed_output.stdout);
        for line in stdout.lines() {
            if let Some(job_part) = line.split_whitespace().next() {
                if let Some(parsed_job_id) = job_part.split('-').last() {
                    if parsed_job_id == job_id.to_string() {
                        return Ok("completed".to_string());
                    }
                }
            }
        }

        Err(format!("Job {} not found", job_id))
    }


    /// Cancel a print job
    pub async fn cancel_job(&self, printer_name: &str, job_id: i32) -> Result<(), String> {
        let output = Command::new("cancel")
            .arg(format!("{}-{}",printer_name, job_id))
            .output()
            .await
            .map_err(|e| format!("Failed to execute cancel command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to cancel job: {}", stderr));
        }

        Ok(())
    }

    /// Get all active print jobs
    pub async fn get_active_jobs(&self) -> Result<Vec<(i32, String, String)>, String> {
        let output = Command::new("lpstat")
            .arg("-o")
            .output()
            .await
            .map_err(|e| format!("Failed to execute lpstat: {}", e))?;

        if !output.status.success() {
            return Ok(Vec::new()); // No active jobs
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut jobs = Vec::new();

        for line in stdout.lines() {
            // <Printer_name>-<Job_id>    <User_name>      1024   <Date>"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Some(dash_pos) = parts[0].rfind('-') {
                    let printer_name = &parts[0][..dash_pos];
                    if let Ok(job_id) = parts[0][dash_pos + 1..].parse::<i32>() {
                        let username = parts[1];
                        jobs.push((job_id, printer_name.to_string(), username.to_string()));
                    }
                }
            }
        }

        Ok(jobs)
    }
}



#[tokio::test]
async fn test_is_cups_available() {
    let service = CupsService::new();

    let available = service.is_available().await;

    assert_eq!(true, available);
}


#[tokio::test]
async fn test_get_printers() -> Result<(), String> {
    let service = CupsService::new();

    match service.is_available().await {
        true => (),
        false => return Err("Failed to check for available printers".to_string()),
    };

    let printers = service.get_printers().await?;

    assert_ne!(0, printers.len());

    Ok(())
}


#[tokio::test]
async fn test_get_printer_metadata() -> Result<(), String> {
    let service = CupsService::new();

    match service.is_available().await {
        true => (),
        false => return Err("Failed to check for available printers".to_string()),
    }

    let printers = service.get_printers().await?;

    let printer = match printers.first() {
        Some(printer) => printer,
        None => return Err("Failed to find printer".to_string()),
    };

    assert_eq!("EPSON_L3110_Series_raspberrypi".to_string(), printer.name);
    assert_eq!("idle".to_string(), printer.status);
    assert_ne!("".to_string(), printer.description);


    let description = service.get_printer_description(printer.name.clone().as_str()).await;

    if let Some(printer_description) = description {
        match printer_description.as_str() {
            "" => return Err(format!("Printer ({}) description not found", printer.name)),
            _ => ()
        }
    }

    Ok(())
}


