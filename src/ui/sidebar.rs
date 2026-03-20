use gpui::*;

use crate::model::resources::RESOURCES;

/// Left sidebar showing available resource types grouped by category
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

    pub fn into_element(self) -> Div {
        let mut sidebar = div()
            .flex()
            .flex_col()
            .w(px(180.0))
            .h_full()
            .bg(rgb(0x181825))
            .border_r_1()
            .border_color(rgb(0x313244))
            .py_2();

        let mut current_category = "";

        for (i, entry) in RESOURCES.iter().enumerate() {
            // Render category header when it changes
            if entry.category != current_category {
                current_category = entry.category;
                sidebar = sidebar.child(
                    div()
                        .px_3()
                        .pt_3()
                        .pb_1()
                        .text_color(rgb(0x6c7086))
                        .text_xs()
                        .child(SharedString::from(current_category)),
                );
            }

            let is_active = entry.api_name == self.current_resource;
            let is_cursor = self.has_focus && i == self.selected_index;

            let bg = if is_cursor {
                rgb(0x585b70)
            } else if is_active {
                rgb(0x313244)
            } else {
                rgb(0x181825)
            };

            let text_color = if is_active {
                rgb(0x89b4fa)
            } else if is_cursor {
                rgb(0xcdd6f4)
            } else {
                rgb(0xbac2de)
            };

            let indicator = if is_active { "›" } else { " " };

            sidebar = sidebar.child(
                div()
                    .px_3()
                    .py_px()
                    .bg(bg)
                    .text_color(text_color)
                    .text_sm()
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .w(px(10.0))
                            .text_color(rgb(0x89b4fa))
                            .child(indicator),
                    )
                    .child(SharedString::from(entry.display_name)),
            );
        }

        sidebar
    }
}
