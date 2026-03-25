use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{IconName, Sizable};

use crate::model::detail::{DetailTab, ResourceDetail};
use crate::ui::theme::PanelColors;

/// Detail panel showing resource information in tabs
pub struct DetailPanel {
    detail: ResourceDetail,
    active_tab: DetailTab,
    logs: Option<String>,
    logs_loading: bool,
    spinner: String,
    can_restart: bool,
    can_port_forward: bool,
    yaml_editor: Option<Entity<InputState>>,
    colors: PanelColors,
}

impl DetailPanel {
    pub fn new(
        detail: &ResourceDetail,
        active_tab: DetailTab,
        logs: Option<&str>,
        logs_loading: bool,
        spinner: &str,
        can_restart: bool,
        can_port_forward: bool,
        yaml_editor: Option<Entity<InputState>>,
        colors: PanelColors,
    ) -> Self {
        Self {
            detail: detail.clone(),
            active_tab,
            logs: logs.map(|s| s.to_string()),
            logs_loading,
            spinner: spinner.to_string(),
            can_restart,
            can_port_forward,
            yaml_editor,
            colors,
        }
    }

    pub fn into_element_with_clicks(
        self,
        on_tab_click: impl Fn(DetailTab, &mut Window, &mut App) + 'static,
        on_restart: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_apply_yaml: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_port_forward: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_pod_click: impl Fn(String, &MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Div {
        let on_pod_click = std::rc::Rc::new(on_pod_click);
        let is_yaml_editor = self.active_tab == DetailTab::Yaml && self.yaml_editor.is_some();
        let active_tab_index = DetailTab::all()
            .iter()
            .position(|t| *t == self.active_tab)
            .unwrap_or(0);

        // Build the tab bar using gpui-component TabBar
        let log_supported = matches!(
            self.detail.resource_type.as_str(),
            "pods" | "deployments" | "statefulsets" | "daemonsets" | "replicasets" | "jobs"
        );
        let is_logs_disabled = !log_supported;
        let tabs = DetailTab::all();

        let mut tab_bar_widget = TabBar::new("detail-tabs")
            .selected_index(active_tab_index)
            .on_click(move |index, window, cx| {
                let tab = tabs[*index];
                if tab == DetailTab::Logs && is_logs_disabled {
                    return;
                }
                on_tab_click(tab, window, cx);
            });

        for tab in DetailTab::all() {
            let icon = match tab {
                DetailTab::Overview => IconName::LayoutDashboard,
                DetailTab::Yaml => IconName::File,
                DetailTab::Events => IconName::Bell,
                DetailTab::Logs => IconName::SquareTerminal,
            };
            let label = format!("{} {}", tab.key_hint(), tab.label());
            tab_bar_widget = tab_bar_widget.child(Tab::new().label(label).prefix(icon));
        }

        // Wrap tab bar + restart button + resource name in a row
        let mut top_bar = div()
            .flex()
            .w_full()
            .items_center()
            .bg(self.colors.muted)
            .child(Component::new(tab_bar_widget));

        // Right side: restart button + resource name
        let mut right = div().flex().items_center().gap_2().pr_2();

        if self.active_tab == DetailTab::Yaml && self.yaml_editor.is_some() {
            right = right.child(Component::new(
                Button::new("apply-yaml-btn")
                    .label("Apply")
                    .icon(IconName::Check)
                    .small()
                    .compact()
                    .on_click(move |ev, window, cx| {
                        on_apply_yaml(ev, window, cx);
                    }),
            ));
        }

        if self.can_port_forward {
            right = right.child(Component::new(
                Button::new("pf-btn")
                    .label("Port Forward")
                    .icon(IconName::ArrowRight)
                    .small()
                    .compact()
                    .on_click(move |ev, window, cx| {
                        on_port_forward(ev, window, cx);
                    }),
            ));
        }

        if self.can_restart {
            right = right.child(Component::new(
                Button::new("restart-btn")
                    .danger()
                    .label("Restart")
                    .icon(IconName::Redo)
                    .small()
                    .compact()
                    .on_click(move |ev, window, cx| {
                        on_restart(ev, window, cx);
                    }),
            ));
        }

        right = right.child(
            div()
                .text_color(self.colors.warning)
                .text_sm()
                .child(SharedString::from(self.detail.name.clone())),
        );

        top_bar = top_bar.child(right);

        // Tab content
        let tab_content = match self.active_tab {
            DetailTab::Overview => self.render_overview(Some(on_pod_click)),
            DetailTab::Yaml => self.render_yaml(),
            DetailTab::Events => self.render_events(),
            DetailTab::Logs => self.render_logs(),
        };

        let mut root = div().flex().flex_col().w_full().h_full().child(top_bar);

        if is_yaml_editor {
            root = root.child(div().flex_1().overflow_hidden().child(tab_content));
        } else {
            root = root.child(
                div()
                    .id("detail-content-scroll")
                    .flex_1()
                    .overflow_scroll()
                    .child(tab_content),
            );
        }

        root
    }

    fn render_overview(
        &self,
        on_pod_click: Option<std::rc::Rc<dyn Fn(String, &MouseDownEvent, &mut Window, &mut App)>>,
    ) -> Div {
        let mut content = div().flex().flex_col().w_full().p_3().gap_2();

        // Status header
        let phase_color = match self.detail.phase.to_lowercase().as_str() {
            s if s.contains("running") || s.contains("ready") || s.contains("active") => {
                self.colors.success
            }
            s if s.contains("pending") || s.contains("waiting") => self.colors.warning,
            s if s.contains("failed")
                || s.contains("error")
                || s.contains("terminated")
                || s.contains("notready") =>
            {
                self.colors.danger
            }
            _ => self.colors.foreground,
        };

        content =
            content.child(
                div()
                    .flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .text_color(phase_color)
                            .text_lg()
                            .child(SharedString::from(self.detail.phase.clone())),
                    )
                    .child(div().text_color(self.colors.muted_foreground).child(
                        SharedString::from(format!(
                            "{} · Age: {}",
                            self.detail.resource_type, self.detail.age
                        )),
                    )),
            );

        // Metadata section
        content = content.child(self.render_section("Metadata", {
            let mut d = div().flex().flex_col().gap_1();
            d = d.child(render_kv("Name", &self.detail.name, &self.colors));
            if let Some(ns) = &self.detail.namespace {
                d = d.child(render_kv("Namespace", ns, &self.colors));
            }
            d = d.child(render_kv("Age", &self.detail.age, &self.colors));
            if !self.detail.owner_references.is_empty() {
                let owners = self
                    .detail
                    .owner_references
                    .iter()
                    .map(|o| format!("{}/{}", o.kind, o.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                d = d.child(render_kv("Owner", &owners, &self.colors));
            }
            d
        }));

        // Labels
        if !self.detail.labels.is_empty() {
            let mut d = div().flex().flex_wrap().gap_1();
            for (k, v) in &self.detail.labels {
                d = d.child(render_tag(k, v, &self.colors));
            }
            content = content.child(self.render_section("Labels", d));
        }

        // Annotations
        if !self.detail.annotations.is_empty() {
            let mut d = div().flex().flex_col().gap_1();
            for (k, v) in &self.detail.annotations {
                d = d.child(render_kv(k, v, &self.colors));
            }
            content = content.child(self.render_section("Annotations", d));
        }

        // Conditions
        if !self.detail.conditions.is_empty() {
            let mut d = div().flex().flex_col().gap_1();
            d = d.child(
                div()
                    .flex()
                    .gap_2()
                    .text_color(self.colors.primary)
                    .text_xs()
                    .child(div().w(px(140.0)).child("TYPE"))
                    .child(div().w(px(60.0)).child("STATUS"))
                    .child(div().w(px(100.0)).child("AGE"))
                    .child(div().flex_1().child("MESSAGE")),
            );
            for cond in &self.detail.conditions {
                let status_color = if cond.status == "True" {
                    self.colors.success
                } else {
                    self.colors.danger
                };
                d = d.child(
                    div()
                        .flex()
                        .gap_2()
                        .text_sm()
                        .child(
                            div()
                                .w(px(140.0))
                                .text_color(self.colors.foreground)
                                .child(SharedString::from(cond.type_.clone())),
                        )
                        .child(
                            div()
                                .w(px(60.0))
                                .text_color(status_color)
                                .child(SharedString::from(cond.status.clone())),
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(self.colors.muted_foreground)
                                .child(SharedString::from(cond.last_transition.clone())),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_color(self.colors.secondary_foreground)
                                .overflow_x_hidden()
                                .child(SharedString::from(cond.message.clone())),
                        ),
                );
            }
            content = content.child(self.render_section("Conditions", d));
        }

        // Containers
        if !self.detail.containers.is_empty() {
            let mut d = div().flex().flex_col().gap_1();
            for c in &self.detail.containers {
                let state_color = if c.state.contains("Running") {
                    self.colors.success
                } else if c.state.contains("Waiting") {
                    self.colors.warning
                } else if c.state.contains("Terminated") {
                    self.colors.danger
                } else {
                    self.colors.muted_foreground
                };

                let mut container_div = div()
                    .flex()
                    .flex_col()
                    .p_2()
                    .mb_1()
                    .bg(self.colors.list_even)
                    .rounded_md()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(self.colors.primary)
                                    .child(SharedString::from(c.name.clone())),
                            )
                            .child(
                                div()
                                    .text_color(state_color)
                                    .text_sm()
                                    .child(SharedString::from(c.state.clone())),
                            ),
                    )
                    .child({
                        let image_text = c.image.clone();
                        div()
                            .flex()
                            .gap_4()
                            .text_sm()
                            .text_color(self.colors.muted_foreground)
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .items_center()
                                    .child("Image:")
                                    .child(
                                        div()
                                            .text_color(self.colors.secondary_foreground)
                                            .child(SharedString::from(c.image.clone())),
                                    )
                                    .child(Component::new(
                                        Button::new("copy-image")
                                            .ghost()
                                            .compact()
                                            .small()
                                            .icon(IconName::Copy)
                                            .on_click(move |_ev, _window, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    image_text.clone(),
                                                ));
                                            }),
                                    )),
                            )
                            .child(
                                div().flex().gap_1().child("Ready:").child(
                                    div()
                                        .text_color(if c.ready {
                                            self.colors.success
                                        } else {
                                            self.colors.danger
                                        })
                                        .child(if c.ready { "Yes" } else { "No" }),
                                ),
                            )
                            .child(
                                div().flex().gap_1().child("Restarts:").child(
                                    div()
                                        .text_color(self.colors.secondary_foreground)
                                        .child(SharedString::from(c.restart_count.to_string())),
                                ),
                            )
                    });

