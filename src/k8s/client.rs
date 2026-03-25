use std::collections::BTreeMap;

use anyhow::{Context, Result};
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    ConfigMap, Event, Namespace, Node, PersistentVolume, PersistentVolumeClaim, Pod, Secret,
    Service, ServiceAccount,
};
use k8s_openapi::api::networking::v1::Ingress;
use kube::api::{Api, DeleteParams, ListParams, LogParams, Patch, PatchParams, ResourceExt};
use kube::{Client, Config};

use crate::model::detail::{
    Condition, ContainerInfo, EventEntry, OwnerRef, PodInfo, ResourceDetail,
};
use crate::model::port_forward::PodPort;
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

    /// List available contexts from kubeconfig
    pub fn list_contexts() -> Result<Vec<String>> {
        let kubeconfig = kube::config::Kubeconfig::read().context("Failed to read kubeconfig")?;
        Ok(kubeconfig.contexts.iter().map(|c| c.name.clone()).collect())
    }

    /// Create a kube::Client from the default kubeconfig
    async fn client() -> Result<Client> {
        let config = Config::infer()
            .await
            .context("Failed to infer kube config")?;
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
            TableColumn::new("UP-TO-DATE", 20),
            TableColumn::new("AVAILABLE", 15),
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
                        let target = p
                            .target_port
                            .as_ref()
                            .map(|tp| format!("{tp:?}"))
                            .unwrap_or_default();
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
                        conds.iter().find(|c| c.type_ == "Ready").map(|c| {
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
        let rows = items
            .items
            .into_iter()
            .map(|ds| {
                let name = ds.name_any();
                let status = ds.status.unwrap_or_default();
                let age = ds
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![
                        name,
                        status.desired_number_scheduled.to_string(),
                        status.current_number_scheduled.to_string(),
                        status.number_ready.to_string(),
                        age,
                    ],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|sts| {
                let name = sts.name_any();
                let status = sts.status.unwrap_or_default();
                let replicas = status.replicas;
                let ready = status.ready_replicas.unwrap_or(0);
                let age = sts
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, format!("{ready}/{replicas}"), age],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|rs| {
                let name = rs.name_any();
                let status = rs.status.unwrap_or_default();
                let age = rs
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![
                        name,
                        status.replicas.to_string(),
                        status.fully_labeled_replicas.unwrap_or(0).to_string(),
                        status.ready_replicas.unwrap_or(0).to_string(),
                        age,
                    ],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|cm| {
                let name = cm.name_any();
                let data_count = cm.data.map(|d| d.len()).unwrap_or(0);
                let age = cm
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, data_count.to_string(), age],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|sec| {
                let name = sec.name_any();
                let type_ = sec.type_.unwrap_or_default();
                let data_count = sec.data.map(|d| d.len()).unwrap_or(0);
                let age = sec
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, type_, data_count.to_string(), age],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|sa| {
                let name = sa.name_any();
                let secrets = sa.secrets.map(|s| s.len()).unwrap_or(0);
                let age = sa
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, secrets.to_string(), age],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|ev| {
                let type_ = ev.type_.unwrap_or_default();
                let reason = ev.reason.unwrap_or_default();
                let object = ev.involved_object.name.unwrap_or_default();
                let message = ev.message.unwrap_or_default();
                TableRow {
                    cells: vec![type_, reason, object, message],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|job| {
                let name = job.name_any();
                let status = job.status.unwrap_or_default();
                let succeeded = status.succeeded.unwrap_or(0);
                let completions = job.spec.and_then(|s| s.completions).unwrap_or(1);
                let age = job
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, format!("{succeeded}/{completions}"), age],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|cj| {
                let name = cj.name_any();
                let schedule = cj
                    .spec
                    .as_ref()
                    .map(|s| s.schedule.clone())
                    .unwrap_or_default();
                let suspend = cj.spec.as_ref().and_then(|s| s.suspend).unwrap_or(false);
                let active = cj
                    .status
                    .and_then(|s| s.active)
                    .map(|a| a.len())
                    .unwrap_or(0);
                let age = cj
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, schedule, suspend.to_string(), active.to_string(), age],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|pv| {
                let name = pv.name_any();
                let capacity = pv
                    .spec
                    .as_ref()
                    .and_then(|s| s.capacity.as_ref())
                    .and_then(|c| c.get("storage"))
                    .map(|s| s.0.clone())
                    .unwrap_or_default();
                let status = pv.status.and_then(|s| s.phase).unwrap_or_default();
                let claim = pv
                    .spec
                    .as_ref()
                    .and_then(|s| s.claim_ref.as_ref())
                    .map(|c| {
                        format!(
                            "{}/{}",
                            c.namespace.as_deref().unwrap_or(""),
                            c.name.as_deref().unwrap_or("")
                        )
                    })
                    .unwrap_or_default();
                let age = pv
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, capacity, status, claim, age],
                }
            })
            .collect();
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
        let rows = items
            .items
            .into_iter()
            .map(|pvc| {
                let name = pvc.name_any();
                let status = pvc
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.clone())
                    .unwrap_or_default();
                let volume = pvc
                    .spec
                    .as_ref()
                    .and_then(|s| s.volume_name.clone())
                    .unwrap_or_default();
                let capacity = pvc
                    .status
                    .as_ref()
                    .and_then(|s| s.capacity.as_ref())
                    .and_then(|c| c.get("storage"))
                    .map(|s| s.0.clone())
                    .unwrap_or_default();
                let age = pvc
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, status, volume, capacity, age],
                }
            })
            .collect();
        Ok(TableData { columns, rows })
    }

    // ── Resource Detail Methods ──────────────────────────────────────

    /// Get detailed info for a single resource
    pub async fn get_resource_detail(
        resource_type: &str,
        name: &str,
        namespace: &str,
    ) -> Result<ResourceDetail> {
        let client = Self::client().await?;

        match resource_type {
            "pods" => Self::get_pod_detail(client, name, namespace).await,
            "deployments" => Self::get_deployment_detail(client, name, namespace).await,
            "services" => Self::get_service_detail(client, name, namespace).await,
            "statefulsets" => Self::get_statefulset_detail(client, name, namespace).await,
            "daemonsets" => Self::get_daemonset_detail(client, name, namespace).await,
            "replicasets" => Self::get_replicaset_detail(client, name, namespace).await,
            "jobs" => Self::get_job_detail(client, name, namespace).await,
            "cronjobs" => Self::get_cronjob_detail(client, name, namespace).await,
            "configmaps" => {
                Self::get_typed_detail::<ConfigMap>(client, name, namespace, "configmaps").await
            }
            "secrets" => Self::get_typed_detail::<Secret>(client, name, namespace, "secrets").await,
            "serviceaccounts" => {
                Self::get_typed_detail::<ServiceAccount>(client, name, namespace, "serviceaccounts")
                    .await
            }
            "ingresses" => {
                Self::get_typed_detail::<Ingress>(client, name, namespace, "ingresses").await
            }
            "persistentvolumeclaims" => {
                Self::get_typed_detail::<PersistentVolumeClaim>(
                    client,
                    name,
                    namespace,
                    "persistentvolumeclaims",
                )
                .await
            }
            "persistentvolumes" => Self::get_pv_detail(client, name).await,
            "namespaces" => Self::get_ns_detail(client, name).await,
            "nodes" => Self::get_node_detail(client, name).await,
            "events" => Self::get_typed_detail::<Event>(client, name, namespace, "events").await,
            other => Self::get_generic_detail(client, other, name, namespace).await,
        }
    }

    /// Get pod logs
    pub async fn get_pod_logs(
        name: &str,
        namespace: &str,
        container: Option<&str>,
        tail_lines: Option<i64>,
    ) -> Result<String> {
        let client = Self::client().await?;
        let api: Api<Pod> = Api::namespaced(client, namespace);
        let mut params = LogParams {
            tail_lines,
            ..Default::default()
        };
        if let Some(c) = container {
            params.container = Some(c.to_string());
        }
        let logs = api.logs(name, &params).await?;
        Ok(logs)
    }

    /// Get logs for any resource type. For pods, fetches directly.
    /// For workloads, finds pods via label selector and aggregates logs.
    pub async fn get_resource_logs(
        resource_type: &str,
        name: &str,
        namespace: &str,
        tail_lines: Option<i64>,
    ) -> Result<String> {
        let client = Self::client().await?;

        match resource_type {
            "pods" => Self::get_pod_logs(name, namespace, None, tail_lines).await,
            "deployments" | "statefulsets" | "daemonsets" | "replicasets" => {
                // Get the workload's selector labels
                let label_selector = Self::get_workload_label_selector(
                    client.clone(),
                    resource_type,
                    name,
                    namespace,
                )
                .await?;
                Self::get_logs_by_selector(client, namespace, &label_selector, tail_lines).await
            }
            "jobs" => {
                let selector = format!("job-name={name}");
                Self::get_logs_by_selector(client, namespace, &selector, tail_lines).await
            }
            other => Err(anyhow::anyhow!("Logs not supported for {other}")),
        }
    }

    /// Get the label selector string from a workload's spec.selector.matchLabels
    async fn get_workload_label_selector(
        client: Client,
        resource_type: &str,
        name: &str,
        namespace: &str,
    ) -> Result<String> {
        let labels: Option<std::collections::BTreeMap<String, String>> = match resource_type {
            "deployments" => {
                let api: Api<Deployment> = Api::namespaced(client, namespace);
                let dep = api.get(name).await?;
                dep.spec.and_then(|s| s.selector.match_labels)
            }
            "statefulsets" => {
                let api: Api<StatefulSet> = Api::namespaced(client, namespace);
                let sts = api.get(name).await?;
                sts.spec.and_then(|s| s.selector.match_labels)
            }
            "daemonsets" => {
                let api: Api<DaemonSet> = Api::namespaced(client, namespace);
                let ds = api.get(name).await?;
                ds.spec.and_then(|s| s.selector.match_labels)
            }
            "replicasets" => {
                let api: Api<ReplicaSet> = Api::namespaced(client, namespace);
                let rs = api.get(name).await?;
                rs.spec.and_then(|s| s.selector.match_labels)
            }
            _ => None,
        };

        labels
            .map(|l| {
                l.iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .ok_or_else(|| anyhow::anyhow!("No selector found for {resource_type}/{name}"))
    }

    /// Fetch logs from all pods matching a label selector, prefixed with pod name
    async fn get_logs_by_selector(
        client: Client,
        namespace: &str,
        label_selector: &str,
        tail_lines: Option<i64>,
    ) -> Result<String> {
        let api: Api<Pod> = Api::namespaced(client, namespace);
        let lp = ListParams::default().labels(label_selector);
        let pods = api.list(&lp).await?;

        let per_pod_lines = tail_lines
            .map(|t| t / std::cmp::max(pods.items.len() as i64, 1))
            .map(|t| std::cmp::max(t, 20)); // at least 20 lines per pod

        let mut all_logs = Vec::new();
        for pod in &pods.items {
            let pod_name = pod.name_any();
            let mut params = LogParams {
                tail_lines: per_pod_lines,
                ..Default::default()
            };
            // Get all containers
            params.container = None;

            match api.logs(&pod_name, &params).await {
                Ok(log) => {
                    for line in log.lines() {
                        all_logs.push(format!("[{pod_name}] {line}"));
                    }
                }
                Err(e) => {
                    all_logs.push(format!("[{pod_name}] Error fetching logs: {e}"));
                }
            }
        }

        if all_logs.is_empty() {
            Ok("No logs found for matching pods".to_string())
        } else {
            Ok(all_logs.join("\n"))
        }
    }

    /// Restart a resource. For workloads (deployments, statefulsets, daemonsets),
    /// performs a rollout restart by patching the pod template annotation.
    /// For pods, deletes the pod (letting the controller recreate it).
    pub async fn restart_resource(
        resource_type: &str,
        name: &str,
        namespace: &str,
    ) -> Result<String> {
        let client = Self::client().await?;
        let now = chrono::Utc::now().to_rfc3339();

        match resource_type {
            "pods" => {
                let api: Api<Pod> = Api::namespaced(client, namespace);
                api.delete(name, &DeleteParams::default()).await?;
                Ok(format!("Pod {name} deleted"))
            }
            "deployments" => {
                let api: Api<Deployment> = Api::namespaced(client, namespace);
                let patch = serde_json::json!({
                    "spec": {
                        "template": {
                            "metadata": {
                                "annotations": {
                                    "kubectl.kubernetes.io/restartedAt": now
                                }
                            }
                        }
                    }
                });
                api.patch(name, &PatchParams::default(), &Patch::Strategic(patch))
                    .await?;
                Ok(format!("Deployment {name} restarted"))
            }
            "statefulsets" => {
                let api: Api<StatefulSet> = Api::namespaced(client, namespace);
                let patch = serde_json::json!({
                    "spec": {
                        "template": {
                            "metadata": {
                                "annotations": {
                                    "kubectl.kubernetes.io/restartedAt": now
                                }
                            }
                        }
                    }
                });
                api.patch(name, &PatchParams::default(), &Patch::Strategic(patch))
                    .await?;
                Ok(format!("StatefulSet {name} restarted"))
            }
            "daemonsets" => {
                let api: Api<DaemonSet> = Api::namespaced(client, namespace);
                let patch = serde_json::json!({
                    "spec": {
                        "template": {
                            "metadata": {
                                "annotations": {
                                    "kubectl.kubernetes.io/restartedAt": now
                                }
                            }
                        }
                    }
                });
                api.patch(name, &PatchParams::default(), &Patch::Strategic(patch))
                    .await?;
                Ok(format!("DaemonSet {name} restarted"))
            }
            other => Err(anyhow::anyhow!("Restart not supported for {other}")),
        }
    }

    /// Apply YAML to the cluster using kubectl
    pub async fn apply_yaml(yaml: &str, namespace: &str) -> Result<String> {
        use tokio::process::Command;

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("k9rs-apply.yaml");
        tokio::fs::write(&temp_file, yaml).await?;

        let output = Command::new("kubectl")
            .args(["apply", "-f", temp_file.to_str().unwrap(), "-n", namespace])
            .output()
            .await
            .context("Failed to run kubectl apply")?;

        // Clean up
        let _ = tokio::fs::remove_file(&temp_file).await;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(if stdout.is_empty() {
                "Applied successfully".to_string()
            } else {
                stdout
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(anyhow::anyhow!("{stderr}"))
        }
    }

    /// Get structured port information from a pod spec
    pub async fn get_pod_ports(name: &str, namespace: &str) -> Result<Vec<PodPort>> {
        let client = Self::client().await?;
        let api: Api<Pod> = Api::namespaced(client, namespace);
        let pod = api.get(name).await?;

        let mut ports = vec![];
        if let Some(spec) = &pod.spec {
            for container in &spec.containers {
                if let Some(container_ports) = &container.ports {
                    for p in container_ports {
                        ports.push(PodPort {
                            container_name: container.name.clone(),
                            port: p.container_port as u16,
                            protocol: p.protocol.clone().unwrap_or_else(|| "TCP".into()),
                            name: p.name.clone(),
                        });
                    }
                }
            }
        }
        Ok(ports)
    }

    /// Start a port forward using kubectl (spawns a background process).
    /// Returns the child process handle for management.
    pub async fn start_port_forward(
        pod_name: &str,
        namespace: &str,
        local_port: u16,
        remote_port: u16,
    ) -> Result<tokio::process::Child> {
        use tokio::process::Command;

        let child = Command::new("kubectl")
            .args([
                "port-forward",
                &format!("pod/{pod_name}"),
                &format!("{local_port}:{remote_port}"),
                "-n",
                namespace,
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to start kubectl port-forward")?;

        Ok(child)
    }

    /// Fetch events related to a specific resource
    async fn get_resource_events(client: Client, name: &str, namespace: &str) -> Vec<EventEntry> {
        let api: Api<Event> = Api::namespaced(client, namespace);
        let field_selector = format!("involvedObject.name={name}");
        let lp = ListParams::default().fields(&field_selector);
        let events = match api.list(&lp).await {
            Ok(list) => list,
            Err(_) => return vec![],
        };
        events
            .items
            .into_iter()
            .map(|ev| EventEntry {
                type_: ev.type_.unwrap_or_default(),
                reason: ev.reason.unwrap_or_default(),
                age: ev
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_default(),
                from: ev.source.and_then(|s| s.component).unwrap_or_default(),
                message: ev.message.unwrap_or_default(),
            })
            .collect()
    }

    /// Fetch pods matching a label selector (for workload detail views)
    async fn get_workload_pods(
        client: Client,
        namespace: &str,
        label_selector: &str,
    ) -> Vec<PodInfo> {
        let api: Api<Pod> = Api::namespaced(client, namespace);
        let lp = ListParams::default().labels(label_selector);
        let pods = match api.list(&lp).await {
            Ok(list) => list,
            Err(_) => return vec![],
        };

        pods.items
            .into_iter()
            .map(|pod| {
                let name = pod.name_any();
                let status = pod.status.as_ref();

                let phase = status
                    .and_then(|s| s.phase.clone())
                    .unwrap_or_else(|| "Unknown".into());

                let container_statuses = status.and_then(|s| s.container_statuses.as_ref());

                let (ready_count, total_count) = container_statuses
                    .map(|cs| {
                        let ready = cs.iter().filter(|c| c.ready).count();
                        (ready, cs.len())
                    })
                    .unwrap_or((0, 0));

                let restarts: i32 = container_statuses
                    .map(|cs| cs.iter().map(|c| c.restart_count).sum())
                    .unwrap_or(0);

                // Find last restart info from container statuses
                let (last_restart_time, last_restart_reason) = container_statuses
                    .and_then(|cs| {
                        cs.iter()
                            .filter_map(|c| {
                                c.last_state.as_ref().and_then(|ls| {
                                    ls.terminated.as_ref().map(|t| {
                                        let time = t
                                            .finished_at
                                            .as_ref()
                                            .map(|ts| format_age(&ts.0))
                                            .unwrap_or_default();
                                        let reason = t.reason.clone().unwrap_or_else(|| {
                                            format!("exit code {}", t.exit_code)
                                        });
                                        (time, reason)
                                    })
                                })
                            })
                            .next()
                    })
                    .unwrap_or_default();

                let node = pod
                    .spec
                    .as_ref()
                    .and_then(|s| s.node_name.clone())
                    .unwrap_or_default();

                let ip = status.and_then(|s| s.pod_ip.clone()).unwrap_or_default();

                let age = pod
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());

                PodInfo {
                    name,
                    ready: format!("{ready_count}/{total_count}"),
                    status: phase,
                    cpu: "-".to_string(),
                    memory: "-".to_string(),
                    restarts,
                    last_restart_time,
                    last_restart_reason,
                    node,
                    ip,
                    age,
                }
            })
            .collect()
    }

    /// Extract common metadata fields into a ResourceDetail
    fn extract_metadata(
        meta: &k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta,
        resource_type: &str,
    ) -> ResourceDetail {
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone();
        let age = meta
            .creation_timestamp
            .as_ref()
            .map(|ts| format_age(&ts.0))
            .unwrap_or_else(|| "Unknown".into());
        let labels = meta.labels.clone().unwrap_or_default();
        let annotations = meta.annotations.clone().unwrap_or_default();
        let owner_references = meta
            .owner_references
            .as_ref()
            .map(|refs| {
                refs.iter()
                    .map(|r| OwnerRef {
                        kind: r.kind.clone(),
                        name: r.name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        ResourceDetail {
            name,
            namespace,
            resource_type: resource_type.to_string(),
            age,
            phase: String::new(),
            labels,
            annotations,
            owner_references,
            conditions: vec![],
            containers: vec![],
            pods: vec![],
            yaml: String::new(),
            events: vec![],
        }
    }

    async fn get_pod_detail(client: Client, name: &str, namespace: &str) -> Result<ResourceDetail> {
        let api: Api<Pod> = Api::namespaced(client.clone(), namespace);
        let pod = api.get(name).await?;

        let mut detail = Self::extract_metadata(&pod.metadata, "pods");
        detail.yaml = serde_yml::to_string(&pod).unwrap_or_default();

        if let Some(status) = &pod.status {
            detail.phase = status.phase.clone().unwrap_or_else(|| "Unknown".into());

            if let Some(conditions) = &status.conditions {
                detail.conditions = conditions
                    .iter()
                    .map(|c| Condition {
                        type_: c.type_.clone(),
                        status: c.status.clone(),
                        reason: c.reason.clone().unwrap_or_default(),
                        message: c.message.clone().unwrap_or_default(),
                        last_transition: c
                            .last_transition_time
                            .as_ref()
                            .map(|ts| format_age(&ts.0))
                            .unwrap_or_default(),
                    })
                    .collect();
            }

            if let Some(cs) = &status.container_statuses {
                detail.containers = cs
                    .iter()
                    .map(|c| {
                        let state = if let Some(s) = &c.state {
                            if s.running.is_some() {
                                "Running".to_string()
                            } else if let Some(w) = &s.waiting {
                                format!("Waiting: {}", w.reason.as_deref().unwrap_or("Unknown"))
                            } else if let Some(t) = &s.terminated {
                                format!("Terminated: {}", t.reason.as_deref().unwrap_or("Unknown"))
                            } else {
                                "Unknown".to_string()
                            }
                        } else {
                            "Unknown".to_string()
                        };
                        ContainerInfo {
                            name: c.name.clone(),
                            image: c.image.clone(),
                            ready: c.ready,
                            restart_count: c.restart_count,
                            state,
                            ports: String::new(),
                        }
                    })
                    .collect();
            }
        }

        // Enrich containers with port info from spec
        if let Some(spec) = &pod.spec {
            for spec_container in &spec.containers {
                if let Some(detail_container) = detail
                    .containers
                    .iter_mut()
                    .find(|c| c.name == spec_container.name)
                {
                    detail_container.ports = spec_container
                        .ports
                        .as_ref()
                        .map(|ports| {
                            ports
                                .iter()
                                .map(|p| {
                                    format!(
                                        "{}/{}",
                                        p.container_port,
                                        p.protocol.as_deref().unwrap_or("TCP")
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                }
            }
        }

        detail.events = Self::get_resource_events(client, name, namespace).await;
        Ok(detail)
    }

    async fn get_deployment_detail(
        client: Client,
        name: &str,
        namespace: &str,
    ) -> Result<ResourceDetail> {
        let api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
        let dep = api.get(name).await?;

        let mut detail = Self::extract_metadata(&dep.metadata, "deployments");
        detail.yaml = serde_yml::to_string(&dep).unwrap_or_default();

        if let Some(status) = &dep.status {
            let ready = status.ready_replicas.unwrap_or(0);
            let desired = status.replicas.unwrap_or(0);
            detail.phase = format!("{ready}/{desired} ready");

            if let Some(conditions) = &status.conditions {
                detail.conditions = conditions
                    .iter()
                    .map(|c| Condition {
                        type_: c.type_.clone(),
                        status: c.status.clone(),
                        reason: c.reason.clone().unwrap_or_default(),
                        message: c.message.clone().unwrap_or_default(),
                        last_transition: c
                            .last_transition_time
                            .as_ref()
                            .map(|ts| format_age(&ts.0))
                            .unwrap_or_default(),
                    })
                    .collect();
            }
        }

        // Show containers from the pod template spec
        if let Some(spec) = &dep.spec {
            detail.containers = spec
                .template
                .spec
                .as_ref()
                .map(|ps| {
                    ps.containers
                        .iter()
                        .map(|c| ContainerInfo {
                            name: c.name.clone(),
                            image: c.image.clone().unwrap_or_default(),
                            ready: true,
                            restart_count: 0,
                            state: "Template".to_string(),
                            ports: c
                                .ports
                                .as_ref()
                                .map(|ports| {
                                    ports
                                        .iter()
                                        .map(|p| {
                                            format!(
                                                "{}/{}",
                                                p.container_port,
                                                p.protocol.as_deref().unwrap_or("TCP")
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                })
                                .unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();
        }

        // Fetch pods for this deployment
        if let Some(spec) = &dep.spec {
            if let Some(selector) = &spec.selector.match_labels {
                let label_selector = selector
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(",");
                detail.pods =
                    Self::get_workload_pods(client.clone(), namespace, &label_selector).await;
            }
        }

        detail.events = Self::get_resource_events(client, name, namespace).await;
        Ok(detail)
    }

    async fn get_service_detail(
        client: Client,
        name: &str,
        namespace: &str,
    ) -> Result<ResourceDetail> {
        let api: Api<Service> = Api::namespaced(client.clone(), namespace);
        let svc = api.get(name).await?;

        let mut detail = Self::extract_metadata(&svc.metadata, "services");
        detail.yaml = serde_yml::to_string(&svc).unwrap_or_default();

        if let Some(spec) = &svc.spec {
            detail.phase = spec.type_.clone().unwrap_or_else(|| "ClusterIP".into());
        }

        detail.events = Self::get_resource_events(client, name, namespace).await;
        Ok(detail)
    }

    async fn get_statefulset_detail(
        client: Client,
        name: &str,
        namespace: &str,
    ) -> Result<ResourceDetail> {
        let api: Api<StatefulSet> = Api::namespaced(client.clone(), namespace);
        let sts = api.get(name).await?;

        let mut detail = Self::extract_metadata(&sts.metadata, "statefulsets");
        detail.yaml = serde_yml::to_string(&sts).unwrap_or_default();

        if let Some(status) = &sts.status {
            let ready = status.ready_replicas.unwrap_or(0);
            let replicas = status.replicas;
            detail.phase = format!("{ready}/{replicas} ready");
        }

        if let Some(spec) = &sts.spec {
            detail.containers = spec
                .template
                .spec
                .as_ref()
                .map(|ps| {
                    ps.containers
                        .iter()
                        .map(|c| ContainerInfo {
                            name: c.name.clone(),
                            image: c.image.clone().unwrap_or_default(),
                            ready: true,
                            restart_count: 0,
                            state: "Template".to_string(),
                            ports: String::new(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            if let Some(selector) = &spec.selector.match_labels {
                let label_selector = selector
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(",");
                detail.pods =
                    Self::get_workload_pods(client.clone(), namespace, &label_selector).await;
            }
        }

        detail.events = Self::get_resource_events(client, name, namespace).await;
        Ok(detail)
    }

    async fn get_daemonset_detail(
        client: Client,
        name: &str,
        namespace: &str,
    ) -> Result<ResourceDetail> {
        let api: Api<DaemonSet> = Api::namespaced(client.clone(), namespace);
        let ds = api.get(name).await?;

        let mut detail = Self::extract_metadata(&ds.metadata, "daemonsets");
        detail.yaml = serde_yml::to_string(&ds).unwrap_or_default();

        if let Some(status) = &ds.status {
            detail.phase = format!(
                "{}/{} ready",
                status.number_ready, status.desired_number_scheduled
            );
        }

        // Fetch pods for daemonset
        if let Some(spec) = &ds.spec {
            if let Some(selector) = &spec.selector.match_labels {
                let label_selector = selector
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(",");
                detail.pods =
                    Self::get_workload_pods(client.clone(), namespace, &label_selector).await;
            }
        }

        detail.events = Self::get_resource_events(client, name, namespace).await;
        Ok(detail)
    }

    async fn get_replicaset_detail(
        client: Client,
        name: &str,
        namespace: &str,
    ) -> Result<ResourceDetail> {
        let api: Api<ReplicaSet> = Api::namespaced(client.clone(), namespace);
        let rs = api.get(name).await?;

        let mut detail = Self::extract_metadata(&rs.metadata, "replicasets");
        detail.yaml = serde_yml::to_string(&rs).unwrap_or_default();

        if let Some(status) = &rs.status {
            let ready = status.ready_replicas.unwrap_or(0);
            detail.phase = format!("{ready}/{} ready", status.replicas);
        }

        if let Some(spec) = &rs.spec {
            if let Some(selector) = &spec.selector.match_labels {
                let label_selector = selector
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(",");
                detail.pods =
                    Self::get_workload_pods(client.clone(), namespace, &label_selector).await;
            }
        }

        detail.events = Self::get_resource_events(client, name, namespace).await;
        Ok(detail)
    }

    async fn get_job_detail(client: Client, name: &str, namespace: &str) -> Result<ResourceDetail> {
        let api: Api<Job> = Api::namespaced(client.clone(), namespace);
        let job = api.get(name).await?;

        let mut detail = Self::extract_metadata(&job.metadata, "jobs");
        detail.yaml = serde_yml::to_string(&job).unwrap_or_default();

        if let Some(status) = &job.status {
            let succeeded = status.succeeded.unwrap_or(0);
            let completions = job.spec.as_ref().and_then(|s| s.completions).unwrap_or(1);
            detail.phase = format!("{succeeded}/{completions} completed");

            if let Some(conditions) = &status.conditions {
                detail.conditions = conditions
                    .iter()
                    .map(|c| Condition {
                        type_: c.type_.clone(),
                        status: c.status.clone(),
                        reason: c.reason.clone().unwrap_or_default(),
                        message: c.message.clone().unwrap_or_default(),
                        last_transition: c
                            .last_transition_time
                            .as_ref()
                            .map(|ts| format_age(&ts.0))
                            .unwrap_or_default(),
                    })
                    .collect();
            }
        }

        // Fetch pods for this job
        let label_selector = format!("job-name={name}");
        detail.pods = Self::get_workload_pods(client.clone(), namespace, &label_selector).await;

        detail.events = Self::get_resource_events(client, name, namespace).await;
        Ok(detail)
    }

    async fn get_cronjob_detail(
        client: Client,
        name: &str,
        namespace: &str,
    ) -> Result<ResourceDetail> {
        let api: Api<CronJob> = Api::namespaced(client.clone(), namespace);
        let cj = api.get(name).await?;

        let mut detail = Self::extract_metadata(&cj.metadata, "cronjobs");
        detail.yaml = serde_yml::to_string(&cj).unwrap_or_default();

        if let Some(spec) = &cj.spec {
            let suspended = spec.suspend.unwrap_or(false);
            detail.phase = if suspended {
                format!("Suspended ({})", spec.schedule)
            } else {
                format!("Active ({})", spec.schedule)
            };
        }

        detail.events = Self::get_resource_events(client, name, namespace).await;
        Ok(detail)
    }

    async fn get_node_detail(client: Client, name: &str) -> Result<ResourceDetail> {
        let api: Api<Node> = Api::all(client.clone());
        let node = api.get(name).await?;

        let mut detail = Self::extract_metadata(&node.metadata, "nodes");
        detail.yaml = serde_yml::to_string(&node).unwrap_or_default();

        if let Some(status) = &node.status {
            detail.phase = status
                .conditions
                .as_ref()
                .and_then(|conds| {
                    conds.iter().find(|c| c.type_ == "Ready").map(|c| {
                        if c.status == "True" {
                            "Ready"
                        } else {
                            "NotReady"
                        }
                    })
                })
                .unwrap_or("Unknown")
                .to_string();

            if let Some(conditions) = &status.conditions {
                detail.conditions = conditions
                    .iter()
                    .map(|c| Condition {
                        type_: c.type_.clone(),
                        status: c.status.clone(),
                        reason: c.reason.clone().unwrap_or_default(),
                        message: c.message.clone().unwrap_or_default(),
                        last_transition: c
                            .last_transition_time
                            .as_ref()
                            .map(|ts| format_age(&ts.0))
                            .unwrap_or_default(),
                    })
                    .collect();
            }
        }

        // Node events are cluster-scoped
        let event_api: Api<Event> = Api::namespaced(client, "default");
        let field_selector = format!("involvedObject.name={name}");
        let lp = ListParams::default().fields(&field_selector);
        if let Ok(events) = event_api.list(&lp).await {
            detail.events = events
                .items
                .into_iter()
                .map(|ev| EventEntry {
                    type_: ev.type_.unwrap_or_default(),
                    reason: ev.reason.unwrap_or_default(),
                    age: ev
                        .metadata
                        .creation_timestamp
                        .as_ref()
                        .map(|ts| format_age(&ts.0))
                        .unwrap_or_default(),
                    from: ev.source.and_then(|s| s.component).unwrap_or_default(),
                    message: ev.message.unwrap_or_default(),
                })
                .collect();
        }

        Ok(detail)
    }

    /// Detail for any typed namespaced resource (ConfigMap, Secret, etc.)
    async fn get_typed_detail<K>(
        client: Client,
        name: &str,
        namespace: &str,
        resource_type: &str,
    ) -> Result<ResourceDetail>
    where
        K: kube::Resource<Scope = k8s_openapi::NamespaceResourceScope>
            + Clone
            + std::fmt::Debug
            + serde::de::DeserializeOwned
            + serde::Serialize
            + 'static,
        <K as kube::Resource>::DynamicType: Default,
    {
        let api: Api<K> = Api::namespaced(client.clone(), namespace);
        let resource = api.get(name).await?;
        let meta = resource.meta().clone();
        let mut detail = Self::extract_metadata(&meta, resource_type);
        detail.yaml = serde_yml::to_string(&resource).unwrap_or_default();
        detail.events = Self::get_resource_events(client, name, namespace).await;
        Ok(detail)
    }

    /// Detail for PersistentVolumes (cluster-scoped)
    async fn get_pv_detail(client: Client, name: &str) -> Result<ResourceDetail> {
        let api: Api<PersistentVolume> = Api::all(client.clone());
        let pv = api.get(name).await?;
        let mut detail = Self::extract_metadata(&pv.metadata, "persistentvolumes");
        detail.yaml = serde_yml::to_string(&pv).unwrap_or_default();
        if let Some(status) = &pv.status {
            detail.phase = status.phase.clone().unwrap_or_default();
        }
        Ok(detail)
    }

    /// Detail for Namespaces (cluster-scoped)
    async fn get_ns_detail(client: Client, name: &str) -> Result<ResourceDetail> {
        let api: Api<Namespace> = Api::all(client.clone());
        let ns = api.get(name).await?;
        let mut detail = Self::extract_metadata(&ns.metadata, "namespaces");
        detail.yaml = serde_yml::to_string(&ns).unwrap_or_default();
        if let Some(status) = &ns.status {
            detail.phase = status.phase.clone().unwrap_or_default();
        }
        Ok(detail)
    }

    /// Generic detail for resource types without a specific handler
    async fn get_generic_detail(
        client: Client,
        resource_type: &str,
        name: &str,
        namespace: &str,
    ) -> Result<ResourceDetail> {
        // Use dynamic API to get any resource as JSON
        // For now, just fetch events and return a minimal detail
        let events = Self::get_resource_events(client, name, namespace).await;
        Ok(ResourceDetail {
            name: name.to_string(),
            namespace: Some(namespace.to_string()),
            resource_type: resource_type.to_string(),
            age: String::new(),
            phase: String::new(),
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            owner_references: vec![],
            conditions: vec![],
            containers: vec![],
            pods: vec![],
            yaml: String::new(),
            events,
        })
    }

    // ── List Methods ──────────────────────────────────────────────────

    async fn list_ingresses(client: Client, namespace: &str) -> Result<TableData> {
        let api: Api<Ingress> = Api::namespaced(client, namespace);
        let items = api.list(&ListParams::default()).await?;
        let columns = vec![
            TableColumn::new("NAME", 25),
            TableColumn::new("CLASS", 15),
            TableColumn::new("HOSTS", 30),
            TableColumn::new("AGE", 10),
        ];
        let rows = items
            .items
            .into_iter()
            .map(|ing| {
                let name = ing.name_any();
                let class = ing
                    .spec
                    .as_ref()
                    .and_then(|s| s.ingress_class_name.clone())
                    .unwrap_or_else(|| "<none>".into());
                let hosts = ing
                    .spec
                    .as_ref()
                    .and_then(|s| s.rules.as_ref())
                    .map(|rules| {
                        rules
                            .iter()
                            .filter_map(|r| r.host.as_ref())
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_else(|| "*".into());
                let age = ing
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|ts| format_age(&ts.0))
                    .unwrap_or_else(|| "Unknown".into());
                TableRow {
                    cells: vec![name, class, hosts, age],
                }
            })
            .collect();
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
