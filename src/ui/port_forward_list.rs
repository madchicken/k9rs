use gpui::*;

use crate::model::port_forward::{PortForwardEntry, PortForwardStatus};

/// Modal overlay listing all active/stopped port forwards
pub struct PortForwardList {
    entries: Vec<PortForwardEntry>,
    selected: usize,
}

impl PortForwardList {
    pub fn new(entries: &[PortForwardEntry], selected: usize) -> Self {
        Self {
            entries: entries.to_vec(),
            selected,
        }
    }

    pub fn into_element(self) -> Div {
        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full()
            .bg(rgba(0x00000088))
            .flex()
            .justify_center()
            .pt_8()
            .on_mouse_down(MouseButton::Left, |_, _, _| {})
            .child(self.render_panel())
    }

    fn render_panel(self) -> Div {
        let mut panel = div()
            .w(px(600.0))
            .max_h(px(500.0))
            .bg(rgb(0x313244))
            .border_1()
            .border_color(rgb(0x585b70))
            .rounded_lg()
            .flex()
            .flex_col()
            .overflow_hidden()
            // Title
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(0x45475a))
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_color(rgb(0x89b4fa))
                            .child("Port Forwards"),
                    )
                    .child(
                        div()
                            .text_color(rgb(0x6c7086))
                            .text_sm()
                            .child(SharedString::from(format!(
                                "({} total)",
                                self.entries.len()
                            ))),
                    ),
            );

        if self.entries.is_empty() {
            panel = panel.child(
                div()
                    .px_3()
                    .py_4()
                    .text_color(rgb(0x6c7086))
                    .child("No port forwards. Select a pod and press 'f' to start one."),
            );
        } else {
            // Header
            panel = panel.child(
                div()
                    .flex()
                    .px_3()
                    .py_1()
                    .gap_2()
                    .text_xs()
                    .text_color(rgb(0x89b4fa))
                    .child(div().w(px(60.0)).child("LOCAL"))
                    .child(div().w(px(60.0)).child("REMOTE"))
                    .child(div().w(px(180.0)).child("POD"))
                    .child(div().w(px(100.0)).child("NAMESPACE"))
                    .child(div().w(px(70.0)).child("STATUS"))
                    .child(div().flex_1().child("AGE")),
            );

            // Entries
            let mut list = div().flex().flex_col();
            for (i, entry) in self.entries.iter().enumerate() {
                let is_selected = i == self.selected;
                let bg = if is_selected {
                    rgb(0x585b70)
                } else if i % 2 == 0 {
                    rgb(0x313244)
                } else {
                    rgb(0x2a2a3c)
                };

                let status_color = match &entry.status {
                    PortForwardStatus::Active => rgb(0xa6e3a1),
                    PortForwardStatus::Failed(_) => rgb(0xf38ba8),
                    PortForwardStatus::Stopped => rgb(0x6c7086),
                };

                list = list.child(
                    div()
                        .flex()
                        .px_3()
                        .py_1()
                        .bg(bg)
                        .gap_2()
                        .text_sm()
                        .child(
                            div()
                                .w(px(60.0))
                                .text_color(rgb(0xcdd6f4))
                                .child(SharedString::from(entry.local_port.to_string())),
                        )
                        .child(
                            div()
                                .w(px(60.0))
                                .text_color(rgb(0xbac2de))
                                .child(SharedString::from(entry.remote_port.to_string())),
                        )
                        .child(
                            div()
                                .w(px(180.0))
                                .text_color(rgb(0xbac2de))
                                .overflow_x_hidden()
                                .child(SharedString::from(entry.pod_name.clone())),
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(rgb(0x6c7086))
                                .child(SharedString::from(entry.namespace.clone())),
                        )
                        .child(
                            div()
                                .w(px(70.0))
                                .text_color(status_color)
                                .child(SharedString::from(entry.status.label())),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_color(rgb(0x6c7086))
                                .child(SharedString::from(entry.started_at.clone())),
                        ),
                );
            }
            panel = panel.child(list);
        }

        // Footer
        panel = panel.child(
            div()
                .px_3()
                .py_1()
                .border_t_1()
                .border_color(rgb(0x45475a))
                .text_color(rgb(0x6c7086))
                .text_xs()
                .child("j/k: navigate | d: stop selected | Esc: close"),
        );

        panel
    }
}
