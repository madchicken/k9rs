use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::ui::theme::PanelColors;

/// A modal overlay for selecting a namespace
pub struct NamespacePicker {
    namespaces: Vec<String>,
    selected: usize,
    filter: String,
    current_namespace: String,
    loading: bool,
    spinner: String,
    colors: PanelColors,
}

impl NamespacePicker {
    pub fn new(
        namespaces: &[String],
        selected: usize,
        filter: &str,
        current_namespace: &str,
        loading: bool,
        spinner: &str,
        colors: PanelColors,
    ) -> Self {
        Self {
            namespaces: namespaces.to_vec(),
            selected,
            filter: filter.to_string(),
            current_namespace: current_namespace.to_string(),
            loading,
            spinner: spinner.to_string(),
            colors,
        }
    }

    pub fn into_element(
        self,
        on_item_click: impl Fn(usize, &MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Stateful<Div> {
        let overlay = self.colors.overlay;
        div()
            .id("ns-picker-overlay")
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
                    .child(div().text_color(colors.primary).child("Switch Namespace")),
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

        // Namespace list (scrollable)
        let mut list = div()
            .id("ns-picker-list")
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
                            .text_color(colors.primary)
                            .child(SharedString::from(self.spinner)),
                    )
                    .child(
                        div()
                            .text_color(colors.muted_foreground)
                            .child("Loading namespaces..."),
                    ),
            );
        } else if self.namespaces.is_empty() {
            list = list.child(
                div()
                    .px_3()
                    .py_2()
                    .text_color(colors.muted_foreground)
                    .child("No matching namespaces"),
            );
        } else {
            for (i, ns) in self.namespaces.iter().enumerate() {
                let is_selected = i == self.selected;
                let is_current = *ns == self.current_namespace;

                let bg = if is_selected {
                    colors.selection
                } else {
                    colors.muted
                };

                let text_color = if is_current {
                    colors.success
                } else if is_selected {
                    colors.foreground
                } else {
                    colors.secondary_foreground
                };

                let hover_bg = colors.border;
                let cb = on_item_click.clone();
                let mut row = div()
                    .id(SharedString::from(format!("ns-row-{i}")))
                    .px_3()
                    .py_1()
                    .bg(bg)
                    .text_color(text_color)
                    .flex()
                    .gap_2()
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .on_mouse_down(MouseButton::Left, move |ev, window, cx| {
                        cb(i, ev, window, cx);
                    });

                if is_current {
                    row = row.child(div().text_color(colors.success).child("*"));
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
                .border_color(colors.border)
                .text_color(colors.muted_foreground)
                .child("↑↓: navigate | Type to filter | Esc: close"),
        );

        panel
    }
}
