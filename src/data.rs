//! Data module for CSV Navigator
//!
//! This module contains the core data structures and types for managing CSV data,
//! including the CsvTable struct for storing and manipulating CSV content.

use serde::{Deserialize, Serialize};

/// Column type enumeration for distinguishing between text and numeric columns
///
/// This is used for proper sorting behavior and potential future type-specific operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColType {
    /// Text column - sorted lexicographically
    #[default]
    Text,
    /// Numeric column - sorted numerically
    Number,
}

/// Main data structure for storing and managing CSV table data
///
/// The CsvTable struct holds all the data for a loaded CSV file, including:
/// - Optional headers (first row)
/// - All data rows as strings
/// - Optional filtered indices for displaying a subset of rows
/// - Column type information for proper sorting and display
///
/// # Performance Considerations
///
/// This structure is designed to handle large CSV files (target: 3M rows).
/// - Filtering uses index projection rather than copying data
/// - Sorting operates on the underlying data or filtered indices
/// - All data is stored as strings to preserve original formatting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvTable {
    /// Optional header row
    /// None indicates the CSV has no header row
    pub headers: Option<Vec<String>>,

    /// All data rows stored as strings
    /// Each inner Vec represents one row with each element being a column value
    pub data: Vec<Vec<String>>,

    /// Optional filtered view of the data
    /// When present, contains indices into the data Vec for rows matching the current filter
    /// None indicates no filter is active (all rows visible)
    pub filtered_indices: Option<Vec<usize>>,

    /// Column type information for each column
    /// Used for proper sorting (numeric vs text) and potential future features
    /// Length should match the number of columns in the data
    pub col_types: Vec<ColType>,
}

impl CsvTable {
    /// Creates a new empty CsvTable
    pub fn new() -> Self {
        Self {
            headers: None,
            data: Vec::new(),
            filtered_indices: None,
            col_types: Vec::new(),
        }
    }

    /// Creates a new CsvTable with specified headers
    pub fn with_headers(headers: Vec<String>) -> Self {
        let col_count = headers.len();
        Self {
            headers: Some(headers),
            data: Vec::new(),
            filtered_indices: None,
            col_types: vec![ColType::Text; col_count],
        }
    }

    /// Returns the number of columns in the table
    pub fn column_count(&self) -> usize {
        self.headers
            .as_ref()
            .map(|h| h.len())
            .or_else(|| self.data.first().map(|row| row.len()))
            .unwrap_or(0)
    }

    /// Returns the total number of data rows (unfiltered)
    pub fn row_count(&self) -> usize {
        self.data.len()
    }

    /// Returns the number of visible rows (filtered or total)
    pub fn visible_row_count(&self) -> usize {
        self.filtered_indices
            .as_ref()
            .map(|indices| indices.len())
            .unwrap_or_else(|| self.row_count())
    }

    /// Returns whether a filter is currently active
    pub fn is_filtered(&self) -> bool {
        self.filtered_indices.is_some()
    }

    /// Clears any active filter, showing all rows
    pub fn clear_filter(&mut self) {
        self.filtered_indices = None;
    }

    /// Gets a reference to a specific row by its actual index
    pub fn get_row(&self, index: usize) -> Option<&Vec<String>> {
        self.data.get(index)
    }

    /// Gets a reference to a specific row by its visible index
    /// (accounting for filtering)
    pub fn get_visible_row(&self, visible_index: usize) -> Option<&Vec<String>> {
        match &self.filtered_indices {
            Some(indices) => {
                // With filter: get actual index from filtered indices
                indices.get(visible_index).and_then(|&idx| self.get_row(idx))
            }
            None => {
                // No filter: visible index is actual index
                self.get_row(visible_index)
            }
        }
    }

    /// Gets a specific cell value by row and column index
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&str> {
        self.data.get(row).and_then(|r| r.get(col)).map(|s| s.as_str())
    }

    /// Sets a specific cell value
    /// Returns true if successful, false if indices are out of bounds
    pub fn set_cell(&mut self, row: usize, col: usize, value: String) -> bool {
        if let Some(row_data) = self.data.get_mut(row) {
            if let Some(cell) = row_data.get_mut(col) {
                *cell = value;
                return true;
            }
        }
        false
    }

    /// Returns an iterator over visible row indices
    pub fn visible_indices(&self) -> Box<dyn Iterator<Item = usize> + '_> {
        match &self.filtered_indices {
            Some(indices) => Box::new(indices.iter().copied()),
            None => Box::new(0..self.data.len()),
        }
    }
}

