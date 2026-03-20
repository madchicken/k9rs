/// A resource entry for the sidebar
pub struct ResourceEntry {
    pub display_name: &'static str,
    pub api_name: &'static str,
    pub category: &'static str,
}

/// All supported resource types, grouped by category
pub const RESOURCES: &[ResourceEntry] = &[
    // Workloads
    ResourceEntry { display_name: "Pods", api_name: "pods", category: "Workloads" },
    ResourceEntry { display_name: "Deployments", api_name: "deployments", category: "Workloads" },
    ResourceEntry { display_name: "StatefulSets", api_name: "statefulsets", category: "Workloads" },
    ResourceEntry { display_name: "DaemonSets", api_name: "daemonsets", category: "Workloads" },
    ResourceEntry { display_name: "ReplicaSets", api_name: "replicasets", category: "Workloads" },
    ResourceEntry { display_name: "Jobs", api_name: "jobs", category: "Workloads" },
    ResourceEntry { display_name: "CronJobs", api_name: "cronjobs", category: "Workloads" },
    // Config
    ResourceEntry { display_name: "ConfigMaps", api_name: "configmaps", category: "Config" },
    ResourceEntry { display_name: "Secrets", api_name: "secrets", category: "Config" },
    ResourceEntry { display_name: "ServiceAccounts", api_name: "serviceaccounts", category: "Config" },
    // Network
    ResourceEntry { display_name: "Services", api_name: "services", category: "Network" },
    ResourceEntry { display_name: "Ingresses", api_name: "ingresses", category: "Network" },
    // Storage
    ResourceEntry { display_name: "PersistentVolumes", api_name: "persistentvolumes", category: "Storage" },
    ResourceEntry { display_name: "PersistentVolumeClaims", api_name: "persistentvolumeclaims", category: "Storage" },
    // Cluster
    ResourceEntry { display_name: "Namespaces", api_name: "namespaces", category: "Cluster" },
    ResourceEntry { display_name: "Nodes", api_name: "nodes", category: "Cluster" },
    ResourceEntry { display_name: "Events", api_name: "events", category: "Cluster" },
];

/// Find the index of a resource by api_name
pub fn resource_index(api_name: &str) -> Option<usize> {
    RESOURCES.iter().position(|r| r.api_name == api_name)
}
