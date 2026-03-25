use gpui::*;

/// A modal overlay for selecting a Kubernetes context
pub struct ContextPicker {
    contexts: Vec<String>,
    selected: usize,
    filter: String,
    current_context: String,
    loading: bool,
    spinner: String,
}

impl ContextPicker {
    pub fn new(
        contexts: &[String],
        selected: usize,
        filter: &str,
        current_context: &str,
        loading: bool,
        spinner: &str,
    ) -> Self {
        Self {
            contexts: contexts.to_vec(),
            selected,
            filter: filter.to_string(),
            current_context: current_context.to_string(),
            loading,
            spinner: spinner.to_string(),
        }
    }

    pub fn into_element(
        self,
        on_select: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Div {
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
            .child(self.render_panel(on_select))
    }

    fn render_panel(
        self,
        on_select: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Div {
        let on_select = std::rc::Rc::new(on_select);

        let mut panel = div()
            .w(px(500.0))
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
                            .child("Switch Context"),
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

        // Context list (scrollable)
        let mut list = div()
            .id("ctx-picker-list")
            .flex_1()
            .overflow_scroll()
            .flex()
            .flex_col();

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
                            .child("Loading contexts..."),
                    ),
            );
        } else if self.contexts.is_empty() {
            list = list.child(
                div()
                    .px_3()
                    .py_2()
                    .text_color(rgb(0x6c7086))
                    .child("No matching contexts"),
            );
        } else {
            for (i, ctx) in self.contexts.iter().enumerate() {
                let is_selected = i == self.selected;
                let is_current = *ctx == self.current_context;

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

                let cb = on_select.clone();
                let mut row = div()
                    .id(SharedString::from(format!("ctx-row-{i}")))
                    .px_3()
                    .py_1()
                    .bg(bg)
                    .text_color(text_color)
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0x45475a)))
                    .flex()
                    .gap_2()
                    .on_click(move |ev, window, cx| cb(i, ev, window, cx));

                if is_current {
                    row = row.child(
                        div()
                            .text_color(rgb(0xa6e3a1))
                            .child("*"),
                    );
                }

                row = row.child(SharedString::from(ctx.clone()));

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
                .child("j/k: navigate | Enter/Click: select | Esc: cancel | Backspace: clear filter"),
        );

        panel
    }
}
