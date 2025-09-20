use tokio::process::Command;
use crate::services::command_exists;
use crate::services::cups::CupsService;


pub struct MaintenanceService;

impl MaintenanceService {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn is_available(&self) -> bool {
        command_exists("escputil")
    }

    pub async fn do_nozzle_heads_check(&self) -> Result<(), String> {
        execute_escputil("-n").await
    }

    pub async fn do_nozzle_heads_cleaning(&self) -> Result<(), String> {
        execute_escputil("-c").await
    }
}

// TODO choose which printer to clean
async fn execute_escputil(argument: &str) -> Result<(), String> {
    let service = CupsService::new();

    let printer_name = service.get_printers().await?
        .get(0)
        .map_or_else(|| "none".to_string(), |printer| printer.name.clone());


    let output = Command::new("escputil")
        .args(["-P", printer_name.as_str(), argument])
        .output()
        .await
        .map_err(|e| {e.to_string()})?;

    if !output.status.success() {
        return match argument {
            "-n" => Err("Error during checking nozzle heads".to_string()),
            "-c" => Err("Error during cleaning nozzle heads".to_string()),
            _ => Err("Error executing escputil".to_string()),
        }
    }

    Ok(())
}
