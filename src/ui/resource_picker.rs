use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::ui::theme::PanelColors;

/// A modal overlay for selecting a resource type
pub struct ResourcePicker {
    resources: Vec<(String, String, String)>, // (display_name, api_name, category)
    selected: usize,
    filter: String,
    current_resource: String,
    colors: PanelColors,
}

impl ResourcePicker {
    pub fn new(
        resources: &[(String, String, String)],
        selected: usize,
        filter: &str,
        current_resource: &str,
        colors: PanelColors,
    ) -> Self {
        Self {
            resources: resources.to_vec(),
            selected,
            filter: filter.to_string(),
            current_resource: current_resource.to_string(),
            colors,
        }
    }

    pub fn into_element(
        self,
        on_item_click: impl Fn(usize, &MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Div {
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
            .child(self.render_panel(on_item_click))
    }

    fn render_panel(
        self,
        on_item_click: impl Fn(usize, &MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Div {
        let on_item_click = std::rc::Rc::new(on_item_click);
        let colors = &self.colors;

        let mut panel = div()
            .w(px(400.0))
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
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .text_color(colors.primary)
                            .child("Switch Resource"),
                    ),
            )
            // Live filter indicator — only shown when typing
            .when(!self.filter.is_empty(), |this| {
                this.child(
                    div()
                        .px_3()
                        .py_1()
                        .border_b_1()
                        .border_color(colors.border)
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_color(colors.muted_foreground)
                                .text_sm()
                                .child("⌕"),
                        )
                        .child(
                            div()
                                .text_color(colors.foreground)
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
                    .text_color(colors.muted_foreground)
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
                            .text_color(colors.primary)
                            .text_sm()
                            .child(SharedString::from(category.clone())),
                    );
                    last_category = category.clone();
                }

                let is_selected = i == self.selected;
                let is_current = *api_name == self.current_resource;

                let bg = if is_selected {
                    colors.selection
                } else {
                    colors.muted
                };

                let text_color = if is_current {
                    colors.danger // pink accent for current resource
                } else if is_selected {
                    colors.foreground
                } else {
                    colors.secondary_foreground
                };

                let hover_bg = colors.border;
                let cb = on_item_click.clone();
                let mut row = div()
                    .id(SharedString::from(format!("res-row-{i}")))
                    .px_3()
                    .ml_2()
                    .py_1()
                    .bg(bg)
                    .text_color(text_color)
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .flex()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, move |ev, window, cx| {
                        cb(i, ev, window, cx);
                    });

                if is_current {
                    row = row.child(
                        div()
                            .text_color(colors.danger)
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
                .border_color(colors.border)
                .text_color(colors.muted_foreground)
                .child("↑↓: navigate | Type to filter | Esc: close"),
        );

        panel
    }
}