impl Default for CsvTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_table_new() {
        let table = CsvTable::new();
        assert!(table.headers.is_none());
        assert_eq!(table.data.len(), 0);
        assert!(table.filtered_indices.is_none());
        assert_eq!(table.col_types.len(), 0);
    }

    #[test]
    fn test_csv_table_with_headers() {
        let headers = vec!["Name".to_string(), "Age".to_string(), "City".to_string()];
        let table = CsvTable::with_headers(headers.clone());

        assert_eq!(table.headers, Some(headers));
        assert_eq!(table.column_count(), 3);
        assert_eq!(table.col_types.len(), 3);
        assert!(table.col_types.iter().all(|t| *t == ColType::Text));
    }

    #[test]
    fn test_column_count() {
        let mut table = CsvTable::new();
        assert_eq!(table.column_count(), 0);

        table.headers = Some(vec!["A".to_string(), "B".to_string()]);
        assert_eq!(table.column_count(), 2);

        let mut table2 = CsvTable::new();
        table2.data.push(vec!["1".to_string(), "2".to_string(), "3".to_string()]);
        assert_eq!(table2.column_count(), 3);
    }

    #[test]
    fn test_row_count_and_visible_count() {
        let mut table = CsvTable::new();
        table.data = vec![
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string(), "4".to_string()],
            vec!["5".to_string(), "6".to_string()],
        ];

        assert_eq!(table.row_count(), 3);
        assert_eq!(table.visible_row_count(), 3);
        assert!(!table.is_filtered());

        table.filtered_indices = Some(vec![0, 2]);
        assert_eq!(table.row_count(), 3);
        assert_eq!(table.visible_row_count(), 2);
        assert!(table.is_filtered());
    }

    #[test]
    fn test_get_row() {
        let mut table = CsvTable::new();
        table.data = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string()],
        ];

        assert_eq!(table.get_row(0), Some(&vec!["a".to_string(), "b".to_string()]));
        assert_eq!(table.get_row(1), Some(&vec!["c".to_string(), "d".to_string()]));
        assert_eq!(table.get_row(2), None);
    }

    #[test]
    fn test_get_visible_row() {
        let mut table = CsvTable::new();
        table.data = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string()],
            vec!["e".to_string(), "f".to_string()],
        ];

        // Without filter
        assert_eq!(table.get_visible_row(1), Some(&vec!["c".to_string(), "d".to_string()]));

        // With filter (showing rows 0 and 2)
        table.filtered_indices = Some(vec![0, 2]);
        assert_eq!(table.get_visible_row(0), Some(&vec!["a".to_string(), "b".to_string()]));
        assert_eq!(table.get_visible_row(1), Some(&vec!["e".to_string(), "f".to_string()]));
        assert_eq!(table.get_visible_row(2), None);
    }

    #[test]
    fn test_get_and_set_cell() {
        let mut table = CsvTable::new();
        table.data = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string()],
        ];

        assert_eq!(table.get_cell(0, 0), Some("a"));
        assert_eq!(table.get_cell(1, 1), Some("d"));
        assert_eq!(table.get_cell(2, 0), None);
        assert_eq!(table.get_cell(0, 3), None);

        assert!(table.set_cell(0, 0, "x".to_string()));
        assert_eq!(table.get_cell(0, 0), Some("x"));

        assert!(!table.set_cell(5, 5, "invalid".to_string()));
    }

    #[test]
    fn test_clear_filter() {
        let mut table = CsvTable::new();
        table.data = vec![
            vec!["1".to_string()],
            vec!["2".to_string()],
        ];
        table.filtered_indices = Some(vec![0]);

        assert!(table.is_filtered());
        table.clear_filter();
        assert!(!table.is_filtered());
        assert_eq!(table.visible_row_count(), 2);
    }

    #[test]
    fn test_visible_indices() {
        let mut table = CsvTable::new();
        table.data = vec![
            vec!["1".to_string()],
            vec!["2".to_string()],
            vec!["3".to_string()],
        ];

        // Without filter
        let indices: Vec<usize> = table.visible_indices().collect();
        assert_eq!(indices, vec![0, 1, 2]);

        // With filter
        table.filtered_indices = Some(vec![0, 2]);
        let indices: Vec<usize> = table.visible_indices().collect();
        assert_eq!(indices, vec![0, 2]);
    }

    #[test]
    fn test_col_type_default() {
        let col_type = ColType::default();
        assert_eq!(col_type, ColType::Text);
    }
}
