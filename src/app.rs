use gpui::*;
use gpui_component::input::InputState;
use gpui_component::table::{Table, TableEvent, TableState};
use gpui_component::theme::ActiveTheme;

use crate::k8s::{runtime::spawn_on_tokio, K8sClient};
use crate::model::detail::{DetailTab, ResourceDetail};
use crate::model::port_forward::{PodPort, PortForwardEntry, PortForwardStatus};
use crate::model::resources::{resource_index, RESOURCES};
use crate::model::table::TableData;
use crate::ui::context_picker::ContextPicker;
use crate::ui::detail_panel::DetailPanel;
use crate::ui::header::build_header;
use crate::ui::namespace_picker::NamespacePicker;
use crate::ui::port_forward_dialog::PortForwardDialog;
use crate::ui::port_forward_list::PortForwardList;
use crate::ui::resource_picker::ResourcePicker;
use crate::ui::resource_table::ResourceTableDelegate;
use crate::ui::sidebar::build_sidebar;
use crate::ui::status_bar::StatusBar;
use crate::ui::theme::PanelColors;

actions!(
    app,
    [
        MoveUp,
        MoveDown,
        Enter,
        GoBack,
        ActivateCommand,
        ActivateFilter,
        ToggleContextPicker,
        ToggleNamespacePicker,
        ToggleResourcePicker,
        ToggleSidebar,
        Backspace,
        DetailTab1,
        DetailTab2,
        DetailTab3,
        DetailTab4,
        RestartResource,
        ApplyYaml,
        OpenPortForward,
        StopPortForward,
    ]
);

#[derive(Clone, Copy, PartialEq)]
enum FocusPanel {
    Sidebar,
    Table,
}

/// A pending confirmation for a destructive action
#[derive(Clone)]
enum PendingConfirmation {
    Restart { name: String, resource_type: String },
    ApplyYaml,
    StartPortForward { description: String },
    StopPortForward { id: u64, description: String },
}

pub struct AppView {
    focus_handle: FocusHandle,
    active_panel: FocusPanel,
    current_resource: String,
    current_namespace: String,
    current_context: String,
    /// Cached lists for header dropdowns
    available_contexts: Vec<String>,
    available_namespaces: Vec<String>,
    table_data: TableData,
    pub table_state: Entity<TableState<ResourceTableDelegate>>,
    selected_row: usize,
    sidebar_selected: usize,
    command_input: String,
    command_mode: bool,
    status_message: String,
    loading: bool,
    spinner_frame: usize,
    filter_mode: bool,
    filter_text: String,
    // Detail view
    detail_visible: bool,
    detail_data: Option<ResourceDetail>,
    detail_tab: DetailTab,
    detail_loading: bool,
    detail_logs: Option<String>,
    detail_logs_loading: bool,
    /// YAML editor state (created when detail loads)
    yaml_editor: Option<Entity<InputState>>,
    /// Background task that refreshes pods in the detail view
    _detail_pods_refresh: Option<gpui::Task<()>>,
    // Port forward dialog
    pf_dialog_visible: bool,
    pf_dialog_pod_name: String,
    pf_dialog_namespace: String,
    pf_dialog_ports: Vec<PodPort>,
    pf_dialog_selected: usize,
    pf_dialog_local_port: String,
    pf_dialog_loading: bool,
    // Port forward list
    pf_list_visible: bool,
    pf_list_selected: usize,
    // Active port forwards
    port_forwards: Vec<PortForwardEntry>,
    pf_handles: Vec<(u64, tokio::process::Child)>,
    pf_next_id: u64,
    // Namespace picker
    ns_picker_visible: bool,
    ns_picker_loading: bool,
    ns_picker_list: Vec<String>,
    ns_picker_selected: usize,
    ns_picker_filter: String,
    // Context picker
    ctx_picker_visible: bool,
    ctx_picker_loading: bool,
    ctx_picker_list: Vec<String>,
    ctx_picker_selected: usize,
    ctx_picker_filter: String,
    // Resource picker
    res_picker_visible: bool,
    res_picker_selected: usize,
    res_picker_filter: String,
    // Confirmation banner for destructive actions
    pending_confirm: Option<PendingConfirmation>,
}

impl Focusable for AppView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl AppView {
    pub fn new(
        cx: &mut Context<Self>,
        window: &mut Window,
        namespace: &str,
        context: Option<&str>,
        resource: &str,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let sidebar_selected = resource_index(resource).unwrap_or(0);

        let delegate = ResourceTableDelegate::new(TableData::empty());
        let table_state = cx.new(|cx| {
            TableState::new(delegate, window, cx)
                .row_selectable(true)
                .col_resizable(true)
                .sortable(true)
        });

        let mut view = Self {
            focus_handle,
            active_panel: FocusPanel::Table,
            current_resource: resource.to_string(),
            current_namespace: namespace.to_string(),
            current_context: "unknown".to_string(),
            available_contexts: K8sClient::list_contexts().unwrap_or_default(),
            available_namespaces: vec![],
            table_data: TableData::empty(),
            table_state: table_state.clone(),
            selected_row: 0,
            sidebar_selected,
            command_input: String::new(),
            command_mode: false,
            status_message: "Connecting to cluster...".to_string(),
            loading: false,
            spinner_frame: 0,
            filter_mode: false,
            filter_text: String::new(),
            detail_visible: false,
            detail_data: None,
            detail_tab: DetailTab::Overview,
            detail_loading: false,
            detail_logs: None,
            detail_logs_loading: false,
            yaml_editor: None,
            _detail_pods_refresh: None,
            pf_dialog_visible: false,
            pf_dialog_pod_name: String::new(),
            pf_dialog_namespace: String::new(),
            pf_dialog_ports: vec![],
            pf_dialog_selected: 0,
            pf_dialog_local_port: String::new(),
            pf_dialog_loading: false,
            pf_list_visible: false,
            pf_list_selected: 0,
            port_forwards: vec![],
            pf_handles: vec![],
            pf_next_id: 1,
            ns_picker_visible: false,
            ns_picker_loading: false,
            ns_picker_list: vec![],
            ns_picker_selected: 0,
            ns_picker_filter: String::new(),
            ctx_picker_visible: false,
            ctx_picker_loading: false,
            ctx_picker_list: vec![],
            ctx_picker_selected: 0,
            ctx_picker_filter: String::new(),
            res_picker_visible: false,
            res_picker_selected: 0,
            res_picker_filter: String::new(),
            pending_confirm: None,
        };

        if let Some(ctx) = context {
            view.current_context = ctx.to_string();
            K8sClient::set_active_context(ctx);
            view.status_message = "Connected".to_string();
        } else {
            view.detect_context();
        }

        // Subscribe to table selection events
        cx.subscribe_in(
            &table_state,
            window,
            |this, _state, event: &TableEvent, _window, cx| match event {
                TableEvent::SelectRow(row_ix) => {
                    this.selected_row = *row_ix;
                    this.active_panel = FocusPanel::Table;
                    cx.notify();
                }
                TableEvent::DoubleClickedRow(_row_ix) => {
                    this.open_detail(cx);
                    cx.notify();
                }
                _ => {}
            },
        )
        .detach();

        view.load_resource_data(cx);
        view.load_available_namespaces(cx);
        view
    }

