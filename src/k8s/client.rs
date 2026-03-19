use anyhow::{Context, Result};
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    ConfigMap, Event, Namespace, Node, PersistentVolume, PersistentVolumeClaim, Pod, Secret,
    Service, ServiceAccount,
};
use k8s_openapi::api::networking::v1::Ingress;
use kube::api::{Api, ListParams, ResourceExt};
use kube::{Client, Config};

use crate::model::table::{TableColumn, TableData, TableRow};

/// Kubernetes client wrapper for k9rs
pub struct K8sClient;

impl K8sClient {
    /// Get the current kubeconfig context name
    pub fn current_context() -> Result<String> {
        let kubeconfig = kube::config::Kubeconfig::read().context("Failed to read kubeconfig")?;
        kubeconfig
            .current_context
            .ok_or_else(|| anyhow::anyhow!("No current context set"))
    }

    /// Create a kube::Client from the default kubeconfig
    async fn client() -> Result<Client> {
        let config = Config::infer().await.context("Failed to infer kube config")?;
        Client::try_from(config).context("Failed to create Kubernetes client")
    }

    /// List all namespace names in the cluster
    pub async fn list_namespace_names() -> Result<Vec<String>> {
        let client = Self::client().await?;
        let api: Api<Namespace> = Api::all(client);
        let nss = api.list(&ListParams::default()).await?;
        Ok(nss.items.into_iter().map(|ns| ns.name_any()).collect())
    }

    /// List resources of a given type in a namespace, returning TableData
    pub async fn list_resources(resource: &str, namespace: &str) -> Result<TableData> {
        let client = Self::client().await?;

        match resource {
            "pods" => Self::list_pods(client, namespace).await,
            "deployments" => Self::list_deployments(client, namespace).await,
            "services" => Self::list_services(client, namespace).await,
            "nodes" => Self::list_nodes(client).await,
            "namespaces" => Self::list_namespaces(client).await,
            "daemonsets" => Self::list_daemonsets(client, namespace).await,
            "statefulsets" => Self::list_statefulsets(client, namespace).await,
            "replicasets" => Self::list_replicasets(client, namespace).await,
            "configmaps" => Self::list_configmaps(client, namespace).await,
            "secrets" => Self::list_secrets(client, namespace).await,
            "serviceaccounts" => Self::list_serviceaccounts(client, namespace).await,
            "events" => Self::list_events(client, namespace).await,
            "jobs" => Self::list_jobs(client, namespace).await,
            "cronjobs" => Self::list_cronjobs(client, namespace).await,
            "persistentvolumes" => Self::list_pvs(client).await,
            "persistentvolumeclaims" => Self::list_pvcs(client, namespace).await,
            "ingresses" => Self::list_ingresses(client, namespace).await,
            other => Err(anyhow::anyhow!("Unknown resource type: {other}")),
        }
    }

    async fn list_pods(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<Pod> = Api::namespaced(client, namespace);
        let pods = api.list(&ListParams::default()).await?;

        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("READY", 8),
            TableColumn::new("STATUS", 12),
            TableColumn::new("RESTARTS", 10),
            TableColumn::new("AGE", 10),
        ];

        let rows: Vec<TableRow> = pods
            .items
            .into_iter()
            .map(|pod| {
                let name = pod.name_any();
                let status = pod
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.clone())
                    .unwrap_or_else(|| "Unknown".into());

                let (ready_count, total_count, restarts) = pod
                    .status
                    .as_ref()
                    .and_then(|s| s.container_statuses.as_ref())
                    .map(|cs| {
                        let ready = cs.iter().filter(|c| c.ready).count();
                        let total = cs.len();
                        let restarts: i32 = cs.iter().map(|c| c.restart_count).sum();
                        (ready, total, restarts)
                    })
                    .unwrap_or((0, 0, 0));

                let age = pod
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());

