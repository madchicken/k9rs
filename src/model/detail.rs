use std::collections::BTreeMap;

/// Which tab is active in the detail view
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    Overview,
    Yaml,
    Events,
    Logs,
}

impl DetailTab {
    pub fn all() -> &'static [DetailTab] {
        &[
            DetailTab::Overview,
            DetailTab::Yaml,
            DetailTab::Events,
            DetailTab::Logs,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            DetailTab::Overview => "Overview",
            DetailTab::Yaml => "YAML",
            DetailTab::Events => "Events",
            DetailTab::Logs => "Logs",
        }
    }

    pub fn key_hint(&self) -> &'static str {
        match self {
            DetailTab::Overview => "1",
            DetailTab::Yaml => "2",
            DetailTab::Events => "3",
            DetailTab::Logs => "4",
        }
    }
}

/// Full detail of a single Kubernetes resource
#[derive(Debug, Clone)]
pub struct ResourceDetail {
    pub name: String,
    pub namespace: Option<String>,
    pub resource_type: String,
    pub age: String,
    pub phase: String,
    pub labels: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, String>,
    pub owner_references: Vec<OwnerRef>,
    pub conditions: Vec<Condition>,
    pub containers: Vec<ContainerInfo>,
    /// Pods running under this workload (for deployments, statefulsets, etc.)
    pub pods: Vec<PodInfo>,
    pub yaml: String,
    pub events: Vec<EventEntry>,
}

#[derive(Debug, Clone)]
pub struct OwnerRef {
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub type_: String,
    pub status: String,
    pub reason: String,
    pub message: String,
    pub last_transition: String,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub ready: bool,
    pub restart_count: i32,
    pub state: String,
    pub ports: String,
}

#[derive(Debug, Clone)]
pub struct PodInfo {
    pub name: String,
    pub ready: String,
    pub status: String,
    pub cpu: String,
    pub memory: String,
    pub restarts: i32,
    pub last_restart_time: String,
    pub last_restart_reason: String,
    pub node: String,
    pub ip: String,
    pub age: String,
}

#[derive(Debug, Clone)]
pub struct EventEntry {
    pub type_: String,
    pub reason: String,
    pub age: String,
    pub from: String,
    pub message: String,
}
