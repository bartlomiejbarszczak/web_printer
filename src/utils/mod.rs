use tokio::process::Command;

#[macro_export]
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


/// Helper function to get available disk space
pub async fn get_disk_space() -> Option<u64> {
    match Command::new("df").args(["-m", "."]).output().await {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(available) = parts[3].parse::<u64>() {
                        return Some(available);
                    }
                }
            }
            None
        },
        _ => None,
    }
}