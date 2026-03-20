use gpui::*;

use crate::k8s::{K8sClient, runtime::spawn_on_tokio};
use crate::model::resources::{RESOURCES, resource_index};
use crate::model::table::TableData;
use crate::ui::header::Header;
use crate::ui::namespace_picker::NamespacePicker;
use crate::ui::resource_table::ResourceTable;
use crate::ui::sidebar::Sidebar;
use crate::ui::status_bar::StatusBar;

actions!(
    app,
    [
        MoveUp,
        MoveDown,
        Enter,
        GoBack,
        ActivateCommand,
        ActivateFilter,
        ToggleNamespacePicker,
        ToggleSidebar,
        Backspace,
    ]
);

/// Which panel has focus
#[derive(Clone, Copy, PartialEq)]
enum FocusPanel {
    Sidebar,
    Table,
}

/// Main application view — the root of the UI tree
pub struct AppView {
    focus_handle: FocusHandle,
    active_panel: FocusPanel,
    current_resource: String,
    current_namespace: String,
    current_context: String,
    table_data: TableData,
    selected_row: usize,
    sidebar_selected: usize,
    command_input: String,
    command_mode: bool,
    status_message: String,
    loading: bool,
    spinner_frame: usize,
    filter_mode: bool,
    filter_text: String,
    ns_picker_visible: bool,
    ns_picker_loading: bool,
    ns_picker_list: Vec<String>,
    ns_picker_selected: usize,
    ns_picker_filter: String,
}

