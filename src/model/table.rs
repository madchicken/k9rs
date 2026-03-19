/// A column definition for the resource table
#[derive(Debug, Clone)]
pub struct TableColumn {
    pub name: String,
    pub min_width: u32,
}

impl TableColumn {
    pub fn new(name: &str, min_width: u32) -> Self {
        Self {
            name: name.to_string(),
            min_width,
        }
    }
}

/// A single row of data in the resource table
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<String>,
}

/// Table data model — holds columns and rows for a resource listing
#[derive(Debug, Clone)]
pub struct TableData {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
}

impl TableData {
    /// Create an empty table (used as initial state)
    pub fn empty() -> Self {
        Self {
            columns: vec![],
            rows: vec![],
        }
    }
}
