use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::table::{Column, TableDelegate, TableState};
use gpui_component::theme::ActiveTheme;

use crate::model::table::TableData;

/// Convert "SOME_NAME" or "SOMENAME" to title case "Some Name"
fn to_title_case(s: &str) -> String {
    s.split('_')
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => {
                    let mut result = c.to_uppercase().to_string();
                    result.extend(chars.map(|c| c.to_ascii_lowercase()));
                    result
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Delegate that feeds our TableData into gpui-component's Table
pub struct ResourceTableDelegate {
    pub data: TableData,
    columns: Vec<Column>,
    /// Title-cased display names for headers
    display_names: Vec<SharedString>,
}

impl ResourceTableDelegate {
    pub fn new(data: TableData) -> Self {
        let (columns, display_names) = Self::build_columns(&data);
        Self {
            data,
            columns,
            display_names,
        }
    }

    pub fn update_data(&mut self, data: TableData) {
        let (columns, display_names) = Self::build_columns(&data);
        self.columns = columns;
        self.display_names = display_names;
        self.data = data;
    }

    fn build_columns(data: &TableData) -> (Vec<Column>, Vec<SharedString>) {
        let columns = data
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                Column::new(
                    SharedString::from(format!("col_{i}")),
                    SharedString::from(to_title_case(&col.name)),
                )
                .width(col.min_width as f32 * 8.0)
            })
            .collect();

        let display_names = data
            .columns
            .iter()
            .map(|col| SharedString::from(to_title_case(&col.name)))
            .collect();

        (columns, display_names)
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

    fn render_th(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let name = self
            .display_names
            .get(col_ix)
            .cloned()
            .unwrap_or_default();

        let primary = cx.theme().primary;

        div()
            .size_full()
            .text_sm()
            .text_color(primary)
            .font_weight(FontWeight::MEDIUM)
            .child(name)
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
            .text_sm()
            .child(SharedString::from(text))
    }
}