                if !c.ports.is_empty() {
                    container_div = container_div.child(
                        div()
                            .flex()
                            .gap_1()
                            .text_sm()
                            .text_color(self.colors.muted_foreground)
                            .child("Ports:")
                            .child(
                                div()
                                    .text_color(self.colors.secondary_foreground)
                                    .child(SharedString::from(c.ports.clone())),
                            ),
                    );
                }

                d = d.child(container_div);
            }
            content = content.child(self.render_section("Containers", d));
        }

        // Pods (for workloads)
        if !self.detail.pods.is_empty() {
            content = content.child(self.render_section(
                &format!("Pods ({})", self.detail.pods.len()),
                self.render_pods_table(on_pod_click),
            ));
        }

        content
    }

    fn render_pods_table(
        &self,
        on_pod_click: Option<std::rc::Rc<dyn Fn(String, &MouseDownEvent, &mut Window, &mut App)>>,
    ) -> Div {
        let col_widths: &[(&str, f32)] = &[
            ("NAME", 220.0),
            ("READY", 55.0),
            ("STATUS", 80.0),
            ("CPU", 50.0),
            ("MEM", 50.0),
            ("RESTARTS", 65.0),
            ("LAST RESTART", 90.0),
            ("REASON", 100.0),
            ("NODE", 130.0),
            ("IP", 110.0),
            ("AGE", 50.0),
        ];

        let mut table = div().flex().flex_col().gap_0();

        // Header row
        let mut header = div()
            .flex()
            .gap_1()
            .py_1()
            .text_xs()
            .text_color(self.colors.primary);
        for (name, w) in col_widths {
            header = header.child(div().w(px(*w)).child(*name));
        }
        table = table.child(header);

        // Data rows
        for (i, pod) in self.detail.pods.iter().enumerate() {
            let bg = if i % 2 == 0 {
                self.colors.background
            } else {
                self.colors.list_even
            };

            let status_color: Hsla = match pod.status.to_lowercase().as_str() {
                "running" => self.colors.success,
                "pending" => self.colors.warning,
                s if s.contains("error") || s.contains("fail") || s.contains("crash") => {
                    self.colors.danger
                }
                "succeeded" => self.colors.primary,
                _ => self.colors.foreground,
            };

            let cells: Vec<(SharedString, Hsla)> = vec![
                (pod.name.clone().into(), self.colors.foreground),
                (pod.ready.clone().into(), self.colors.secondary_foreground),
                (pod.status.clone().into(), status_color),
                (pod.cpu.clone().into(), self.colors.secondary_foreground),
                (pod.memory.clone().into(), self.colors.secondary_foreground),
                (
                    pod.restarts.to_string().into(),
                    if pod.restarts > 0 {
                        self.colors.warning
                    } else {
                        self.colors.secondary_foreground
                    },
                ),
                (
                    pod.last_restart_time.clone().into(),
                    self.colors.muted_foreground,
                ),
                (
                    pod.last_restart_reason.clone().into(),
                    self.colors.muted_foreground,
                ),
                (pod.node.clone().into(), self.colors.muted_foreground),
                (pod.ip.clone().into(), self.colors.muted_foreground),
                (pod.age.clone().into(), self.colors.muted_foreground),
            ];

            let mut row = div().flex().gap_1().py_px().bg(bg).text_sm();

            for (j, (text, color)) in cells.into_iter().enumerate() {
                let w = col_widths[j].1;
                if j == 0 {
                    // NAME column — clickable link
                    let link_hover = self.colors.link_hover;
                    let mut name_cell = div()
                        .w(px(w))
                        .text_color(self.colors.link)
                        .overflow_x_hidden()
                        .cursor_pointer()
                        .hover(move |s| s.text_color(link_hover));

                    if let Some(cb) = &on_pod_click {
                        let cb = cb.clone();
                        let pod_name = pod.name.clone();
                        name_cell =
                            name_cell.on_mouse_down(MouseButton::Left, move |ev, window, cx| {
                                cb(pod_name.clone(), ev, window, cx);
                            });
                    }

                    row = row.child(name_cell.child(text));
                } else {
                    row = row.child(
                        div()
                            .w(px(w))
                            .text_color(color)
                            .overflow_x_hidden()
                            .child(text),
                    );
                }
            }

            table = table.child(row);
        }

        table
    }

    fn render_yaml(&self) -> Div {
        if self.detail.yaml.is_empty() {
            return div()
                .p_4()
                .text_color(self.colors.muted_foreground)
                .child("No YAML available");
        }

        if let Some(editor) = &self.yaml_editor {
            // Use the code editor component
            div()
                .flex()
                .flex_col()
                .size_full()
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .bg(self.colors.muted)
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .text_color(self.colors.muted_foreground)
                        .child("Edit YAML · Ctrl+S to apply"),
                )
                .child(
                    div()
                        .flex_1()
                        .p_1()
                        .font_family("Monaco")
                        .text_base()
                        .child(Component::new(
                            Input::new(editor)
                                .bg(self.colors.muted)
                                .h_full()
                                .appearance(false),
                        )),
                )
        } else {
            // Fallback: read-only YAML display
            let mut content = div().flex().flex_col().p_3().font_family("Monaco");
            for line in self.detail.yaml.lines() {
                content = content.child(
                    div()
                        .text_color(self.colors.foreground)
                        .text_sm()
                        .child(SharedString::from(line.to_string())),
                );
            }
            content
        }
    }

    fn render_events(&self) -> Div {
        if self.detail.events.is_empty() {
            return div()
                .p_4()
                .text_color(self.colors.muted_foreground)
                .child("No events");
        }

        let mut content = div().flex().flex_col().p_3();

        content = content.child(
            div()
                .flex()
                .gap_2()
                .py_1()
                .text_color(self.colors.primary)
                .text_xs()
                .child(div().w(px(70.0)).child("TYPE"))
                .child(div().w(px(120.0)).child("REASON"))
                .child(div().w(px(60.0)).child("AGE"))
                .child(div().w(px(140.0)).child("FROM"))
                .child(div().flex_1().child("MESSAGE")),
        );

        for (i, ev) in self.detail.events.iter().enumerate() {
            let type_color = if ev.type_ == "Warning" {
                self.colors.warning
            } else {
                self.colors.success
            };

            let bg = if i % 2 == 0 {
                self.colors.background
            } else {
                self.colors.list_even
            };

            content = content.child(
                div()
                    .flex()
                    .gap_2()
                    .py_px()
                    .bg(bg)
                    .text_sm()
                    .child(
                        div()
                            .w(px(70.0))
                            .text_color(type_color)
                            .child(SharedString::from(ev.type_.clone())),
                    )
                    .child(
                        div()
                            .w(px(120.0))
                            .text_color(self.colors.foreground)
                            .child(SharedString::from(ev.reason.clone())),
                    )
                    .child(
                        div()
                            .w(px(60.0))
                            .text_color(self.colors.muted_foreground)
                            .child(SharedString::from(ev.age.clone())),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .text_color(self.colors.muted_foreground)
                            .overflow_x_hidden()
                            .child(SharedString::from(ev.from.clone())),
                    )
                    .child(
                        copyable_value(
                            &format!("ev-msg-{i}"),
                            &ev.message,
                            self.colors.secondary_foreground,
                        ),
                    ),
            );
        }

        content
    }

    fn render_logs(&self) -> Div {
        let log_supported = matches!(
            self.detail.resource_type.as_str(),
            "pods" | "deployments" | "statefulsets" | "daemonsets" | "replicasets" | "jobs"
        );
        if !log_supported {
            return div()
                .p_4()
                .text_color(self.colors.muted_foreground)
                .child("Logs not available for this resource type");
        }

        if self.logs_loading {
            return div()
                .p_4()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_color(self.colors.primary)
                        .child(SharedString::from(self.spinner.clone())),
                )
                .child(
                    div()
                        .text_color(self.colors.muted_foreground)
                        .child("Loading logs..."),
                );
        }

        match &self.logs {
            None => div()
                .p_4()
                .text_color(self.colors.muted_foreground)
                .child("Switch to Logs tab (4) to load logs"),
            Some(logs) if logs.is_empty() => div()
                .p_4()
                .text_color(self.colors.muted_foreground)
                .child("No logs available"),
            Some(logs) => {
                let mut content = div().flex().flex_col().p_2().font_family("Monaco");
                for (i, line) in logs.lines().enumerate() {
                    content = content.child(
                        copyable_value(
                            &format!("log-{i}"),
                            line,
                            self.colors.foreground,
                        )
                        .text_sm(),
                    );
                }
                content
            }
        }
    }

    fn render_section(&self, title: &str, inner: Div) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_color(self.colors.primary)
                    .text_sm()
                    .pb_1()
                    .border_b_1()
                    .border_color(self.colors.border)
                    .child(SharedString::from(title.to_string())),
            )
            .child(inner)
    }
}

