use gpui::*;

use crate::model::port_forward::PodPort;

/// Modal dialog for starting a port forward
pub struct PortForwardDialog {
    pod_name: String,
    ports: Vec<PodPort>,
    selected_port: usize,
    local_port: String,
    loading: bool,
    spinner: String,
}

impl PortForwardDialog {
    pub fn new(
        pod_name: &str,
        ports: &[PodPort],
        selected_port: usize,
        local_port: &str,
        loading: bool,
        spinner: &str,
    ) -> Self {
        Self {
            pod_name: pod_name.to_string(),
            ports: ports.to_vec(),
            selected_port,
            local_port: local_port.to_string(),
            loading: loading,
            spinner: spinner.to_string(),
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
            .w(px(450.0))
            .max_h(px(400.0))
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
                            .child("Port Forward"),
                    )
                    .child(
                        div()
                            .text_color(rgb(0xf9e2af))
                            .text_sm()
                            .child(SharedString::from(self.pod_name.clone())),
                    ),
            );

        if self.loading {
            panel = panel.child(
                div()
                    .px_3()
                    .py_4()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(
                        div()
                            .text_color(rgb(0x89b4fa))
                            .child(SharedString::from(self.spinner)),
                    )
                    .child(
                        div()
                            .text_color(rgb(0x6c7086))
                            .child("Detecting ports..."),
                    ),
            );
        } else if self.ports.is_empty() {
            panel = panel.child(
                div()
                    .px_3()
                    .py_4()
                    .text_color(rgb(0x6c7086))
                    .child("No exposed ports found. Enter ports manually."),
            );
        } else {
            // Port list
            panel = panel.child(
                div()
                    .px_3()
                    .py_1()
                    .text_color(rgb(0x6c7086))
                    .text_xs()
                    .child("Select remote port:"),
            );

            let mut list = div().flex().flex_col();
            for (i, port) in self.ports.iter().enumerate() {
                let is_selected = i == self.selected_port;
                let bg = if is_selected {
                    rgb(0x585b70)
                } else {
                    rgb(0x313244)
                };
                let text_color = if is_selected {
                    rgb(0xcdd6f4)
                } else {
                    rgb(0xbac2de)
                };

                list = list.child(
                    div()
                        .px_3()
                        .py_1()
                        .bg(bg)
                        .text_color(text_color)
                        .text_sm()
                        .flex()
                        .gap_2()
                        .child(SharedString::from(port.display())),
                );
            }
            panel = panel.child(list);
        }

        // Local port input
        panel = panel.child(
            div()
                .px_3()
                .py_2()
                .border_t_1()
                .border_color(rgb(0x45475a))
                .flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_color(rgb(0x6c7086))
                        .text_sm()
                        .child("Local port:"),
                )
                .child(
                    div()
                        .px_2()
                        .py_px()
                        .bg(rgb(0x1e1e2e))
                        .border_1()
                        .border_color(rgb(0x45475a))
                        .rounded_sm()
                        .min_w(px(80.0))
                        .text_color(rgb(0xcdd6f4))
                        .child(if self.local_port.is_empty() {
                            SharedString::from("(same as remote)")
                        } else {
                            SharedString::from(self.local_port.clone())
                        }),
                ),
        );

        // Footer
        panel = panel.child(
            div()
                .px_3()
                .py_1()
                .border_t_1()
                .border_color(rgb(0x45475a))
                .text_color(rgb(0x6c7086))
                .text_xs()
                .child("j/k: select port | type: local port | Enter: start | Esc: cancel"),
        );

        panel
    }
}
