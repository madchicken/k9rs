use gpui::*;

/// A modal overlay for selecting a namespace
pub struct NamespacePicker {
    namespaces: Vec<String>,
    selected: usize,
    filter: String,
    current_namespace: String,
    loading: bool,
    spinner: String,
}

impl NamespacePicker {
    pub fn new(
        namespaces: &[String],
        selected: usize,
        filter: &str,
        current_namespace: &str,
        loading: bool,
        spinner: &str,
    ) -> Self {
        Self {
            namespaces: namespaces.to_vec(),
            selected,
            filter: filter.to_string(),
            current_namespace: current_namespace.to_string(),
            loading,
            spinner: spinner.to_string(),
        }
    }

    pub fn into_element(self) -> Div {
        // Full-screen backdrop to capture clicks and prevent pass-through
        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full()
            .bg(rgba(0x00000088))
            .flex()
            .justify_center()
            .pt_8()
            .on_mouse_down(MouseButton::Left, |_, _, _| {
                // Absorb clicks on the backdrop
            })
            .child(self.render_panel())
    }

    fn render_panel(self) -> Div {
        let mut panel = div()
            .w(px(400.0))
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
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(rgb(0x45475a))
                    .child(
                        div()
                            .text_color(rgb(0x89b4fa))
                            .child("Switch Namespace"),
                    ),
            )
            // Filter input display
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(0x45475a))
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .text_color(rgb(0x6c7086))
                            .child("Filter:"),
                    )
                    .child(
                        div()
                            .text_color(rgb(0xcdd6f4))
                            .child(if self.filter.is_empty() {
                                SharedString::from("(type to filter)")
                            } else {
                                SharedString::from(self.filter.clone())
                            }),
                    ),
            );

        // Namespace list
        let mut list = div().flex().flex_col();

        if self.loading {
            list = list.child(
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
                            .child("Loading namespaces..."),
                    ),
            );
        } else if self.namespaces.is_empty() {
            list = list.child(
                div()
                    .px_3()
                    .py_2()
                    .text_color(rgb(0x6c7086))
                    .child("No matching namespaces"),
            );
        } else {
            for (i, ns) in self.namespaces.iter().enumerate() {
                let is_selected = i == self.selected;
                let is_current = *ns == self.current_namespace;

                let bg = if is_selected {
                    rgb(0x585b70)
                } else {
                    rgb(0x313244)
                };

                let text_color = if is_current {
                    rgb(0xa6e3a1)
                } else if is_selected {
                    rgb(0xcdd6f4)
                } else {
                    rgb(0xbac2de)
                };

                let mut row = div()
                    .px_3()
                    .py_1()
                    .bg(bg)
                    .text_color(text_color)
                    .flex()
                    .gap_2();

                if is_current {
                    row = row.child(
                        div()
                            .text_color(rgb(0xa6e3a1))
                            .child("*"),
                    );
                }

                row = row.child(SharedString::from(ns.clone()));

                list = list.child(row);
            }
        }

        panel = panel.child(list);

        // Help footer
        panel = panel.child(
            div()
                .px_3()
                .py_1()
                .border_t_1()
                .border_color(rgb(0x45475a))
                .text_color(rgb(0x6c7086))
                .child("j/k: navigate | Enter: select | Esc: cancel | Backspace: clear filter"),
        );

        panel
    }
}
