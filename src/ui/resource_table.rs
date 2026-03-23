use gpui::*;
use gpui_component::table::{Column, TableDelegate, TableState};

use crate::model::table::TableData;

/// Delegate that feeds our TableData into gpui-component's Table
pub struct ResourceTableDelegate {
    pub data: TableData,
    columns: Vec<Column>,
}

impl ResourceTableDelegate {
    pub fn new(data: TableData) -> Self {
        let columns = data
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                Column::new(
                    SharedString::from(format!("col_{i}")),
                    SharedString::from(col.name.clone()),
                )
                .width(col.min_width as f32 * 8.0)
            })
            .collect();

        Self { data, columns }
    }

    pub fn update_data(&mut self, data: TableData) {
        self.columns = data
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                Column::new(
                    SharedString::from(format!("col_{i}")),
                    SharedString::from(col.name.clone()),
                )
                .width(col.min_width as f32 * 8.0)
            })
            .collect();
        self.data = data;
    }
}

impl TableDelegate for ResourceTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.data.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let text = self
            .data
            .rows
            .get(row_ix)
            .and_then(|row| row.cells.get(col_ix))
            .cloned()
            .unwrap_or_default();

        div()
            .size_full()
            .overflow_x_hidden()
            .child(SharedString::from(text))
    }
}
