use gpui::*;

use crate::model::table::TableData;

/// The main resource table component
pub struct ResourceTable {
    data: TableData,
    selected_row: usize,
}

impl ResourceTable {
    pub fn new(data: &TableData, selected_row: usize) -> Self {
        Self {
            data: data.clone(),
            selected_row,
        }
    }

    fn render_header_row(&self) -> Div {
        let mut row = div()
            .flex()
            .w_full()
            .px_4()
            .py_1()
            .bg(rgb(0x45475a))
            .gap_2();

        for col in &self.data.columns {
            row = row.child(
                div()
                    .min_w(px(col.min_width as f32 * 8.0))
                    .flex_1()
                    .text_color(rgb(0x89b4fa))
                    .child(SharedString::from(col.name.clone())),
            );
        }

        row
    }

    fn render_data_row(&self, row_idx: usize) -> Div {
        let row_data = &self.data.rows[row_idx];
        let is_selected = row_idx == self.selected_row;

        let bg = if is_selected {
            rgb(0x585b70)
        } else if row_idx % 2 == 0 {
            rgb(0x1e1e2e)
        } else {
            rgb(0x24243a)
        };

        let text_color = if is_selected {
            rgb(0xcdd6f4)
        } else {
            rgb(0xbac2de)
        };

        let mut row = div()
            .flex()
            .w_full()
            .px_4()
            .py_px()
            .bg(bg)
            .text_color(text_color)
            .gap_2();

        for (i, cell) in row_data.cells.iter().enumerate() {
            let min_w = self
                .data
                .columns
                .get(i)
                .map(|c| c.min_width)
                .unwrap_or(10);
            row = row.child(
                div()
                    .min_w(px(min_w as f32 * 8.0))
                    .flex_1()
                    .overflow_x_hidden()
                    .child(SharedString::from(cell.clone())),
            );
        }

        row
    }

    pub fn into_element(self) -> Div {
        let mut table = div().flex().flex_col().w_full();

        // Header
        if !self.data.columns.is_empty() {
            table = table.child(self.render_header_row());
        }

        // Data rows
        for i in 0..self.data.rows.len() {
            table = table.child(self.render_data_row(i));
        }

        // Empty state
        if self.data.rows.is_empty() && !self.data.columns.is_empty() {
            table = table.child(
                div()
                    .flex()
                    .w_full()
                    .py_8()
                    .justify_center()
                    .text_color(rgb(0x6c7086))
                    .child("No resources found"),
            );
        }

        table
    }
}
