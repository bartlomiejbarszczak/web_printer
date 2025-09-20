use tokio::process::Command;
use crate::models::{Scanner, ScanJob};
use crate::services::command_exists;

macro_rules! capitalize {
    ($s:expr) => {{
        let input = $s.to_lowercase();

        let mut chars: Vec<char> = input.chars().collect();
        if let Some(first_char) = chars.get_mut(0) {
            *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
        }
        chars.into_iter().collect::<String>()
    }};
}


pub struct SaneService;

impl SaneService {
    pub fn new() -> Self {
        Self
    }

    /// Check if SANE is available
    pub async fn is_available(&self) -> bool {
        command_exists("scanimage")

    }

    /// Get list of available scanners
    pub async fn get_scanners(&self) -> Result<Vec<Scanner>, String> {
        let output = Command::new("sane-find-scanner")
            .output()
            .await
            .map_err(|e| format!("Failed to execute sane-find-scanner: {}", e))?;

        if !output.status.success() {
            return Err("SANE service not available".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut scanners = Vec::new();

        for line in stdout.lines() {
            if line.contains("found") && line.contains("scanner") {
                // "found possible USB scanner (vendor=0x04b8 [EPSON], product=0x1142 [L3110 Series]) at libusb:001:002"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Some(at_pos) = line.find(" at ") {
                        let device_part = &line[at_pos + 4..];
                        let vendor = self.extract_vendor_from_line(line);
                        let model = self.extract_model_from_line(line);
                        let partial_name = device_part.trim();

                        let name = self.get_scanner_name_from_scanimage(partial_name).await?;


                        scanners.push(Scanner {
                            name,
                            vendor: vendor.unwrap_or("Unknown".to_string()),
                            model: model.unwrap_or("Scanner".to_string()),
                            device_type: "flatbed scanner".to_string(),
                        });
                    }
                }
            }
        }

        Ok(scanners)
    }

    async fn get_scanner_name_from_scanimage(&self, pattern_name: &str) -> Result<String, String> {
        let output = Command::new("scanimage")
            .arg("-L")
            .output()
            .await
            .map_err(|e| format!("Failed to execute scanimage: {}", e))?;


        if !output.status.success() {
            return Err("scanimage command failed".to_string());
        };

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if line.contains(pattern_name) {
                let name = line.split_once('`').unwrap().1.split_once('\'').unwrap().0;

                return Ok(name.to_string());
            }
        }

        Err("Couldnt find matching name".to_string())
    }

    /// Extract vendor from scanner line
    fn extract_vendor_from_line(&self, line: &str) -> Option<String> {
        // found possible USB scanner (vendor=0x04b8 [EPSON], product=0x1142 [L3110 Series]) at libusb:001:002
        if line.contains("vendor=") {
            let body = line.split("(").nth(1)?.split(")").nth(0)?;
            let vendor_name = body.split_once('[')?.1.split_once(']')?.0;

            return Some(capitalize!(vendor_name));
        };

        None
    }

    /// Extract model from scanner line
    fn extract_model_from_line(&self, line: &str) -> Option<String> {
        if line.contains("product=") {
            let body = line.split("(").nth(1)?.split(")").nth(0)?;
            let model_name = body.split(", ").nth(1)?.split("[").nth(1)?.split("]").nth(0)?;

            return Some(model_name.to_string())
        }

        None
    }

    /// Start a scan job
    pub async fn start_scan(&self, job: &ScanJob) -> Result<String, String> {
        let output_path = job.get_file_path()
            .ok_or("No output filename specified")?;

        let mut cmd = Command::new("scanimage");

        // Add device option
        cmd.args(["-d", &job.scanner]);

        // Add resolution
        cmd.args(["--resolution", &job.resolution.to_string()]);

        // Add format
        let format_arg = match job.format {
            crate::models::ScanFormat::Pdf => "pdf",
            crate::models::ScanFormat::Jpeg => "jpeg",
            crate::models::ScanFormat::Png => "png",
            crate::models::ScanFormat::Tiff => "tiff",
        };
        cmd.args(["--format", format_arg]);

        // Add color mode
        let mode_arg = match job.color_mode {
            crate::models::ColorMode::Color => "Color",
            crate::models::ColorMode::Grayscale => "Gray",
            crate::models::ColorMode::Monochrome => "Lineart",
        };
        cmd.args(["--mode", mode_arg]);

        // Add output file
        cmd.args(["-o", &output_path]);

        let output = cmd.output()
            .await
            .map_err(|e| format!("Failed to execute scanimage: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Scan failed: {}", stderr));
        }

        Ok(output_path)
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize!("hello"), "Hello");
        assert_eq!(capitalize!("hello world"), "Hello world");
        assert_eq!(capitalize!(""), "");
        assert_eq!(capitalize!("a"), "A");
        assert_eq!(capitalize!("HELLO"), "Hello")
    }
}