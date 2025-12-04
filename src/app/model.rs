// Defines the core data structures for the application.

/// Represents the status of a systemd service.
#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub active_state: String, // e.g., "active", "inactive"
    pub sub_state: String,    // e.g., "running", "dead", "exited"
    pub loaded_state: String, // e.g., "loaded", "not-found"
    pub is_user_config: bool,
}

impl Service {
    pub fn is_running(&self) -> bool {
        self.active_state == "active" && self.sub_state == "running"
    }
}
