use gpui::*;
use gpui_component::ActiveTheme;

use crate::model::resources::RESOURCES;

/// Left sidebar showing available resource types grouped by category.
/// `on_click` closures are wired externally via `with_click_handlers`.
pub struct Sidebar {
    current_resource: String,
    selected_index: usize,
    has_focus: bool,
}

impl Sidebar {
    pub fn new(current_resource: &str, selected_index: usize, has_focus: bool) -> Self {
        Self {
            current_resource: current_resource.to_string(),
            selected_index,
            has_focus,
        }
    }

    /// Build the sidebar element with click handlers.
    /// `on_item_click` is called with the resource index when an item is clicked.
    pub fn into_element_with_clicks(
        self,
        cx: &App,
        on_item_click: impl Fn(usize, &MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Div {
        let on_item_click = std::rc::Rc::new(on_item_click);

        let theme = cx.theme();
        let sidebar_bg = theme.sidebar;
        let muted_color = theme.muted;
        let muted_fg = theme.muted_foreground;
        let selection_color = theme.selection;
        let primary_color = theme.primary;
        let foreground_color = theme.foreground;
        let secondary_fg = theme.secondary_foreground;
        let border_color = theme.border;

        let mut sidebar = div()
            .flex()
            .flex_col()
            .w(px(180.0))
            .h_full()
            .bg(sidebar_bg)
            .border_r_1()
            .border_color(muted_color)
            .py_2();

        let mut current_category = "";

        for (i, entry) in RESOURCES.iter().enumerate() {
            if entry.category != current_category {
                current_category = entry.category;
                sidebar = sidebar.child(
                    div()
                        .px_3()
                        .pt_3()
                        .pb_1()
                        .text_color(muted_fg)
                        .text_xs()
                        .child(SharedString::from(current_category)),
                );
            }

            let is_active = entry.api_name == self.current_resource;
            let is_cursor = self.has_focus && i == self.selected_index;

            let bg = if is_cursor {
                selection_color
            } else if is_active {
                muted_color
            } else {
                sidebar_bg
            };

            let text_color = if is_active {
                primary_color
            } else if is_cursor {
                foreground_color
            } else {
                secondary_fg
            };

            let indicator = if is_active { "›" } else { " " };

            let cb = on_item_click.clone();
            sidebar = sidebar.child(
                div()
                    .px_3()
                    .py_px()
                    .bg(bg)
                    .text_color(text_color)
                    .text_sm()
                    .flex()
                    .gap_1()
                    .cursor_pointer()
                    .hover(move |s| s.bg(border_color))
                    .on_mouse_down(MouseButton::Left, move |ev, window, cx| {
                        cb(i, ev, window, cx);
                    })
                    .child(
                        div()
                            .w(px(10.0))
                            .text_color(primary_color)
                            .child(indicator),
                    )
                    .child(SharedString::from(entry.display_name)),
            );
        }

        sidebar
    }
}
