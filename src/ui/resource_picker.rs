use gpui::prelude::FluentBuilder;
use gpui::*;

/// A modal overlay for selecting a resource type
pub struct ResourcePicker {
    resources: Vec<(String, String, String)>, // (display_name, api_name, category)
    selected: usize,
    filter: String,
    current_resource: String,
}

impl ResourcePicker {
    pub fn new(
        resources: &[(String, String, String)],
        selected: usize,
        filter: &str,
        current_resource: &str,
    ) -> Self {
        Self {
            resources: resources.to_vec(),
            selected,
            filter: filter.to_string(),
            current_resource: current_resource.to_string(),
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
                            .child("Switch Resource"),
                    ),
            )
            // Live filter indicator
            .when(!self.filter.is_empty(), |this| {
                this.child(
                    div()
                        .px_3()
                        .py_1()
                        .border_b_1()
                        .border_color(rgb(0x45475a))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_color(rgb(0x6c7086))
                                .text_sm()
                                .child("⌕"),
                        )
                        .child(
                            div()
                                .text_color(rgb(0xcdd6f4))
                                .text_sm()
                                .child(SharedString::from(self.filter.clone())),
                        ),
                )
            });

        // Resource list (scrollable)
        let mut list = div()
            .id("res-picker-list")
            .flex_1()
            .overflow_scroll()
            .flex()
            .flex_col();

        if self.resources.is_empty() {
            list = list.child(
                div()
                    .px_3()
                    .py_2()
                    .text_color(rgb(0x6c7086))
                    .child("No matching resources"),
            );
        } else {
            let mut last_category = String::new();
            for (i, (display_name, api_name, category)) in self.resources.iter().enumerate() {
                // Category header
                if *category != last_category {
                    list = list.child(
                        div()
                            .px_3()
                            .pt_2()
                            .pb_1()
                            .text_color(rgb(0x89b4fa))
                            .text_sm()
                            .child(SharedString::from(category.clone())),
                    );
                    last_category = category.clone();
                }

                let is_selected = i == self.selected;
                let is_current = *api_name == self.current_resource;

                let bg = if is_selected {
                    rgb(0x585b70)
                } else {
                    rgb(0x313244)
                };

                let text_color = if is_current {
                    rgb(0xf38ba8) // pink, matching header
                } else if is_selected {
                    rgb(0xcdd6f4)
                } else {
                    rgb(0xbac2de)
                };

                let cb = on_select.clone();
                let mut row = div()
                    .id(SharedString::from(format!("res-row-{i}")))
                    .px_3()
                    .ml_2()
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
                            .text_color(rgb(0xf38ba8))
                            .child("*"),
                    );
                }

                row = row.child(SharedString::from(display_name.clone()));

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
                .child("↑↓: navigate | Type to filter | Esc: close"),
        );

        panel
    }
}