impl Focusable for AppView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl AppView {
    pub fn new(
        cx: &mut Context<Self>,
        _window: &mut Window,
        namespace: &str,
        context: Option<&str>,
        resource: &str,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let sidebar_selected = resource_index(resource).unwrap_or(0);

        let mut view = Self {
            focus_handle,
            active_panel: FocusPanel::Table,
            current_resource: resource.to_string(),
            current_namespace: namespace.to_string(),
            current_context: "unknown".to_string(),
            table_data: TableData::empty(),
            selected_row: 0,
            sidebar_selected,
            command_input: String::new(),
            command_mode: false,
            status_message: "Connecting to cluster...".to_string(),
            loading: false,
            spinner_frame: 0,
            filter_mode: false,
            filter_text: String::new(),
            ns_picker_visible: false,
            ns_picker_loading: false,
            ns_picker_list: vec![],
            ns_picker_selected: 0,
            ns_picker_filter: String::new(),
        };

        if let Some(ctx) = context {
            view.current_context = ctx.to_string();
            view.status_message = "Connected".to_string();
        } else {
            view.detect_context();
        }

        view.load_resource_data(cx);
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

    fn switch_resource(&mut self, api_name: &str, cx: &mut Context<Self>) {
        self.current_resource = api_name.to_string();
        self.sidebar_selected = resource_index(api_name).unwrap_or(self.sidebar_selected);
        self.filter_text.clear();
        self.filter_mode = false;
        self.selected_row = 0;
        self.load_resource_data(cx);
    }

    fn load_resource_data(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.spinner_frame = 0;
        let resource = self.current_resource.clone();
        let namespace = self.current_namespace.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = spawn_on_tokio(async move {
                K8sClient::list_resources(&resource, &namespace).await
            })
            .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.loading = false;
                    match result {
                        Ok(data) => {
                            this.table_data = data;
                            this.selected_row = 0;
                            this.status_message = "Connected".to_string();
                        }
                        Err(e) => {
                            this.status_message = format!("Error: {e}");
                            this.table_data = TableData::empty();
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
            let result = spawn_on_tokio(async move {
                K8sClient::list_namespace_names().await
            })
            .await;

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
                    row.cells.iter().any(|cell| cell.to_lowercase().contains(&filter))
                })
                .collect()
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.ns_picker_visible {
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

    fn handle_command(&mut self, cx: &mut Context<Self>) {
        let cmd = self.command_input.trim().to_string();
        self.command_input.clear();
        self.command_mode = false;

        if cmd.is_empty() {
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
        // Animate spinner while loading
        if self.loading {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
            cx.on_next_frame(window, |this, _window, cx| {
                if this.loading {
                    cx.notify();
                }
            });
        }
        let header = Header::new(
            &self.current_context,
            &self.current_namespace,
            &self.current_resource,
        );

        let filtered_rows = self.filtered_rows();
        let filtered_table = TableData {
            columns: self.table_data.columns.clone(),
            rows: filtered_rows.iter().map(|(_, row)| (*row).clone()).collect(),
        };
        let table = ResourceTable::new(&filtered_table, self.selected_row);

        let sidebar = Sidebar::new(
            &self.current_resource,
            self.sidebar_selected,
            self.active_panel == FocusPanel::Sidebar,
        );

        let loading = self.loading;
        let spinner_text = SharedString::from(
            SPINNER_FRAMES[self.spinner_frame % SPINNER_FRAMES.len()],
        );
        let loading_resource = self.current_resource.clone();

        // WeakEntity for mouse click callbacks
        let weak = cx.weak_entity();

        let status = StatusBar::new(
            &self.status_message,
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
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .on_action(cx.listener(|this, _: &MoveUp, _window, cx| {
                this.move_selection(-1);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &MoveDown, _window, cx| {
                this.move_selection(1);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ActivateCommand, _window, cx| {
                this.command_mode = true;
                this.command_input.clear();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ActivateFilter, _window, cx| {
                this.filter_mode = true;
                this.filter_text.clear();
                this.selected_row = 0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &GoBack, _window, cx| {
                if this.ns_picker_visible {
                    this.ns_picker_visible = false;
                    this.ns_picker_filter.clear();
                } else if this.filter_mode {
                    this.filter_mode = false;
                    this.filter_text.clear();
                    this.selected_row = 0;
                } else if this.command_mode {
                    this.command_mode = false;
                    this.command_input.clear();
                } else if this.active_panel == FocusPanel::Sidebar {
                    this.active_panel = FocusPanel::Table;
                } else if this.current_resource == "namespaces" {
                    this.switch_resource("pods", cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Enter, _window, cx| {
                if this.ns_picker_visible {
                    this.select_namespace(cx);
                } else if this.filter_mode {
                    this.filter_mode = false;
                } else if this.command_mode {
                    this.handle_command(cx);
                } else if this.active_panel == FocusPanel::Sidebar {
                    // Select the resource from the sidebar
                    if let Some(entry) = RESOURCES.get(this.sidebar_selected) {
                        let api_name = entry.api_name.to_string();
                        this.active_panel = FocusPanel::Table;
                        this.switch_resource(&api_name, cx);
                    }
                } else if this.current_resource == "namespaces" {
                    if let Some(row) = this.table_data.rows.get(this.selected_row) {
                        if let Some(ns_name) = row.cells.first() {
                            this.current_namespace = ns_name.clone();
                            this.switch_resource("pods", cx);
                        }
                    }
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleNamespacePicker, _window, cx| {
                this.toggle_namespace_picker(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _window, cx| {
                this.active_panel = match this.active_panel {
                    FocusPanel::Sidebar => FocusPanel::Table,
                    FocusPanel::Table => FocusPanel::Sidebar,
                };
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Backspace, _window, cx| {
                if this.ns_picker_visible {
                    this.ns_picker_filter.pop();
                    this.ns_picker_selected = 0;
                } else if this.filter_mode {
                    this.filter_text.pop();
                    this.selected_row = 0;
                } else if this.command_mode {
                    this.command_input.pop();
                }
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                if this.ns_picker_visible {
                    if let Some(key_char) = &event.keystroke.key_char {
                        this.ns_picker_filter.push_str(key_char);
                        this.ns_picker_selected = 0;
                    }
                } else if this.filter_mode {
                    if let Some(key_char) = &event.keystroke.key_char {
                        this.filter_text.push_str(key_char);
                        this.selected_row = 0;
                    }
                } else if this.command_mode {
                    if let Some(key_char) = &event.keystroke.key_char {
                        this.command_input.push_str(key_char);
                    }
                }
                cx.notify();
            }))
            // Header
            .child(header.into_element())
            // Body: sidebar + table
            .child({
                let weak_sidebar = weak.clone();
                let weak_table = weak.clone();
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    // Sidebar with click handlers
                    .child(sidebar.into_element_with_clicks(
                        move |idx, _ev, _window, cx| {
                            weak_sidebar.update(cx, |this, cx| {
                                if let Some(entry) = RESOURCES.get(idx) {
                                    this.switch_resource(entry.api_name, cx);
                                    this.active_panel = FocusPanel::Table;
                                }
                                cx.notify();
                            }).ok();
                        },
                    ))
                    // Table area (with loading spinner or data)
                    .child(if loading {
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
                                            .text_color(rgb(0x89b4fa))
                                            .child(spinner_text.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0x6c7086))
                                            .child(SharedString::from(format!("Loading {}...", loading_resource))),
                                    ),
                            )
                    } else {
                        div()
                            .id("table-scroll")
                            .flex_1()
                            .overflow_y_scroll()
                            .child(table.into_element_with_clicks(
                                move |idx, _ev, _window, cx| {
                                    weak_table.update(cx, |this, cx| {
                                        this.selected_row = idx;
                                        this.active_panel = FocusPanel::Table;
                                        cx.notify();
                                    }).ok();
                                },
                            ))
                    })
            })
            // Status bar
            .child(status.into_element());

        // Namespace picker overlay
        if self.ns_picker_visible {
            let picker = NamespacePicker::new(
                &self.filtered_namespaces(),
                self.ns_picker_selected,
                &self.ns_picker_filter,
                &self.current_namespace,
                self.ns_picker_loading,
                &spinner_text,
            );
            root = root.child(picker.into_element());
        }

        root
    }
}