                TableRow {
                    cells: vec![
                        name,
                        format!("{ready_count}/{total_count}"),
                        status,
                        restarts.to_string(),
                        age,
                    ],
                }
            })
            .collect();

        Ok(TableData { columns, rows })
    }

    async fn list_deployments(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<Deployment> = Api::namespaced(client, namespace);
        let deps = api.list(&ListParams::default()).await?;

        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("READY", 10),
            TableColumn::new("UP-TO-DATE", 12),
            TableColumn::new("AVAILABLE", 10),
            TableColumn::new("AGE", 10),
        ];

        let rows = deps
            .items
            .into_iter()
            .map(|dep| {
                let name = dep.name_any();
                let status = dep.status.unwrap_or_default();
                let replicas = status.replicas.unwrap_or(0);
                let ready = status.ready_replicas.unwrap_or(0);
                let updated = status.updated_replicas.unwrap_or(0);
                let available = status.available_replicas.unwrap_or(0);
                let age = dep
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());

                TableRow {
                    cells: vec![
                        name,
                        format!("{ready}/{replicas}"),
                        updated.to_string(),
                        available.to_string(),
                        age,
                    ],
                }
            })
            .collect();

        Ok(TableData { columns, rows })
    }

    async fn list_services(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<Service> = Api::namespaced(client, namespace);
        let svcs = api.list(&ListParams::default()).await?;

        let columns = vec![
            TableColumn::new("NAME", 25),
            TableColumn::new("TYPE", 12),
            TableColumn::new("CLUSTER-IP", 16),
            TableColumn::new("PORTS", 20),
            TableColumn::new("AGE", 10),
        ];

        let rows = svcs
            .items
            .into_iter()
            .map(|svc| {
                let name = svc.name_any();
                let spec = svc.spec.unwrap_or_default();
                let svc_type = spec.type_.unwrap_or_else(|| "ClusterIP".into());
                let cluster_ip = spec.cluster_ip.unwrap_or_else(|| "None".into());
                let ports = spec
                    .ports
                    .unwrap_or_default()
                    .iter()
                    .map(|p| {
                        let proto = p.protocol.as_deref().unwrap_or("TCP");
                        let target = p.target_port.as_ref().map(|tp| format!("{tp:?}")).unwrap_or_default();
                        format!("{}:{}/{}", p.port, target, proto)
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let age = svc
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());

                TableRow {
                    cells: vec![name, svc_type, cluster_ip, ports, age],
                }
            })
            .collect();

        Ok(TableData { columns, rows })
    }

    async fn list_nodes(client: Client) -> Result<TableData> {
        let api: Api<Node> = Api::all(client);
        let nodes = api.list(&ListParams::default()).await?;

        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("STATUS", 10),
            TableColumn::new("ROLES", 15),
            TableColumn::new("VERSION", 15),
            TableColumn::new("AGE", 10),
        ];

        let rows = nodes
            .items
            .into_iter()
            .map(|node| {
                let name = node.name_any();
                let status = node
                    .status
                    .as_ref()
                    .and_then(|s| s.conditions.as_ref())
                    .and_then(|conds| {
                        conds
                            .iter()
                            .find(|c| c.type_ == "Ready")
                            .map(|c| {
                                if c.status == "True" {
                                    "Ready"
                                } else {
                                    "NotReady"
                                }
                            })
                    })
                    .unwrap_or("Unknown")
                    .to_string();
                let roles = node
                    .metadata
                    .labels
                    .as_ref()
                    .map(|labels| {
                        labels
                            .keys()
                            .filter(|k| k.starts_with("node-role.kubernetes.io/"))
                            .map(|k| k.trim_start_matches("node-role.kubernetes.io/"))
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_default();
                let roles = if roles.is_empty() {
                    "<none>".to_string()
                } else {
                    roles
                };
                let version = node
                    .status
                    .as_ref()
                    .and_then(|s| s.node_info.as_ref())
                    .map(|i| i.kubelet_version.clone())
                    .unwrap_or_default();
                let age = node
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());

                TableRow {
                    cells: vec![name, status, roles, version, age],
                }
            })
            .collect();

        Ok(TableData { columns, rows })
    }

    async fn list_namespaces(client: Client) -> Result<TableData> {
        let api: Api<Namespace> = Api::all(client);
        let nss = api.list(&ListParams::default()).await?;

        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("STATUS", 12),
            TableColumn::new("AGE", 10),
        ];

        let rows = nss
            .items
            .into_iter()
            .map(|ns| {
                let name = ns.name_any();
                let status = ns
                    .status
                    .and_then(|s| s.phase)
                    .unwrap_or_else(|| "Unknown".into());
                let age = ns
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, status, age],
                }
            })
            .collect();

        Ok(TableData { columns, rows })
    }

    // Stub implementations for other resource types — follow the same pattern
    async fn list_daemonsets(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<DaemonSet> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("DESIRED", 8),
            TableColumn::new("CURRENT", 8),
            TableColumn::new("READY", 8),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|ds| {
            let name = ds.name_any();
            let status = ds.status.unwrap_or_default();
            let age = ds.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![
                name,
                status.desired_number_scheduled.to_string(),
                status.current_number_scheduled.to_string(),
                status.number_ready.to_string(),
                age,
            ]}
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_statefulsets(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<StatefulSet> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("READY", 10),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|sts| {
            let name = sts.name_any();
            let status = sts.status.unwrap_or_default();
            let replicas = status.replicas;
            let ready = status.ready_replicas.unwrap_or(0);
            let age = sts.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![name, format!("{ready}/{replicas}"), age] }
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_replicasets(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<ReplicaSet> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 35),
            TableColumn::new("DESIRED", 8),
            TableColumn::new("CURRENT", 8),
            TableColumn::new("READY", 8),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|rs| {
            let name = rs.name_any();
            let status = rs.status.unwrap_or_default();
            let age = rs.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![
                name,
                status.replicas.to_string(),
                status.fully_labeled_replicas.unwrap_or(0).to_string(),
                status.ready_replicas.unwrap_or(0).to_string(),
                age,
            ]}
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_configmaps(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<ConfigMap> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 35),
            TableColumn::new("DATA", 6),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|cm| {
            let name = cm.name_any();
            let data_count = cm.data.map(|d| d.len()).unwrap_or(0);
            let age = cm.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![name, data_count.to_string(), age] }
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_secrets(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<Secret> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 35),
            TableColumn::new("TYPE", 25),
            TableColumn::new("DATA", 6),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|sec| {
            let name = sec.name_any();
            let type_ = sec.type_.unwrap_or_default();
            let data_count = sec.data.map(|d| d.len()).unwrap_or(0);
            let age = sec.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![name, type_, data_count.to_string(), age] }
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_serviceaccounts(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<ServiceAccount> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("SECRETS", 8),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|sa| {
            let name = sa.name_any();
            let secrets = sa.secrets.map(|s| s.len()).unwrap_or(0);
            let age = sa.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![name, secrets.to_string(), age] }
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_events(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<Event> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("TYPE", 8),
            TableColumn::new("REASON", 15),
            TableColumn::new("OBJECT", 25),
            TableColumn::new("MESSAGE", 40),
        ];
        let rows = items.items.into_iter().map(|ev| {
            let type_ = ev.type_.unwrap_or_default();
            let reason = ev.reason.unwrap_or_default();
            let object = ev.involved_object.name.unwrap_or_default();
            let message = ev.message.unwrap_or_default();
            TableRow { cells: vec![type_, reason, object, message] }
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_jobs(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<Job> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("COMPLETIONS", 12),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|job| {
            let name = job.name_any();
            let status = job.status.unwrap_or_default();
            let succeeded = status.succeeded.unwrap_or(0);
            let completions = job.spec.and_then(|s| s.completions).unwrap_or(1);
            let age = job.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![name, format!("{succeeded}/{completions}"), age] }
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_cronjobs(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<CronJob> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("SCHEDULE", 15),
            TableColumn::new("SUSPEND", 8),
            TableColumn::new("ACTIVE", 8),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|cj| {
            let name = cj.name_any();
            let schedule = cj.spec.as_ref().map(|s| s.schedule.clone()).unwrap_or_default();
            let suspend = cj.spec.as_ref().and_then(|s| s.suspend).unwrap_or(false);
            let active = cj.status.and_then(|s| s.active).map(|a| a.len()).unwrap_or(0);
            let age = cj.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![
                name, schedule, suspend.to_string(), active.to_string(), age,
            ]}
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_pvs(client: Client) -> Result<TableData> {
        let api: Api<PersistentVolume> = Api::all(client);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("CAPACITY", 10),
            TableColumn::new("STATUS", 10),
            TableColumn::new("CLAIM", 25),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|pv| {
            let name = pv.name_any();
            let capacity = pv.spec.as_ref()
                .and_then(|s| s.capacity.as_ref())
                .and_then(|c| c.get("storage"))
                .map(|s| s.0.clone())
                .unwrap_or_default();
            let status = pv.status.and_then(|s| s.phase).unwrap_or_default();
            let claim = pv.spec.as_ref()
                .and_then(|s| s.claim_ref.as_ref())
                .map(|c| format!("{}/{}", c.namespace.as_deref().unwrap_or(""), c.name.as_deref().unwrap_or("")))
                .unwrap_or_default();
            let age = pv.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![name, capacity, status, claim, age] }
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_pvcs(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<PersistentVolumeClaim> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 30),
            TableColumn::new("STATUS", 10),
            TableColumn::new("VOLUME", 25),
            TableColumn::new("CAPACITY", 10),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|pvc| {
            let name = pvc.name_any();
            let status = pvc.status.as_ref().and_then(|s| s.phase.clone()).unwrap_or_default();
            let volume = pvc.spec.as_ref().and_then(|s| s.volume_name.clone()).unwrap_or_default();
            let capacity = pvc.status.as_ref()
                .and_then(|s| s.capacity.as_ref())
                .and_then(|c| c.get("storage"))
                .map(|s| s.0.clone())
                .unwrap_or_default();
            let age = pvc.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![name, status, volume, capacity, age] }
        }).collect();
        Ok(TableData { columns, rows })
    }

    async fn list_ingresses(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<Ingress> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 25),
            TableColumn::new("CLASS", 15),
            TableColumn::new("HOSTS", 30),
            TableColumn::new("AGE", 10),
        ];
        let rows = items.items.into_iter().map(|ing| {
            let name = ing.name_any();
            let class = ing.spec.as_ref()
                .and_then(|s| s.ingress_class_name.clone())
                .unwrap_or_else(|| "<none>".into());
            let hosts = ing.spec.as_ref()
                .and_then(|s| s.rules.as_ref())
                .map(|rules| rules.iter()
                    .filter_map(|r| r.host.as_ref())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(","))
                .unwrap_or_else(|| "*".into());
            let age = ing.metadata.creation_timestamp.as_ref()
                .map(|ts| format_age(&ts.0)).unwrap_or_else(|| "Unknown".into());
            TableRow { cells: vec![name, class, hosts, age] }
        }).collect();
        Ok(TableData { columns, rows })
    }
}

/// Format a timestamp into a human-readable age string (e.g. "5d", "3h", "12m")
fn format_age(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(timestamp);

    let total_seconds = duration.num_seconds();
    if total_seconds < 0 {
        return "future".into();
    }

    let days = duration.num_days();
    let hours = duration.num_hours();
    let minutes = duration.num_minutes();

    if days > 0 {
        format!("{days}d")
    } else if hours > 0 {
        format!("{hours}h")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{total_seconds}s")
    }
}
