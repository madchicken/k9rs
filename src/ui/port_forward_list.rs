use gpui::*;

use crate::model::port_forward::{PortForwardEntry, PortForwardStatus};
use crate::ui::theme::PanelColors;

/// Modal overlay listing all active/stopped port forwards
pub struct PortForwardList {
    entries: Vec<PortForwardEntry>,
    selected: usize,
    colors: PanelColors,
}

impl PortForwardList {
    pub fn new(entries: &[PortForwardEntry], selected: usize, colors: PanelColors) -> Self {
        Self {
            entries: entries.to_vec(),
            selected,
            colors,
        }
    }

    pub fn into_element(self) -> Div {
        let overlay = self.colors.overlay;
        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full()
            .bg(overlay)
            .flex()
            .justify_center()
            .pt_8()
            .on_mouse_down(MouseButton::Left, |_, _, _| {})
            .child(self.render_panel())
    }

    fn render_panel(self) -> Div {
        let colors = &self.colors;
        let mut panel = div()
            .w(px(600.0))
            .max_h(px(500.0))
            .bg(colors.muted)
            .border_1()
            .border_color(colors.selection)
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
                    .border_color(colors.border)
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().text_color(colors.primary).child("Port Forwards"))
                    .child(div().text_color(colors.muted_foreground).text_sm().child(
                        SharedString::from(format!("({} total)", self.entries.len())),
                    )),
            );

        if self.entries.is_empty() {
            panel = panel.child(
                div()
                    .px_3()
                    .py_4()
                    .text_color(colors.muted_foreground)
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
                    .text_color(colors.primary)
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
                    colors.selection
                } else if i % 2 == 0 {
                    colors.muted
                } else {
                    colors.list_even
                };

                let status_color = match &entry.status {
                    PortForwardStatus::Active => colors.success,
                    PortForwardStatus::Failed(_) => colors.danger,
                    PortForwardStatus::Stopped => colors.muted_foreground,
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
                                .text_color(colors.foreground)
                                .child(SharedString::from(entry.local_port.to_string())),
                        )
                        .child(
                            div()
                                .w(px(60.0))
                                .text_color(colors.secondary_foreground)
                                .child(SharedString::from(entry.remote_port.to_string())),
                        )
                        .child(
                            div()
                                .w(px(180.0))
                                .text_color(colors.secondary_foreground)
                                .overflow_x_hidden()
                                .child(SharedString::from(entry.pod_name.clone())),
                        )
                        .child(
                            div()
                                .w(px(100.0))
                                .text_color(colors.muted_foreground)
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
                                .text_color(colors.muted_foreground)
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
                .border_color(colors.border)
                .text_color(colors.muted_foreground)
                .text_xs()
                .child("j/k: navigate | d: stop selected | Esc: close"),
        );

        panel
    }
}
