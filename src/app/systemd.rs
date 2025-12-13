// Handles all interactions with the `systemctl` command.

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
    if let Ok(home) = env::var("HOME") {
        let config_path = PathBuf::from(home).join(".config/systemd/user");

        if let Ok(entries) = fs::read_dir(config_path) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {

                    if file_name.ends_with(".service") {
                        names.insert(file_name);
                    }
                }
            }
        }
    }
    names
}

pub fn get_user_services() -> Result<Vec<Service>> {
    let user_config_services = get_user_defined_services();


    // but sticking to your text parsing for simplicity, added --plain to ensure no colors/styling
    let output = Command::new("systemctl")
        .arg("--user")
        .arg("list-units")
        .arg("--type=service")
        .arg("--all")
        .arg("--no-pager")
        .arg("--no-legend")
        .arg("--plain")
        .output()
        .context("Failed to execute systemctl command")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("systemctl returned non-zero status"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();
    let mut seen_names = HashSet::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let name = parts[0].to_string();
        let is_config = user_config_services.contains(&name);

        services.push(Service {
            name: name.clone(),
            loaded_state: parts[1].to_string(),
            active_state: parts[2].to_string(),
            sub_state: parts[3].to_string(),
            is_user_config: is_config,
        });

        seen_names.insert(name);
    }


    // systemctl list-unit-files is VERY slow compared to list-units.
    // If you experience lag, consider removing this second command and only
    // showing loaded units. For now, I've left it but ensure it's plain text.
    let output_files = Command::new("systemctl")
        .arg("--user")
        .arg("list-unit-files")
        .arg("--type=service")
        .arg("--no-pager")
        .arg("--no-legend")
        .arg("--plain")
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
            if !seen_names.contains(name) {
                let is_config = user_config_services.contains(name);


                // if you really want to see every installed service on the OS.
                // Current logic shows EVERYTHING installed on the OS.
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
        .output() // This blocks!
        .context("Failed to fetch logs")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}