    fn detect_context(&mut self) {
        match K8sClient::current_context() {
            Ok(ctx) => {
                self.current_context = ctx;
                self.status_message = "Connected".to_string();
            }
            Err(e) => {
                self.status_message = format!("No cluster: {e}");
            }
        }
    }

    fn load_available_namespaces(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = spawn_on_tokio(async move {
                K8sClient::list_namespace_names().await
            })
            .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    if let Ok(mut names) = result {
                        names.sort();
                        this.available_namespaces = names;
                        cx.notify();
                    }
                })
            })
            .ok();
        })
        .detach();
    }

    fn switch_context(&mut self, context_name: &str, cx: &mut Context<Self>) {
        self.current_context = context_name.to_string();
        self.current_namespace = "default".to_string();
        self.available_namespaces.clear();
        self.status_message = format!("Switching to context {}...", context_name);

        let ctx = context_name.to_string();
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            // Switch kubectl context
            let context = ctx.clone();
            let result = spawn_on_tokio(async move {
                let output = tokio::process::Command::new("kubectl")
                    .args(["config", "use-context", &context])
                    .output()
                    .await;
                match output {
                    Ok(o) if o.status.success() => Ok(()),
                    Ok(o) => Err(anyhow::anyhow!(
                        String::from_utf8_lossy(&o.stderr).to_string()
                    )),
                    Err(e) => Err(anyhow::anyhow!("{e}")),
                }
            })
            .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    match result {
                        Ok(()) => {
                            this.status_message = format!("Context: {}", ctx);
                            this.load_resource_data(cx);
                            this.load_available_namespaces(cx);
                        }
                        Err(e) => {
                            this.status_message = format!("Failed to switch context: {e}");
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();
    }

    fn switch_namespace(&mut self, namespace: &str, cx: &mut Context<Self>) {
        self.current_namespace = namespace.to_string();
        self.load_resource_data(cx);
    }

    fn switch_resource(&mut self, api_name: &str, cx: &mut Context<Self>) {
        self.current_resource = api_name.to_string();
        self.sidebar_selected = resource_index(api_name).unwrap_or(self.sidebar_selected);
        self.filter_text.clear();
        self.filter_mode = false;
        self.selected_row = 0;
        self.detail_visible = false;
        self.detail_data = None;
        self.detail_logs = None;
        self.load_resource_data(cx);
    }

    fn load_resource_data(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.spinner_frame = 0;
        let resource = self.current_resource.clone();
        let namespace = self.current_namespace.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result =
                spawn_on_tokio(
                    async move { K8sClient::list_resources(&resource, &namespace).await },
                )
                .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.loading = false;
                    match result {
                        Ok(data) => {
                            this.table_data = data.clone();
                            this.selected_row = 0;
                            this.status_message = "Connected".to_string();
                            // Update the gpui-component table delegate
                            this.table_state.update(cx, |state, cx| {
                                state.delegate_mut().update_data(data);
                                state.refresh(cx);
                            });
                        }
                        Err(e) => {
                            this.status_message = format!("Error: {e}");
                            this.table_data = TableData::empty();
                            this.table_state.update(cx, |state, cx| {
                                state.delegate_mut().update_data(TableData::empty());
                                state.refresh(cx);
                            });
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();
    }

    fn load_namespaces(&mut self, cx: &mut Context<Self>) {
        self.ns_picker_loading = true;
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result =
                spawn_on_tokio(async move { K8sClient::list_namespace_names().await }).await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.ns_picker_loading = false;
                    match result {
                        Ok(mut names) => {
                            names.sort();
                            this.ns_picker_selected = names
                                .iter()
                                .position(|n| n == &this.current_namespace)
                                .unwrap_or(0);
                            this.ns_picker_list = names;
                        }
                        Err(e) => {
                            this.status_message = format!("Error listing namespaces: {e}");
                            this.ns_picker_visible = false;
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();
    }

    // ── Detail view methods ──

    fn open_detail(&mut self, cx: &mut Context<Self>) {
        let filtered = self.filtered_rows();
        let row = match filtered.get(self.selected_row) {
            Some((_, row)) => row,
            None => return,
        };
        let name = match row.cells.first() {
            Some(name) => name.clone(),
            None => return,
        };

        self.detail_visible = true;
        self.detail_loading = true;
        self.detail_data = None;
        self.detail_tab = DetailTab::Overview;
        self.detail_logs = None;
        self.detail_logs_loading = false;
        self.yaml_editor = None;
        // Exit filter/command mode when entering detail view
        self.filter_mode = false;
        self.command_mode = false;

        let resource_type = self.current_resource.clone();
        let namespace = self.current_namespace.clone();

        // Initial detail fetch
        let rt = resource_type.clone();
        let n = name.clone();
        let ns = namespace.clone();
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result =
                spawn_on_tokio(async move { K8sClient::get_resource_detail(&rt, &n, &ns).await })
                    .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.detail_loading = false;
                    match result {
                        Ok(detail) => {
                            this.detail_data = Some(detail);
                        }
                        Err(e) => {
                            this.status_message = format!("Error: {e}");
                            this.detail_visible = false;
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();

        // Start background pods refresh every 5 seconds
        self._detail_pods_refresh = Some(cx.spawn(async move |this, cx: &mut AsyncApp| {
            loop {
                // Sleep 5 seconds on the Tokio runtime
                spawn_on_tokio(async {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                })
                .await;

                // Check if still in detail view
                let should_continue = cx
                    .update(|cx| this.update(cx, |this, _cx| this.detail_visible).ok())
                    .ok()
                    .flatten()
                    .unwrap_or(false);

                if !should_continue {
                    break;
                }

                // Fetch fresh detail (which includes pods)
                let rt = resource_type.clone();
                let n = name.clone();
                let ns = namespace.clone();
                let result =
                    spawn_on_tokio(
                        async move { K8sClient::get_resource_detail(&rt, &n, &ns).await },
                    )
                    .await;

                let updated = cx.update(|cx| {
                    this.update(cx, |this, cx| {
                        if !this.detail_visible {
                            return false;
                        }
                        if let Ok(fresh) = result {
                            if let Some(existing) = &mut this.detail_data {
                                // Update pods, conditions, and phase without replacing
                                // the whole detail (preserves user's tab position etc.)
                                existing.pods = fresh.pods;
                                existing.conditions = fresh.conditions;
                                existing.phase = fresh.phase;
                                existing.containers = fresh.containers;
                                existing.events = fresh.events;
                            }
                            cx.notify();
                        }
                        true
                    })
                    .ok()
                    .unwrap_or(false)
                });

                if updated.ok() != Some(true) {
                    break;
                }
            }
        }));
    }

    /// Open detail view for a specific pod by name (used when clicking pod names in workload detail)
    fn open_pod_detail_by_name(&mut self, pod_name: &str, cx: &mut Context<Self>) {
        self.detail_visible = true;
        self.detail_loading = true;
        self.detail_data = None;
        self.detail_tab = DetailTab::Overview;
        self.detail_logs = None;
        self.detail_logs_loading = false;
        self.yaml_editor = None;

        let name = pod_name.to_string();
        let namespace = self.current_namespace.clone();

        let n = name.clone();
        let ns = namespace.clone();
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result =
                spawn_on_tokio(
                    async move { K8sClient::get_resource_detail("pods", &n, &ns).await },
                )
                .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.detail_loading = false;
                    match result {
                        Ok(detail) => {
                            this.detail_data = Some(detail);
                        }
                        Err(e) => {
                            this.status_message = format!("Error: {e}");
                            this.detail_visible = false;
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();

        // Start pod refresh loop
        let resource_type = "pods".to_string();
        self._detail_pods_refresh = Some(cx.spawn(async move |this, cx: &mut AsyncApp| loop {
            spawn_on_tokio(async {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            })
            .await;

            let should_continue = cx
                .update(|cx| this.update(cx, |this, _cx| this.detail_visible).ok())
                .ok()
                .flatten()
                .unwrap_or(false);

            if !should_continue {
                break;
            }

            let rt = resource_type.clone();
            let n = name.clone();
            let ns = namespace.clone();
            let result =
                spawn_on_tokio(async move { K8sClient::get_resource_detail(&rt, &n, &ns).await })
                    .await;

            let updated = cx.update(|cx| {
                this.update(cx, |this, cx| {
                    if !this.detail_visible {
                        return false;
                    }
                    if let Ok(fresh) = result {
                        if let Some(existing) = &mut this.detail_data {
                            existing.pods = fresh.pods;
                            existing.conditions = fresh.conditions;
                            existing.phase = fresh.phase;
                            existing.containers = fresh.containers;
                            existing.events = fresh.events;
                        }
                        cx.notify();
                    }
                    true
                })
                .ok()
                .unwrap_or(false)
            });

            if updated.ok() != Some(true) {
                break;
            }
        }));
    }

    fn close_detail(&mut self) {
        self.detail_visible = false;
        self.detail_data = None;
        self.detail_logs = None;
        self.detail_logs_loading = false;
        self.yaml_editor = None;
        self._detail_pods_refresh = None;
    }

    fn can_restart(&self) -> bool {
        matches!(
            self.current_resource.as_str(),
            "pods" | "deployments" | "statefulsets" | "daemonsets"
        )
    }

    fn can_port_forward(&self) -> bool {
        matches!(
            self.current_resource.as_str(),
            "pods" | "deployments" | "statefulsets" | "daemonsets"
        )
    }

    fn restart_current_resource(&mut self, cx: &mut Context<Self>) {
        // Determine what to restart: detail view resource or selected table row
        let (name, resource_type) = if let Some(detail) = &self.detail_data {
            if self.detail_visible {
                (detail.name.clone(), detail.resource_type.clone())
            } else {
                return;
            }
        } else {
            // From table view
            let filtered = self.filtered_rows();
            let row = match filtered.get(self.selected_row) {
                Some((_, row)) => row,
                None => return,
            };
            match row.cells.first() {
                Some(name) => (name.clone(), self.current_resource.clone()),
                None => return,
            }
        };

        if !matches!(
            resource_type.as_str(),
            "pods" | "deployments" | "statefulsets" | "daemonsets"
        ) {
            self.status_message = format!("Restart not supported for {resource_type}");
            return;
        }

        let namespace = self.current_namespace.clone();
        self.status_message = format!("Restarting {name}...");

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let rt = resource_type.clone();
            let n = name.clone();
            let ns = namespace.clone();
            let result =
                spawn_on_tokio(async move { K8sClient::restart_resource(&rt, &n, &ns).await })
                    .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    match result {
                        Ok(msg) => {
                            this.status_message = msg;
                            // Reload data after restart
                            if this.detail_visible {
                                this.open_detail(cx);
                            } else {
                                this.load_resource_data(cx);
                            }
                        }
                        Err(e) => {
                            this.status_message = format!("Restart failed: {e}");
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();
    }

    /// Ensure the YAML editor exists and is populated with current YAML
    fn ensure_yaml_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.yaml_editor.is_some() {
            return;
        }
        let yaml = self
            .detail_data
            .as_ref()
            .map(|d| d.yaml.clone())
            .unwrap_or_default();

        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("yaml")
                .line_number(true)
                .soft_wrap(false)
                .default_value(yaml)
        });
        self.yaml_editor = Some(editor);
    }

    fn apply_yaml(&mut self, cx: &mut Context<Self>) {
        let editor = match &self.yaml_editor {
            Some(e) => e,
            None => return,
        };
        let yaml_text = editor.read(cx).value().to_string();
        if yaml_text.trim().is_empty() {
            self.status_message = "YAML is empty".to_string();
            return;
        }

        let namespace = self.current_namespace.clone();
        self.status_message = "Applying YAML...".to_string();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let ns = namespace.clone();
            let yaml = yaml_text.clone();
            let result =
                spawn_on_tokio(async move { K8sClient::apply_yaml(&yaml, &ns).await }).await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    match result {
                        Ok(msg) => {
                            this.status_message = msg;
                            // Reload detail
                            this.open_detail(cx);
                        }
                        Err(e) => {
                            this.status_message = format!("Apply failed: {e}");
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();
    }

    // ── Port forward methods ──

    fn open_port_forward_dialog(&mut self, cx: &mut Context<Self>) {
        // Determine pod name: from detail view or selected table row
        let (pod_name, namespace) = if self.detail_visible {
            if let Some(d) = &self.detail_data {
                if d.resource_type == "pods" {
                    (
                        d.name.clone(),
                        d.namespace
                            .clone()
                            .unwrap_or(self.current_namespace.clone()),
                    )
                } else {
                    self.status_message = "Port forward is only available for pods".to_string();
                    return;
                }
            } else {
                return;
            }
        } else if self.current_resource == "pods" {
            let filtered = self.filtered_rows();
            match filtered.get(self.selected_row) {
                Some((_, row)) => match row.cells.first() {
                    Some(name) => (name.clone(), self.current_namespace.clone()),
                    None => return,
                },
                None => return,
            }
        } else {
            self.status_message = "Port forward is only available for pods".to_string();
            return;
        };

        self.pf_dialog_visible = true;
        self.pf_dialog_pod_name = pod_name.clone();
        self.pf_dialog_namespace = namespace.clone();
        self.pf_dialog_ports = vec![];
        self.pf_dialog_selected = 0;
        self.pf_dialog_local_port.clear();
        self.pf_dialog_loading = true;

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let n = pod_name.clone();
            let ns = namespace.clone();
            let result =
                spawn_on_tokio(async move { K8sClient::get_pod_ports(&n, &ns).await }).await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.pf_dialog_loading = false;
                    match result {
                        Ok(ports) => {
                            this.pf_dialog_ports = ports;
                            // Auto-suggest local port = remote port
                            if let Some(first) = this.pf_dialog_ports.first() {
                                this.pf_dialog_local_port = first.port.to_string();
                            }
                        }
                        Err(e) => {
                            this.status_message = format!("Error: {e}");
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();
    }

    fn start_port_forward(&mut self, cx: &mut Context<Self>) {
        let remote_port = if self.pf_dialog_ports.is_empty() {
            // Try parsing local_port as remote too
            match self.pf_dialog_local_port.parse::<u16>() {
                Ok(p) => p,
                Err(_) => {
                    self.status_message = "Invalid port number".to_string();
                    return;
                }
            }
        } else {
            match self.pf_dialog_ports.get(self.pf_dialog_selected) {
                Some(p) => p.port,
                None => return,
            }
        };

        let local_port = if self.pf_dialog_local_port.is_empty() {
            remote_port
        } else {
            match self.pf_dialog_local_port.parse::<u16>() {
                Ok(p) => p,
                Err(_) => {
                    self.status_message = "Invalid local port number".to_string();
                    return;
                }
            }
        };

        let pod_name = self.pf_dialog_pod_name.clone();
        let namespace = self.pf_dialog_namespace.clone();
        let id = self.pf_next_id;
        self.pf_next_id += 1;

        self.pf_dialog_visible = false;
        self.status_message =
            format!("Starting port forward {local_port} -> {pod_name}:{remote_port}...");

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let pn = pod_name.clone();
            let ns = namespace.clone();
            let result = spawn_on_tokio(async move {
                K8sClient::start_port_forward(&pn, &ns, local_port, remote_port).await
            })
            .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    match result {
                        Ok(child) => {
                            this.port_forwards.push(PortForwardEntry {
                                id,
                                pod_name: pod_name.clone(),
                                namespace: namespace.clone(),
                                local_port,
                                remote_port,
                                status: PortForwardStatus::Active,
                                started_at: chrono::Utc::now().format("%H:%M:%S").to_string(),
                            });
                            this.pf_handles.push((id, child));
                            this.status_message = format!(
                                "Port forward {local_port} -> {pod_name}:{remote_port} started"
                            );
                        }
                        Err(e) => {
                            this.status_message = format!("Port forward failed: {e}");
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();
    }

    fn stop_port_forward(&mut self, id: u64) {
        // Kill the kubectl process
        if let Some(pos) = self.pf_handles.iter().position(|(hid, _)| *hid == id) {
            let (_, mut child) = self.pf_handles.remove(pos);
            // kill_on_drop is set, but let's be explicit
            let _ = child.start_kill();
        }

        // Update status
        if let Some(entry) = self.port_forwards.iter_mut().find(|e| e.id == id) {
            entry.status = PortForwardStatus::Stopped;
            self.status_message = format!(
                "Stopped port forward {} -> {}:{}",
                entry.local_port, entry.pod_name, entry.remote_port
            );
        }
    }

    fn check_port_forward_health(&mut self) {
        for (id, child) in &mut self.pf_handles {
            if let Ok(Some(status)) = child.try_wait() {
                let id = *id;
                if let Some(entry) = self.port_forwards.iter_mut().find(|e| e.id == id) {
                    if matches!(entry.status, PortForwardStatus::Active) {
                        entry.status =
                            PortForwardStatus::Failed(format!("Process exited with {status}"));
                    }
                }
            }
        }
        // Remove finished handles
        self.pf_handles
            .retain_mut(|(_, child)| child.try_wait().ok().flatten().is_none());
    }

    fn active_pf_count(&self) -> usize {
        self.port_forwards
            .iter()
            .filter(|e| matches!(e.status, PortForwardStatus::Active))
            .count()
    }

    fn switch_detail_tab(&mut self, tab: DetailTab, cx: &mut Context<Self>) {
        self.detail_tab = tab;
        if tab == DetailTab::Logs && self.detail_logs.is_none() && !self.detail_logs_loading {
            self.load_detail_logs(cx);
        }
    }

    fn load_detail_logs(&mut self, cx: &mut Context<Self>) {
        let (name, resource_type) = match &self.detail_data {
            Some(d) => (d.name.clone(), d.resource_type.clone()),
            None => return,
        };

        // Check if this resource type supports logs
        if !matches!(
            resource_type.as_str(),
            "pods" | "deployments" | "statefulsets" | "daemonsets" | "replicasets" | "jobs"
        ) {
            self.detail_logs = Some(format!("Logs not available for {resource_type}"));
            return;
        }

        let namespace = self.current_namespace.clone();
        self.detail_logs_loading = true;

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = spawn_on_tokio(async move {
                K8sClient::get_resource_logs(&resource_type, &name, &namespace, Some(500)).await
            })
            .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.detail_logs_loading = false;
                    match result {
                        Ok(logs) => {
                            this.detail_logs = Some(logs);
                        }
                        Err(e) => {
                            this.detail_logs = Some(format!("Error loading logs: {e}"));
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();
    }

    // ── Selection / filtering ──

    fn filtered_rows(&self) -> Vec<(usize, &crate::model::table::TableRow)> {
        if self.filter_text.is_empty() {
            self.table_data.rows.iter().enumerate().collect()
        } else {
            let filter = self.filter_text.to_lowercase();
            self.table_data
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    row.cells
                        .iter()
                        .any(|cell| cell.to_lowercase().contains(&filter))
                })
                .collect()
        }
    }

    fn update_table_filter(&mut self, cx: &mut Context<Self>) {
        let filtered_rows = self.filtered_rows();
        let filtered_table = TableData {
            columns: self.table_data.columns.clone(),
            rows: filtered_rows
                .iter()
                .map(|(_, row)| (*row).clone())
                .collect(),
        };
        self.table_state.update(cx, |state, cx| {
            state.delegate_mut().update_data(filtered_table);
            state.refresh(cx);
        });
    }

    fn move_selection(&mut self, delta: i32) {
        if self.pf_dialog_visible {
            let count = self.pf_dialog_ports.len();
            if count == 0 {
                return;
            }
            let new_idx = self.pf_dialog_selected as i32 + delta;
            self.pf_dialog_selected = new_idx.clamp(0, count as i32 - 1) as usize;
            // Update local port suggestion
            if let Some(p) = self.pf_dialog_ports.get(self.pf_dialog_selected) {
                self.pf_dialog_local_port = p.port.to_string();
            }
            return;
        }
        if self.pf_list_visible {
            let count = self.port_forwards.len();
            if count == 0 {
                return;
            }
            let new_idx = self.pf_list_selected as i32 + delta;
            self.pf_list_selected = new_idx.clamp(0, count as i32 - 1) as usize;
            return;
        }
        if self.res_picker_visible {
            let filtered = self.filtered_resources();
            let count = filtered.len();
            if count == 0 {
                return;
            }
            let new_idx = self.res_picker_selected as i32 + delta;
            self.res_picker_selected = new_idx.clamp(0, count as i32 - 1) as usize;
        } else if self.ctx_picker_visible {
            let filtered = self.filtered_contexts();
            let count = filtered.len();
            if count == 0 {
                return;
            }
            let new_idx = self.ctx_picker_selected as i32 + delta;
            self.ctx_picker_selected = new_idx.clamp(0, count as i32 - 1) as usize;
        } else if self.ns_picker_visible {
            let filtered = self.filtered_namespaces();
            let count = filtered.len();
            if count == 0 {
                return;
            }
            let new_idx = self.ns_picker_selected as i32 + delta;
            self.ns_picker_selected = new_idx.clamp(0, count as i32 - 1) as usize;
        } else if self.active_panel == FocusPanel::Sidebar {
            let count = RESOURCES.len();
            let new_idx = self.sidebar_selected as i32 + delta;
            self.sidebar_selected = new_idx.clamp(0, count as i32 - 1) as usize;
        } else {
            let row_count = self.filtered_rows().len();
            if row_count == 0 {
                return;
            }
            let new_idx = self.selected_row as i32 + delta;
            self.selected_row = new_idx.clamp(0, row_count as i32 - 1) as usize;
        }
    }

    fn filtered_namespaces(&self) -> Vec<String> {
        if self.ns_picker_filter.is_empty() {
            self.ns_picker_list.clone()
        } else {
            let filter = self.ns_picker_filter.to_lowercase();
            self.ns_picker_list
                .iter()
                .filter(|ns| ns.to_lowercase().contains(&filter))
                .cloned()
                .collect()
        }
    }

    fn select_namespace(&mut self, cx: &mut Context<Self>) {
        let filtered = self.filtered_namespaces();
        if let Some(ns) = filtered.get(self.ns_picker_selected) {
            self.current_namespace = ns.clone();
            self.ns_picker_visible = false;
            self.ns_picker_filter.clear();

            self.load_resource_data(cx);
        }
    }

    fn toggle_namespace_picker(&mut self, cx: &mut Context<Self>) {
        if self.ns_picker_visible {
            self.ns_picker_visible = false;
            self.ns_picker_filter.clear();

        } else {
            self.ns_picker_visible = true;
            self.ns_picker_filter.clear();
            self.ns_picker_selected = 0;

            self.load_namespaces(cx);
        }
    }

    // ── Context picker methods ──

    fn load_contexts(&mut self, cx: &mut Context<Self>) {
        self.ctx_picker_loading = true;
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            // Reading kubeconfig is synchronous but we do it off the main thread
            let result = K8sClient::list_contexts();
            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.ctx_picker_loading = false;
                    match result {
                        Ok(mut names) => {
                            names.sort();
                            this.ctx_picker_selected = names
                                .iter()
                                .position(|n| n == &this.current_context)
                                .unwrap_or(0);
                            this.ctx_picker_list = names;
                        }
                        Err(e) => {
                            this.status_message = format!("Error listing contexts: {e}");
                            this.ctx_picker_visible = false;
                        }
                    }
                    cx.notify();
                })
            })
            .ok();
        })
        .detach();
    }

    fn filtered_contexts(&self) -> Vec<String> {
        if self.ctx_picker_filter.is_empty() {
            self.ctx_picker_list.clone()
        } else {
            let filter = self.ctx_picker_filter.to_lowercase();
            self.ctx_picker_list
                .iter()
                .filter(|c| c.to_lowercase().contains(&filter))
                .cloned()
                .collect()
        }
    }

    fn select_context(&mut self, cx: &mut Context<Self>) {
        let filtered = self.filtered_contexts();
        if let Some(ctx) = filtered.get(self.ctx_picker_selected) {
            self.current_context = ctx.clone();
            K8sClient::set_active_context(ctx);
            self.ctx_picker_visible = false;
            self.ctx_picker_filter.clear();

            // Reset namespace to default and reload
            self.current_namespace = "default".to_string();
            self.load_resource_data(cx);
        }
    }

    fn toggle_context_picker(&mut self, cx: &mut Context<Self>) {
        if self.ctx_picker_visible {
            self.ctx_picker_visible = false;
            self.ctx_picker_filter.clear();

        } else {
            self.ctx_picker_visible = true;
            self.ctx_picker_filter.clear();
            self.ctx_picker_selected = 0;

            self.load_contexts(cx);
        }
    }

    // ── Resource picker methods ──

    fn filtered_resources(&self) -> Vec<(String, String, String)> {
        let all: Vec<(String, String, String)> = RESOURCES
            .iter()
            .map(|r| {
                (
                    r.display_name.to_string(),
                    r.api_name.to_string(),
                    r.category.to_string(),
                )
            })
            .collect();
        if self.res_picker_filter.is_empty() {
            all
        } else {
            let filter = self.res_picker_filter.to_lowercase();
            all.into_iter()
                .filter(|(display, api, _)| {
                    display.to_lowercase().contains(&filter)
                        || api.to_lowercase().contains(&filter)
                })
                .collect()
        }
    }

    fn select_resource(&mut self, cx: &mut Context<Self>) {
        let filtered = self.filtered_resources();
        if let Some((_, api_name, _)) = filtered.get(self.res_picker_selected) {
            let api_name = api_name.clone();
            self.res_picker_visible = false;
            self.res_picker_filter.clear();

            self.switch_resource(&api_name, cx);
        }
    }

    // ── Confirmation methods ──

    fn confirm_message(confirm: &PendingConfirmation) -> String {
        match confirm {
            PendingConfirmation::Restart {
                name,
                resource_type,
            } => format!("Restart {} '{}'?", resource_type, name),
            PendingConfirmation::ApplyYaml => "Apply YAML changes?".to_string(),
            PendingConfirmation::StartPortForward { description } => {
                format!("Start port forward {}?", description)
            }
            PendingConfirmation::StopPortForward { description, .. } => {
                format!("Stop port forward {}?", description)
            }
        }
    }

    fn execute_confirmed(&mut self, cx: &mut Context<Self>) {
        let confirm = match self.pending_confirm.take() {
            Some(c) => c,
            None => return,
        };
        match confirm {
            PendingConfirmation::Restart { .. } => {
                self.restart_current_resource(cx);
            }
            PendingConfirmation::ApplyYaml => {
                self.apply_yaml(cx);
            }
            PendingConfirmation::StartPortForward { .. } => {
                self.start_port_forward(cx);
            }
            PendingConfirmation::StopPortForward { id, .. } => {
                self.stop_port_forward(id);
            }
        }
    }

    /// Returns true if any picker overlay is currently open
    fn any_picker_visible(&self) -> bool {
        self.ns_picker_visible || self.ctx_picker_visible || self.res_picker_visible
    }

    /// Get mutable ref to the active picker's filter string
    fn active_picker_filter_mut(&mut self) -> Option<&mut String> {
        if self.ns_picker_visible {
            Some(&mut self.ns_picker_filter)
        } else if self.ctx_picker_visible {
            Some(&mut self.ctx_picker_filter)
        } else if self.res_picker_visible {
            Some(&mut self.res_picker_filter)
        } else {
            None
        }
    }

    /// Reset the active picker's selection index to 0
    fn reset_active_picker_selection(&mut self) {
        if self.ns_picker_visible {
            self.ns_picker_selected = 0;
        } else if self.ctx_picker_visible {
            self.ctx_picker_selected = 0;
        } else if self.res_picker_visible {
            self.res_picker_selected = 0;
        }
    }

    fn toggle_resource_picker(&mut self) {
        if self.res_picker_visible {
            self.res_picker_visible = false;
            self.res_picker_filter.clear();

        } else {
            self.res_picker_visible = true;
            self.res_picker_filter.clear();
            self.res_picker_selected = RESOURCES
                .iter()
                .position(|r| r.api_name == self.current_resource)
                .unwrap_or(0);

        }
    }

    fn handle_command(&mut self, cx: &mut Context<Self>) {
        let cmd = self.command_input.trim().to_string();
        self.command_input.clear();
        self.command_mode = false;

        if cmd.is_empty() {
            return;
        }

        // Special commands
        if matches!(cmd.as_str(), "pf" | "portforward" | "port-forwards") {
            self.pf_list_visible = true;
            self.pf_list_selected = 0;
            return;
        }

        let resource = match cmd.as_str() {
            "po" | "pods" | "pod" => "pods",
            "dp" | "deploy" | "deployments" | "deployment" => "deployments",
            "svc" | "services" | "service" => "services",
            "no" | "nodes" | "node" => "nodes",
            "ns" | "namespaces" | "namespace" => "namespaces",
            "ds" | "daemonsets" | "daemonset" => "daemonsets",
            "sts" | "statefulsets" | "statefulset" => "statefulsets",
            "rs" | "replicasets" | "replicaset" => "replicasets",
            "cm" | "configmaps" | "configmap" => "configmaps",
            "sec" | "secrets" | "secret" => "secrets",
            "sa" | "serviceaccounts" | "serviceaccount" => "serviceaccounts",
            "ing" | "ingresses" | "ingress" => "ingresses",
            "pv" | "persistentvolumes" => "persistentvolumes",
            "pvc" | "persistentvolumeclaims" => "persistentvolumeclaims",
            "ev" | "events" | "event" => "events",
            "cj" | "cronjobs" | "cronjob" => "cronjobs",
            "job" | "jobs" => "jobs",
            other => other,
        };

        self.switch_resource(resource, cx);
    }
}

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Animate spinner while anything is loading
        let any_loading = self.loading
            || self.detail_loading
            || self.detail_logs_loading
            || self.ns_picker_loading
            || self.ctx_picker_loading;
        if any_loading {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
            cx.on_next_frame(window, |this, _window, cx| {
                if this.loading
                    || this.detail_loading
                    || this.detail_logs_loading
                    || this.ns_picker_loading
                    || this.ctx_picker_loading
                {
                    cx.notify();
                }
            });
        }

        // Ensure YAML editor exists when detail data is available
        if self.detail_visible && self.detail_data.is_some() {
            self.ensure_yaml_editor(window, cx);
        }

        // When detail or picker is visible, focus the app root so key bindings work
        // (the table captures arrow keys otherwise)
        if self.detail_visible || self.any_picker_visible() {
            if !self.focus_handle.is_focused(window) {
                self.focus_handle.focus(window);
            }
        }

        let weak = cx.weak_entity();

        let weak_ctx = weak.clone();
        let weak_ns = weak.clone();
        let weak_res = weak.clone();

        let header = build_header(
            &self.current_context,
            &self.current_namespace,
            &self.current_resource,
            &self.available_contexts,
            std::rc::Rc::new(move |ctx, _window, cx| {
                weak_ctx.update(cx, |this, cx| {
                    this.switch_context(&ctx, cx);
                    cx.notify();
                }).ok();
            }),
            std::rc::Rc::new(move |_ev, _window, cx| {
                weak_ns.update(cx, |this, cx| {
                    this.toggle_namespace_picker(cx);
                    cx.notify();
                }).ok();
            }),
            std::rc::Rc::new(move |res, _window, cx| {
                weak_res.update(cx, |this, cx| {
                    this.switch_resource(&res, cx);
                    cx.notify();
                }).ok();
            }),
            cx,
        );

        // (Table data is updated in load_resource_data and update_table_filter)

        let pf_count = self.active_pf_count();

        let loading = self.loading;
        let spinner_text =
            SharedString::from(SPINNER_FRAMES[self.spinner_frame % SPINNER_FRAMES.len()]);
        let loading_resource = self.current_resource.clone();

        let status_msg = if pf_count > 0 {
            format!("{} | PF: {} active", self.status_message, pf_count)
        } else {
            self.status_message.clone()
        };
        let status = StatusBar::new(
            &status_msg,
            self.command_mode,
            &self.command_input,
            self.filter_mode,
            &self.filter_text,
        );

        let mut root = div()
            .id("app-root")
            .key_context("app")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_action(cx.listener(|this, _: &MoveUp, _window, cx| {
                if this.filter_mode && !this.any_picker_visible() {
                    this.filter_mode = false; // done typing, now navigating
                }
                this.move_selection(-1);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &MoveDown, _window, cx| {
                if this.filter_mode && !this.any_picker_visible() {
                    this.filter_mode = false; // done typing, now navigating
                }
                this.move_selection(1);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ActivateCommand, _window, cx| {
                if !this.any_picker_visible() {
                    this.command_mode = true;
                    this.command_input.clear();
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ActivateFilter, _window, cx| {
                if this.any_picker_visible() {
                    // `/` inside a picker is just another filter char — ignore the action
                } else if !this.detail_visible {
                    this.filter_mode = true;
                    this.filter_text.clear();
                    this.selected_row = 0;
                    this.update_table_filter(cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &GoBack, window, cx| {
                if this.pending_confirm.is_some() {
                    this.pending_confirm = None;
                } else if this.pf_dialog_visible {
                    this.pf_dialog_visible = false;
                } else if this.pf_list_visible {
                    this.pf_list_visible = false;
                } else if this.any_picker_visible() {
                    // If there's filter text, first Esc clears it; second Esc closes picker
                    let has_filter = this.active_picker_filter_mut()
                        .map(|f| !f.is_empty())
                        .unwrap_or(false);
                    if has_filter {
                        if let Some(f) = this.active_picker_filter_mut() {
                            f.clear();
                        }
                        this.reset_active_picker_selection();
                    } else {
                        this.res_picker_visible = false;
                        this.res_picker_filter.clear();
                        this.ctx_picker_visible = false;
                        this.ctx_picker_filter.clear();
                        this.ns_picker_visible = false;
                        this.ns_picker_filter.clear();
                    }
                } else if this.detail_visible {
                    this.close_detail();
                    // Re-focus the table
                    let handle = this.table_state.read(cx).focus_handle(cx);
                    handle.focus(window);
                } else if this.filter_mode {
                    this.filter_mode = false;
                    this.filter_text.clear();
                    this.selected_row = 0;
                    this.update_table_filter(cx);
                } else if this.command_mode {
                    this.command_mode = false;
                    this.command_input.clear();
                } else if this.active_panel == FocusPanel::Sidebar {
                    this.active_panel = FocusPanel::Table;
                    let handle = this.table_state.read(cx).focus_handle(cx);
                    handle.focus(window);
                } else if this.current_resource == "namespaces" {
                    this.switch_resource("pods", cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Enter, window, cx| {
                if this.pending_confirm.is_some() {
                    this.execute_confirmed(cx);
                } else if this.pf_dialog_visible {
                    let desc = format!(
                        "{}:{} -> localhost:{}",
                        this.pf_dialog_pod_name,
                        this.pf_dialog_ports
                            .get(this.pf_dialog_selected)
                            .map(|p| p.port.to_string())
                            .unwrap_or_default(),
                        this.pf_dialog_local_port
                    );
                    this.pending_confirm = Some(PendingConfirmation::StartPortForward {
                        description: desc,
                    });
                } else if this.res_picker_visible {
                    this.select_resource(cx);
                } else if this.ctx_picker_visible {
                    this.select_context(cx);
                } else if this.ns_picker_visible {
                    this.select_namespace(cx);
                } else if this.filter_mode {
                    this.filter_mode = false;
                } else if this.command_mode {
                    this.handle_command(cx);
                } else if this.detail_visible {
                    // Enter in detail view does nothing extra
                } else if this.active_panel == FocusPanel::Sidebar {
                    if let Some(entry) = RESOURCES.get(this.sidebar_selected) {
                        let api_name = entry.api_name.to_string();
                        this.active_panel = FocusPanel::Table;
                        this.switch_resource(&api_name, cx);
                        // Focus table for keyboard nav
                        let handle = this.table_state.read(cx).focus_handle(cx);
                        handle.focus(window);
                    }
                } else if this.current_resource == "namespaces" {
                    if let Some(row) = this.table_data.rows.get(this.selected_row) {
                        if let Some(ns_name) = row.cells.first() {
                            this.current_namespace = ns_name.clone();
                            this.switch_resource("pods", cx);
                        }
                    }
                } else {
                    // Open detail view for selected resource
                    this.open_detail(cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleNamespacePicker, _window, cx| {
                if !this.any_picker_visible() && !this.pf_dialog_visible && !this.pf_list_visible {
                    this.toggle_namespace_picker(cx);
                } else if this.ns_picker_visible {
                    this.toggle_namespace_picker(cx); // close if already open
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleContextPicker, _window, cx| {
                if !this.any_picker_visible() && !this.pf_dialog_visible && !this.pf_list_visible {
                    this.toggle_context_picker(cx);
                } else if this.ctx_picker_visible {
                    this.toggle_context_picker(cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleResourcePicker, _window, cx| {
                if !this.any_picker_visible() && !this.pf_dialog_visible && !this.pf_list_visible {
                    this.toggle_resource_picker();
                } else if this.res_picker_visible {
                    this.toggle_resource_picker();
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, window, cx| {
                if !this.detail_visible && !this.any_picker_visible() && !this.pf_dialog_visible && !this.pf_list_visible {
                    this.active_panel = match this.active_panel {
                        FocusPanel::Sidebar => {
                            // Focus the table so keyboard nav works
                            let handle = this.table_state.read(cx).focus_handle(cx);
                            handle.focus(window);
                            FocusPanel::Table
                        }
                        FocusPanel::Table => {
                            // Focus back to app root for sidebar nav
                            this.focus_handle.focus(window);
                            FocusPanel::Sidebar
                        }
                    };
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Backspace, _window, cx| {
                if this.pf_dialog_visible {
                    this.pf_dialog_local_port.pop();
                } else if this.any_picker_visible() {
                    if let Some(f) = this.active_picker_filter_mut() {
                        f.pop();
                    }
                    this.reset_active_picker_selection();
                } else if this.filter_mode {
                    this.filter_text.pop();
                    this.selected_row = 0;
                    this.update_table_filter(cx);
                } else if this.command_mode {
                    this.command_input.pop();
                }
                cx.notify();
            }))
            // Detail tab switching (1-4) — guarded when pickers open
            .on_action(cx.listener(|this, _: &DetailTab1, _window, cx| {
                if this.detail_visible && !this.any_picker_visible() {
                    this.switch_detail_tab(DetailTab::Overview, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &DetailTab2, _window, cx| {
                if this.detail_visible && !this.any_picker_visible() {
                    this.switch_detail_tab(DetailTab::Yaml, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &DetailTab3, _window, cx| {
                if this.detail_visible && !this.any_picker_visible() {
                    this.switch_detail_tab(DetailTab::Events, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &DetailTab4, _window, cx| {
                if this.detail_visible && !this.any_picker_visible() {
                    this.switch_detail_tab(DetailTab::Logs, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &RestartResource, _window, cx| {
                if !this.command_mode
                    && !this.filter_mode
                    && !this.any_picker_visible()
                    && !this.pf_dialog_visible
                    && !this.pf_list_visible
                    && this.pending_confirm.is_none()
                {
                    // Determine what would be restarted
                    let target = if this.detail_visible {
                        this.detail_data
                            .as_ref()
                            .map(|d| (d.name.clone(), d.resource_type.clone()))
                    } else {
                        let filtered = this.filtered_rows();
                        filtered.get(this.selected_row).and_then(|(_, row)| {
                            row.cells
                                .first()
                                .map(|n| (n.clone(), this.current_resource.clone()))
                        })
                    };
                    match target {
                        Some((name, resource_type)) => {
                            if matches!(
                                resource_type.as_str(),
                                "pods" | "deployments" | "statefulsets" | "daemonsets"
                            ) {
                                this.pending_confirm =
                                    Some(PendingConfirmation::Restart { name, resource_type });
                            } else {
                                this.status_message =
                                    format!("Restart not supported for {resource_type}");
                            }
                        }
                        None => {
                            this.status_message = "No resource selected to restart".to_string();
                        }
                    }
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &ApplyYaml, _window, cx| {
                if this.detail_visible
                    && this.detail_tab == DetailTab::Yaml
                    && !this.any_picker_visible()
                    && this.pending_confirm.is_none()
                {
                    this.pending_confirm = Some(PendingConfirmation::ApplyYaml);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &OpenPortForward, _window, cx| {
                if !this.command_mode
                    && !this.filter_mode
                    && !this.pf_dialog_visible
                    && !this.pf_list_visible
                    && !this.any_picker_visible()
                {
                    this.open_port_forward_dialog(cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &StopPortForward, _window, cx| {
                if this.pf_list_visible
                    && !this.any_picker_visible()
                    && this.pending_confirm.is_none()
                {
                    if let Some(entry) = this.port_forwards.get(this.pf_list_selected) {
                        let desc = format!(
                            "{}:{} -> {}",
                            entry.pod_name, entry.remote_port, entry.local_port
                        );
                        this.pending_confirm = Some(PendingConfirmation::StopPortForward {
                            id: entry.id,
                            description: desc,
                        });
                    }
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                if this.pf_dialog_visible {
                    // Type digits for local port
                    if let Some(key_char) = &event.keystroke.key_char {
                        if key_char.chars().all(|c| c.is_ascii_digit()) {
                            this.pf_dialog_local_port.push_str(key_char);
                        }
                    }
                } else if this.any_picker_visible() {
                    // All chars go to filter (arrows handled by MoveUp/MoveDown actions)
                    if let Some(key_char) = &event.keystroke.key_char {
                        if let Some(f) = this.active_picker_filter_mut() {
                            f.push_str(key_char);
                        }
                        this.reset_active_picker_selection();
                    }
                } else if this.filter_mode {
                    if let Some(key_char) = &event.keystroke.key_char {
                        this.filter_text.push_str(key_char);
                        this.selected_row = 0;
                        this.update_table_filter(cx);
                    }
                } else if this.command_mode {
                    if let Some(key_char) = &event.keystroke.key_char {
                        this.command_input.push_str(key_char);
                    }
                }
                cx.notify();
            }))
            // Header
            .child(header);

        // Confirmation banner
        if let Some(confirm) = &self.pending_confirm {
            let msg = Self::confirm_message(confirm);
            root = root.child(
                div()
                    .w_full()
                    .px_4()
                    .py_2()
                    .bg(rgb(0xf9e2af))
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_color(rgb(0x1e1e2e))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(SharedString::from(msg)),
                    )
                    .child(
                        div()
                            .text_color(rgb(0x1e1e2e))
                            .text_sm()
                            .child("Enter: confirm | Esc: cancel"),
                    ),
            );
        }

        root = root
            // Body: sidebar + content
            .child({
                let weak_sidebar = weak.clone();
                let detail_visible = self.detail_visible;

                let current_resource = self.current_resource.clone();
                let mut body = div().flex().flex_1().overflow_hidden().child(
                    build_sidebar(
                        &current_resource,
                        pf_count,
                        move |idx, _ev, _window, cx| {
                            weak_sidebar
                                .update(cx, |this, cx| {
                                    if let Some(entry) = RESOURCES.get(idx) {
                                        this.switch_resource(entry.api_name, cx);
                                        this.active_panel = FocusPanel::Table;
                                    }
                                    cx.notify();
                                })
                                .ok();
                        },
                    ),
                );

                if detail_visible {
                    // Detail panel
                    if self.detail_loading {
                        body = body.child(
                            div().flex_1().flex().items_center().justify_center().child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xl()
                                            .text_color(cx.theme().primary)
                                            .child(spinner_text.clone()),
                                    )
                                    .child(
                                        div().text_color(cx.theme().muted_foreground).child("Loading details..."),
                                    ),
                            ),
                        );
                    } else if let Some(detail) = &self.detail_data {
                        let panel = DetailPanel::new(
                            detail,
                            self.detail_tab,
                            self.detail_logs.as_deref(),
                            self.detail_logs_loading,
                            SPINNER_FRAMES[self.spinner_frame % SPINNER_FRAMES.len()],
                            self.can_restart(),
                            self.can_port_forward(),
                            self.yaml_editor.clone(),
                            PanelColors::from_theme(cx),
                        );
                        let weak_detail = weak.clone();
                        let weak_restart = weak.clone();
                        let weak_apply = weak.clone();
                        let weak_pf = weak.clone();
                        let weak_pod = weak.clone();
                        body = body.child(div().flex_1().child(
                            panel.into_element_with_clicks(
                                move |tab, _window, cx| {
                                    weak_detail
                                        .update(cx, |this, cx| {
                                            this.switch_detail_tab(tab, cx);
                                            cx.notify();
                                        })
                                        .ok();
                                },
                                move |_ev, _window, cx| {
                                    weak_restart
                                        .update(cx, |this, cx| {
                                            // Same confirmation as keyboard `r`
                                            if this.pending_confirm.is_none() {
                                                let target = this.detail_data.as_ref().map(|d| {
                                                    (d.name.clone(), d.resource_type.clone())
                                                });
                                                if let Some((name, resource_type)) = target {
                                                    if matches!(
                                                        resource_type.as_str(),
                                                        "pods"
                                                            | "deployments"
                                                            | "statefulsets"
                                                            | "daemonsets"
                                                    ) {
                                                        this.pending_confirm =
                                                            Some(PendingConfirmation::Restart {
                                                                name,
                                                                resource_type,
                                                            });
                                                    }
                                                }
                                            }
                                            cx.notify();
                                        })
                                        .ok();
                                },
                                // Apply YAML button
                                move |_ev, _window, cx| {
                                    weak_apply
                                        .update(cx, |this, cx| {
                                            if this.pending_confirm.is_none() {
                                                this.pending_confirm =
                                                    Some(PendingConfirmation::ApplyYaml);
                                            }
                                            cx.notify();
                                        })
                                        .ok();
                                },
                                // Port Forward button
                                move |_ev, _window, cx| {
                                    weak_pf
                                        .update(cx, |this, cx| {
                                            this.open_port_forward_dialog(cx);
                                            cx.notify();
                                        })
                                        .ok();
                                },
                                move |pod_name, _ev, _window, cx| {
                                    weak_pod
                                        .update(cx, |this, cx| {
                                            this.open_pod_detail_by_name(&pod_name, cx);
                                            cx.notify();
                                        })
                                        .ok();
                                },
                            ),
                        ));
                    }
                } else if loading {
                    body = body.child(
                        div()
                            .id("table-scroll")
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xl()
                                            .text_color(cx.theme().primary)
                                            .child(spinner_text.clone()),
                                    )
                                    .child(div().text_color(cx.theme().muted_foreground).child(
                                        SharedString::from(format!(
                                            "Loading {}...",
                                            loading_resource
                                        )),
                                    )),
                            ),
                    );
                } else {
                    body = body.child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(Component::new(Table::new(&self.table_state).stripe(true))),
                    );
                }

                body
            })
            // Status bar
            .child(status.into_element(cx));

        // Check port-forward health
        self.check_port_forward_health();

        // Resource picker overlay
        if self.res_picker_visible {
            let weak_res_pick = cx.weak_entity();
            let picker = ResourcePicker::new(
                &self.filtered_resources(),
                self.res_picker_selected,
                &self.res_picker_filter,
                &self.current_resource,
                PanelColors::from_theme(cx),
            );
            root = root.child(picker.into_element(move |idx, _ev, _window, cx| {
                weak_res_pick
                    .update(cx, |this, cx| {
                        this.res_picker_selected = idx;
                        this.select_resource(cx);
                        cx.notify();
                    })
                    .ok();
            }));
        }

        // Context picker overlay
        if self.ctx_picker_visible {
            let weak_ctx_pick = cx.weak_entity();
            let picker = ContextPicker::new(
                &self.filtered_contexts(),
                self.ctx_picker_selected,
                &self.ctx_picker_filter,
                &self.current_context,
                self.ctx_picker_loading,
                &spinner_text,
                PanelColors::from_theme(cx),
            );
            root = root.child(picker.into_element(move |idx, _ev, _window, cx| {
                weak_ctx_pick
                    .update(cx, |this, cx| {
                        this.ctx_picker_selected = idx;
                        this.select_context(cx);
                        cx.notify();
                    })
                    .ok();
            }));
        }

        // Namespace picker overlay
        if self.ns_picker_visible {
            let picker = NamespacePicker::new(
                &self.filtered_namespaces(),
                self.ns_picker_selected,
                &self.ns_picker_filter,
                &self.current_namespace,
                self.ns_picker_loading,
                &spinner_text,
                PanelColors::from_theme(cx),
            );
            let weak_ns_pick = cx.weak_entity();
            root = root.child(picker.into_element(move |idx, _ev, _window, cx| {
                weak_ns_pick.update(cx, |this, cx| {
                    this.ns_picker_selected = idx;
                    this.select_namespace(cx);
                    cx.notify();
                }).ok();
            }));
        }

        // Port forward dialog overlay
        if self.pf_dialog_visible {
            let dialog = PortForwardDialog::new(
                &self.pf_dialog_pod_name,
                &self.pf_dialog_ports,
                self.pf_dialog_selected,
                &self.pf_dialog_local_port,
                self.pf_dialog_loading,
                &spinner_text,
                PanelColors::from_theme(cx),
            );
            root = root.child(dialog.into_element());
        }

        // Port forward list overlay
        if self.pf_list_visible {
            let weak_pf_stop = cx.weak_entity();
            let list = PortForwardList::new(&self.port_forwards, self.pf_list_selected, PanelColors::from_theme(cx));
            root = root.child(list.into_element(move |id, _ev, _window, cx| {
                weak_pf_stop
                    .update(cx, |this, cx| {
                        let desc = this
                            .port_forwards
                            .iter()
                            .find(|e| e.id == id)
                            .map(|e| {
                                format!(
                                    "{}:{} -> {}",
                                    e.pod_name, e.remote_port, e.local_port
                                )
                            })
                            .unwrap_or_default();
                        this.pending_confirm = Some(PendingConfirmation::StopPortForward {
                            id,
                            description: desc,
                        });
                        cx.notify();
                    })
                    .ok();
            }));
        }

        root
    }
}