/// A value div that shows a copy icon on hover. Clicking copies to clipboard.
fn copyable_value(id: &str, value: &str, color: Hsla) -> Stateful<Div> {
    let val = value.to_string();
    let val_for_click = val.clone();
    let group_id = SharedString::from(format!("copy-{id}"));
    let group_id2 = group_id.clone();
    div()
        .id(SharedString::from(id.to_string()))
        .group(group_id)
        .flex_1()
        .flex()
        .items_center()
        .gap_1()
        .cursor_pointer()
        .child(
            div()
                .text_color(color)
                .overflow_x_hidden()
                .child(SharedString::from(val)),
        )
        .child(
            div()
                .text_color(color)
                .text_xs()
                .invisible()
                .group_hover(group_id2, |s| s.visible())
                .child("⎘"),
        )
        .on_click(move |_ev, _window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(val_for_click.clone()));
        })
}

fn render_kv(key: &str, value: &str, colors: &PanelColors) -> Div {
    let id = format!("kv-{}-{}", key, &value[..value.len().min(20)]);
    div()
        .flex()
        .gap_2()
        .text_sm()
        .child(
            div()
                .w(px(120.0))
                .text_color(colors.muted_foreground)
                .child(SharedString::from(key.to_string())),
        )
        .child(copyable_value(&id, value, colors.foreground))
}

fn render_tag(key: &str, value: &str, _colors: &PanelColors) -> AnyElement {
    Component::new(
        gpui_component::tag::Tag::info()
            .outline()
            .with_size(gpui_component::Size::Small)
            .child(SharedString::from(format!("{key}={value}"))),
    )
    .into_any_element()
}
