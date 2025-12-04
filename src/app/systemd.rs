use super::model::Service;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

pub enum ServiceAction {
    Start,
    Stop,
    Restart,
}

fn get_user_defined_services() -> HashSet<String> {
    let mut names = HashSet::new();
    // Get home directory
    if let Ok(home) = env::var("HOME") {
        let config_path = PathBuf::from(home).join(".config/systemd/user");

        // Read directory if it exists
        if let Ok(entries) = fs::read_dir(config_path) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    // We assume anything ending in .service is a relevant unit
                    if file_name.ends_with(".service") {
                        names.insert(file_name);
                    }
                }
            }
        }
    }
    names
}

/// Fetches the list of user services from systemd.
/// We use `--user` to target ~/.config/systemd/user and /usr/lib/systemd/user
pub fn get_user_services() -> Result<Vec<Service>> {
    let user_config_services = get_user_defined_services();

    // We filter for services, --all to see inactive ones, and use no-legend/no-pager for parsing safety.
    let output = Command::new("systemctl")
        .arg("--user")
        .arg("list-units")
        .arg("--type=service")
        .arg("--all")
        .arg("--no-pager")
        .arg("--no-legend")
        .output()
        .context("Failed to execute systemctl command")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("systemctl returned non-zero status"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();
    let mut seen_names = HashSet::new();

    // Parse the output line by line
    // Expected format approx: unit_name loaded active sub description...
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let name = parts[0].to_string();
        let is_config = user_config_services.contains(&name);

        // Basic parsing strategy based on standard systemctl output
        services.push(Service {
            name: name.clone(),
            loaded_state: parts[1].to_string(),
            active_state: parts[2].to_string(),
            sub_state: parts[3].to_string(),
            is_user_config: is_config,
        });

        seen_names.insert(name);
    }

    let output_files = Command::new("systemctl")
        .arg("--user")
        .arg("list-unit-files")
        .arg("--type=service")
        .arg("--no-pager")
        .arg("--no-legend")
        .output()
        .context("Failed to execute systemctl list-unit-files")?;

    if output_files.status.success() {
        let stdout_files = String::from_utf8_lossy(&output_files.stdout);
        for line in stdout_files.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let name = parts[0];
            // If we haven't seen this service in the loaded list, add it as unloaded/inactive
            if !seen_names.contains(name) {
                let is_config = user_config_services.contains(name);

                services.push(Service {
                    name: name.to_string(),
                    loaded_state: "unloaded".to_string(),
                    active_state: "inactive".to_string(),
                    sub_state: "dead".to_string(),
                    is_user_config: is_config,
                });
            }
        }
    }

    services.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(services)
}

pub fn control_service(service_name: &str, action: ServiceAction) -> Result<()> {
    let action_str = match action {
        ServiceAction::Start => "start",
        ServiceAction::Stop => "stop",
        ServiceAction::Restart => "restart",
    };

    let status = Command::new("systemctl")
        .arg("--user")
        .arg(action_str)
        .arg(service_name)
        .status()
        .context(format!("Failed to {} service {}", action_str, service_name))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to perform action on service"))
    }
}


pub fn get_service_logs(service_name: &str) -> Result<Vec<String>> {
    let output = Command::new("journalctl")
        .arg("--user")
        .arg("-u")
        .arg(service_name)
        .arg("-n")
        .arg("100")
        .arg("--no-pager")
        .output()
        .context("Failed to fetch logs")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}