/// A detected port from a pod's container spec
#[derive(Debug, Clone)]
pub struct PodPort {
    pub container_name: String,
    pub port: u16,
    pub protocol: String,
    pub name: Option<String>,
}

impl PodPort {
    pub fn display(&self) -> String {
        let name_suffix = self
            .name
            .as_ref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        format!(
            "{}:{}/{}{}",
            self.container_name, self.port, self.protocol, name_suffix
        )
    }
}

/// Status of a port forward
#[derive(Debug, Clone)]
pub enum PortForwardStatus {
    Active,
    Failed(String),
    Stopped,
}

impl PortForwardStatus {
    pub fn label(&self) -> String {
        match self {
            PortForwardStatus::Active => "Active".to_string(),
            PortForwardStatus::Failed(msg) => format!("Failed: {msg}"),
            PortForwardStatus::Stopped => "Stopped".to_string(),
        }
    }
}

/// A tracked port-forward entry
#[derive(Debug, Clone)]
pub struct PortForwardEntry {
    pub id: u64,
    pub pod_name: String,
    pub namespace: String,
    pub local_port: u16,
    pub remote_port: u16,
    pub status: PortForwardStatus,
    pub started_at: String,
}
