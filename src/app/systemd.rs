// Handles all interactions with the `systemctl` command.

use super::model::Service;
use anyhow::{Context, Result};
use std::process::Command;

pub enum ServiceAction {
    Start,
    Stop,
    Restart,
}

/// Fetches the list of user services from systemd.
/// We use `--user` to target ~/.config/systemd/user and /usr/lib/systemd/user
pub fn get_user_services() -> Result<Vec<Service>> {
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

    // Parse the output line by line
    // Expected format approx: unit_name loaded active sub description...
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        // Basic parsing strategy based on standard systemctl output
        services.push(Service {
            name: parts[0].to_string(),
            loaded_state: parts[1].to_string(),
            active_state: parts[2].to_string(),
            sub_state: parts[3].to_string(),
        });
    }

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

