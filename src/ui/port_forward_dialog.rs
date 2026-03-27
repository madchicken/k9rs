use gpui::*;
use gpui_component::input::{Input, InputState};

use crate::model::port_forward::PodPort;
use crate::ui::theme::PanelColors;

/// Modal dialog for starting a port forward
pub struct PortForwardDialog {
    pod_name: String,
    ports: Vec<PodPort>,
    selected_port: usize,
    local_port_input: Option<Entity<InputState>>,
    loading: bool,
    spinner: String,
    colors: PanelColors,
}

impl PortForwardDialog {
    pub fn new(
        pod_name: &str,
        ports: &[PodPort],
        selected_port: usize,
        local_port_input: Option<Entity<InputState>>,
        loading: bool,
        spinner: &str,
        colors: PanelColors,
    ) -> Self {
        Self {
            pod_name: pod_name.to_string(),
            ports: ports.to_vec(),
            selected_port,
            local_port_input,
            loading,
            spinner: spinner.to_string(),
            colors,
        }
    }

    pub fn into_element(self) -> Stateful<Div> {
        let overlay = self.colors.overlay;
        div()
            .id("pf-dialog-overlay")
            .occlude()
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
            .w(px(450.0))
            .max_h(px(400.0))
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
                    .child(div().text_color(colors.primary).child("Port Forward"))
                    .child(
                        div()
                            .text_color(colors.warning)
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
                            .text_color(colors.primary)
                            .child(SharedString::from(self.spinner)),
                    )
                    .child(
                        div()
                            .text_color(colors.muted_foreground)
                            .child("Detecting ports..."),
                    ),
            );
        } else if self.ports.is_empty() {
            panel = panel.child(
                div()
                    .px_3()
                    .py_4()
                    .text_color(colors.muted_foreground)
                    .child("No exposed ports found. Enter ports manually."),
            );
        } else {
            // Port list
            panel = panel.child(
                div()
                    .px_3()
                    .py_1()
                    .text_color(colors.muted_foreground)
                    .text_xs()
                    .child("Select remote port:"),
            );

            let mut list = div().flex().flex_col();
            for (i, port) in self.ports.iter().enumerate() {
                let is_selected = i == self.selected_port;
                let bg = if is_selected {
                    colors.selection
                } else {
                    colors.muted
                };
                let text_color = if is_selected {
                    colors.foreground
                } else {
                    colors.secondary_foreground
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

        // Local port input — uses gpui-component Input for proper text editing
        if let Some(input) = &self.local_port_input {
            panel = panel.child(
                div()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(colors.border)
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_color(colors.muted_foreground)
                            .text_sm()
                            .child("Local port:"),
                    )
                    .child(div().w(px(160.0)).child(Component::new(Input::new(input)))),
            );
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
                .child("↑↓: select port | Type: local port | Enter: start | Esc: cancel"),
        );

        panel
    }
}
