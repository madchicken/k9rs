use gpui::*;
use gpui_component::input::{Input, InputState};
use gpui_component::tab::{Tab, TabBar};

use crate::model::detail::{DetailTab, ResourceDetail};

/// Detail panel showing resource information in tabs
pub struct DetailPanel {
    detail: ResourceDetail,
    active_tab: DetailTab,
    logs: Option<String>,
    logs_loading: bool,
    spinner: String,
    can_restart: bool,
    yaml_editor: Option<Entity<InputState>>,
}

impl DetailPanel {
    pub fn new(
        detail: &ResourceDetail,
        active_tab: DetailTab,
        logs: Option<&str>,
        logs_loading: bool,
        spinner: &str,
        can_restart: bool,
        yaml_editor: Option<Entity<InputState>>,
    ) -> Self {
        Self {
            detail: detail.clone(),
            active_tab,
            logs: logs.map(|s| s.to_string()),
            logs_loading,
            spinner: spinner.to_string(),
            can_restart,
            yaml_editor,
        }
    }

    pub fn into_element_with_clicks(
        self,
        on_tab_click: impl Fn(DetailTab, &mut Window, &mut App) + 'static,
        on_restart: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
        on_pod_click: impl Fn(String, &MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Div {
        let on_pod_click = std::rc::Rc::new(on_pod_click);
        let is_yaml_editor = self.active_tab == DetailTab::Yaml && self.yaml_editor.is_some();
        let active_tab_index = DetailTab::all()
            .iter()
            .position(|t| *t == self.active_tab)
            .unwrap_or(0);

        // Build the tab bar using gpui-component TabBar
        let is_logs_disabled = self.detail.resource_type != "pods";
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
            let label = format!("{} {}", tab.key_hint(), tab.label());
            tab_bar_widget = tab_bar_widget.child(Tab::new().label(label));
        }

        // Wrap tab bar + restart button + resource name in a row
        let mut top_bar = div()
            .flex()
            .w_full()
            .items_center()
            .bg(rgb(0x313244))
            .child(Component::new(tab_bar_widget));

        // Right side: restart button + resource name
        let mut right = div().flex().items_center().gap_2().pr_2();

        if self.can_restart {
            right = right.child(
                div()
                    .px_2()
                    .py_px()
                    .bg(rgb(0xf38ba8))
                    .text_color(rgb(0x1e1e2e))
                    .text_xs()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0xeba0ac)))
                    .on_mouse_down(MouseButton::Left, move |ev, window, cx| {
                        on_restart(ev, window, cx);
                    })
                    .flex()
                    .gap_1()
                    .child("↻ Restart")
                    .child(div().text_color(rgba(0x1e1e2eaa)).child("(r)")),
            );
        }

        right = right.child(
            div()
                .text_color(rgb(0xf9e2af))
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
                    .overflow_y_scroll()
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
                rgb(0xa6e3a1)
            }
            s if s.contains("pending") || s.contains("waiting") => rgb(0xf9e2af),
            s if s.contains("failed")
                || s.contains("error")
                || s.contains("terminated")
                || s.contains("notready") =>
            {
                rgb(0xf38ba8)
            }
            _ => rgb(0xcdd6f4),
        };

        content = content.child(
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
                .child(
                    div()
                        .text_color(rgb(0x6c7086))
                        .child(SharedString::from(format!(
                            "{} · Age: {}",
                            self.detail.resource_type, self.detail.age
                        ))),
                ),
        );

        // Metadata section
        content = content.child(self.render_section("Metadata", {
            let mut d = div().flex().flex_col().gap_1();
            d = d.child(render_kv("Name", &self.detail.name));
            if let Some(ns) = &self.detail.namespace {
                d = d.child(render_kv("Namespace", ns));
            }
            d = d.child(render_kv("Age", &self.detail.age));
            if !self.detail.owner_references.is_empty() {
                let owners = self
                    .detail
                    .owner_references
                    .iter()
                    .map(|o| format!("{}/{}", o.kind, o.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                d = d.child(render_kv("Owner", &owners));
            }
            d
        }));

        // Labels
        if !self.detail.labels.is_empty() {
            let mut d = div().flex().flex_wrap().gap_1();
            for (k, v) in &self.detail.labels {
                d = d.child(render_tag(k, v));
            }
            content = content.child(self.render_section("Labels", d));
        }

        // Annotations
        if !self.detail.annotations.is_empty() {
            let mut d = div().flex().flex_col().gap_1();
            for (k, v) in &self.detail.annotations {
                d = d.child(render_kv(k, v));
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
                    .text_color(rgb(0x89b4fa))
                    .text_xs()
                    .child(div().w(px(140.0)).child("TYPE"))
                    .child(div().w(px(60.0)).child("STATUS"))
                    .child(div().w(px(100.0)).child("AGE"))
                    .child(div().flex_1().child("MESSAGE")),
            );
            for cond in &self.detail.conditions {
                let status_color = if cond.status == "True" {
                    rgb(0xa6e3a1)
                } else {
                    rgb(0xf38ba8)
                };
                d = d.child(
                    div()
                        .flex()
                        .gap_2()
                        .text_sm()
                        .child(
                            div()
                                .w(px(140.0))
                                .text_color(rgb(0xcdd6f4))
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
                                .text_color(rgb(0x6c7086))
                                .child(SharedString::from(cond.last_transition.clone())),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_color(rgb(0xbac2de))
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
                    rgb(0xa6e3a1)
                } else if c.state.contains("Waiting") {
                    rgb(0xf9e2af)
                } else if c.state.contains("Terminated") {
                    rgb(0xf38ba8)
                } else {
                    rgb(0x6c7086)
                };

                let mut container_div = div()
                    .flex()
                    .flex_col()
                    .p_2()
                    .mb_1()
                    .bg(rgb(0x24243a))
                    .rounded_md()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(rgb(0x89b4fa))
                                    .child(SharedString::from(c.name.clone())),
                            )
                            .child(
                                div()
                                    .text_color(state_color)
                                    .text_sm()
                                    .child(SharedString::from(c.state.clone())),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .text_sm()
                            .text_color(rgb(0x6c7086))
                            .child(
                                div().flex().gap_1().child("Image:").child(
                                    div()
                                        .text_color(rgb(0xbac2de))
                                        .child(SharedString::from(c.image.clone())),
                                ),
                            )
                            .child(
                                div().flex().gap_1().child("Ready:").child(
                                    div()
                                        .text_color(if c.ready {
                                            rgb(0xa6e3a1)
                                        } else {
                                            rgb(0xf38ba8)
                                        })
                                        .child(if c.ready { "Yes" } else { "No" }),
                                ),
                            )
                            .child(
                                div().flex().gap_1().child("Restarts:").child(
                                    div()
                                        .text_color(rgb(0xbac2de))
                                        .child(SharedString::from(c.restart_count.to_string())),
                                ),
                            ),
                    );

                if !c.ports.is_empty() {
                    container_div = container_div.child(
                        div()
                            .flex()
                            .gap_1()
                            .text_sm()
                            .text_color(rgb(0x6c7086))
                            .child("Ports:")
                            .child(
                                div()
                                    .text_color(rgb(0xbac2de))
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
            .text_color(rgb(0x89b4fa));
        for (name, w) in col_widths {
            header = header.child(div().w(px(*w)).child(*name));
        }
        table = table.child(header);

        // Data rows
        for (i, pod) in self.detail.pods.iter().enumerate() {
            let bg = if i % 2 == 0 {
                rgb(0x1e1e2e)
            } else {
                rgb(0x24243a)
            };

            let status_color: Rgba = match pod.status.to_lowercase().as_str() {
                "running" => rgb(0xa6e3a1),
                "pending" => rgb(0xf9e2af),
                s if s.contains("error") || s.contains("fail") || s.contains("crash") => {
                    rgb(0xf38ba8)
                }
                "succeeded" => rgb(0x89b4fa),
                _ => rgb(0xcdd6f4),
            };

            let cells: Vec<(SharedString, Rgba)> = vec![
                (pod.name.clone().into(), rgb(0xcdd6f4)),
                (pod.ready.clone().into(), rgb(0xbac2de)),
                (pod.status.clone().into(), status_color),
                (pod.cpu.clone().into(), rgb(0xbac2de)),
                (pod.memory.clone().into(), rgb(0xbac2de)),
                (
                    pod.restarts.to_string().into(),
                    if pod.restarts > 0 {
                        rgb(0xf9e2af)
                    } else {
                        rgb(0xbac2de)
                    },
                ),
                (pod.last_restart_time.clone().into(), rgb(0x6c7086)),
                (pod.last_restart_reason.clone().into(), rgb(0x6c7086)),
                (pod.node.clone().into(), rgb(0x6c7086)),
                (pod.ip.clone().into(), rgb(0x6c7086)),
                (pod.age.clone().into(), rgb(0x6c7086)),
            ];

            let mut row = div().flex().gap_1().py_px().bg(bg).text_sm();

            for (j, (text, color)) in cells.into_iter().enumerate() {
                let w = col_widths[j].1;
                if j == 0 {
                    // NAME column — clickable link
                    let mut name_cell = div()
                        .w(px(w))
                        .text_color(rgb(0x89b4fa))
                        .overflow_x_hidden()
                        .cursor_pointer()
                        .hover(|s| s.text_color(rgb(0xb4d0fb)));

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
                .text_color(rgb(0x6c7086))
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
                        .bg(rgb(0x313244))
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .text_color(rgb(0x6c7086))
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
                                .bg(rgb(0x313244))
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
                        .text_color(rgb(0xcdd6f4))
                        .text_sm()
                        .child(SharedString::from(line.to_string())),
                );
            }
            content
        }
    }

    fn render_events(&self) -> Div {
        if self.detail.events.is_empty() {
            return div().p_4().text_color(rgb(0x6c7086)).child("No events");
        }

        let mut content = div().flex().flex_col().p_3();

        content = content.child(
            div()
                .flex()
                .gap_2()
                .py_1()
                .text_color(rgb(0x89b4fa))
                .text_xs()
                .child(div().w(px(70.0)).child("TYPE"))
                .child(div().w(px(120.0)).child("REASON"))
                .child(div().w(px(60.0)).child("AGE"))
                .child(div().w(px(140.0)).child("FROM"))
                .child(div().flex_1().child("MESSAGE")),
        );

        for (i, ev) in self.detail.events.iter().enumerate() {
            let type_color = if ev.type_ == "Warning" {
                rgb(0xf9e2af)
            } else {
                rgb(0xa6e3a1)
            };

            let bg = if i % 2 == 0 {
                rgb(0x1e1e2e)
            } else {
                rgb(0x24243a)
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
                            .text_color(rgb(0xcdd6f4))
                            .child(SharedString::from(ev.reason.clone())),
                    )
                    .child(
                        div()
                            .w(px(60.0))
                            .text_color(rgb(0x6c7086))
                            .child(SharedString::from(ev.age.clone())),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .text_color(rgb(0x6c7086))
                            .overflow_x_hidden()
                            .child(SharedString::from(ev.from.clone())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(rgb(0xbac2de))
                            .overflow_x_hidden()
                            .child(SharedString::from(ev.message.clone())),
                    ),
            );
        }

        content
    }

    fn render_logs(&self) -> Div {
        if self.detail.resource_type != "pods" {
            return div()
                .p_4()
                .text_color(rgb(0x6c7086))
                .child("Logs are only available for pods");
        }

        if self.logs_loading {
            return div()
                .p_4()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_color(rgb(0x89b4fa))
                        .child(SharedString::from(self.spinner.clone())),
                )
                .child(div().text_color(rgb(0x6c7086)).child("Loading logs..."));
        }

        match &self.logs {
            None => div()
                .p_4()
                .text_color(rgb(0x6c7086))
                .child("Switch to Logs tab (4) to load logs"),
            Some(logs) if logs.is_empty() => div()
                .p_4()
                .text_color(rgb(0x6c7086))
                .child("No logs available"),
            Some(logs) => {
                let mut content = div().flex().flex_col().p_2().font_family("Monaco");
                for line in logs.lines() {
                    content = content.child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xcdd6f4))
                            .child(SharedString::from(line.to_string())),
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
                    .text_color(rgb(0x89b4fa))
                    .text_sm()
                    .pb_1()
                    .border_b_1()
                    .border_color(rgb(0x45475a))
                    .child(SharedString::from(title.to_string())),
            )
            .child(inner)
    }
}

fn render_kv(key: &str, value: &str) -> Div {
    div()
        .flex()
        .gap_2()
        .text_sm()
        .child(
            div()
                .w(px(120.0))
                .text_color(rgb(0x6c7086))
                .child(SharedString::from(key.to_string())),
        )
        .child(
            div()
                .flex_1()
                .text_color(rgb(0xcdd6f4))
                .child(SharedString::from(value.to_string())),
        )
}

fn render_tag(key: &str, value: &str) -> Div {
    div()
        .flex()
        .text_sm()
        .px_2()
        .py_px()
        .mb_1()
        .bg(rgb(0x313244))
        .rounded_sm()
        .child(
            div()
                .text_color(rgb(0x89b4fa))
                .child(SharedString::from(format!("{key}="))),
        )
        .child(
            div()
                .text_color(rgb(0xa6e3a1))
                .child(SharedString::from(value.to_string())),
        )
}
