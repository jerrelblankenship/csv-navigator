//! Data module for CSV Navigator
//!
//! This module contains the core data structures and types for managing CSV data,
//! including the CsvTable struct for storing and manipulating CSV content.

use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Default sample size for column type inference
/// Analyzes up to this many rows to determine if a column is numeric or text
const TYPE_INFERENCE_SAMPLE_SIZE: usize = 10_000;

/// Error type for CSV operations
#[derive(Debug)]
pub enum CsvError {
    /// IO error during file operations
    Io(std::io::Error),
    /// CSV parsing error
    Csv(csv::Error),
    /// Invalid data structure (e.g., inconsistent column counts)
    InvalidData(String),
}

impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsvError::Io(e) => write!(f, "IO error: {}", e),
            CsvError::Csv(e) => write!(f, "CSV error: {}", e),
            CsvError::InvalidData(s) => write!(f, "Invalid data: {}", s),
        }
    }
}

impl std::error::Error for CsvError {}

impl From<std::io::Error> for CsvError {
    fn from(err: std::io::Error) -> Self {
        CsvError::Io(err)
    }
}

impl From<csv::Error> for CsvError {
    fn from(err: csv::Error) -> Self {
        CsvError::Csv(err)
    }
}

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

    /// Infers column types by analyzing a sample of the data
    ///
    /// This method examines up to `sample_size` rows (default 10,000) to determine
    /// whether each column contains numeric or text data. A column is considered
    /// numeric if all non-empty values can be parsed as f64 numbers.
    ///
    /// # Arguments
    ///
    /// * `sample_size` - Maximum number of rows to sample (None = use default)
    ///
    /// # Examples
    ///
    /// ```
    /// # use csv_navigator::data::CsvTable;
    /// let mut table = CsvTable::new();
    /// table.data.push(vec!["123".to_string(), "Alice".to_string()]);
    /// table.data.push(vec!["456".to_string(), "Bob".to_string()]);
    /// table.col_types = vec![csv_navigator::data::ColType::Text; 2];
    ///
    /// table.infer_column_types(None);
    /// assert_eq!(table.col_types[0], csv_navigator::data::ColType::Number);
    /// assert_eq!(table.col_types[1], csv_navigator::data::ColType::Text);
    /// ```
    pub fn infer_column_types(&mut self, sample_size: Option<usize>) {
        let col_count = self.column_count();
        if col_count == 0 {
            return;
        }

        let sample_size = sample_size.unwrap_or(TYPE_INFERENCE_SAMPLE_SIZE);
        let rows_to_check = self.data.len().min(sample_size);

        // Track whether each column is numeric
        // Start by assuming all columns are numeric, then eliminate as we find text
        let mut col_is_numeric = vec![true; col_count];

        for row_idx in 0..rows_to_check {
            if let Some(row) = self.data.get(row_idx) {
                for (col_idx, value) in row.iter().enumerate() {
                    if col_idx >= col_count {
                        break;
                    }

                    // Skip if already determined to be text
                    if !col_is_numeric[col_idx] {
                        continue;
                    }

                    // Skip empty values - they don't affect type determination
                    if value.trim().is_empty() {
                        continue;
                    }

                    // Try to parse as a number
                    if value.parse::<f64>().is_err() {
                        col_is_numeric[col_idx] = false;
                    }
                }
            }
        }

        // Update column types based on inference
        self.col_types = col_is_numeric
            .into_iter()
            .map(|is_num| {
                if is_num {
                    ColType::Number
                } else {
                    ColType::Text
                }
            })
            .collect();
    }

    /// Loads a CSV file from a file path
    ///
    /// This method uses buffered IO for efficient reading of large files.
    /// It automatically detects whether the CSV has headers based on the `has_headers` parameter.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CSV file
    /// * `has_headers` - Whether the CSV file has a header row
    ///
    /// # Returns
    ///
    /// Returns a Result containing a CsvTable or a CsvError
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use csv_navigator::data::CsvTable;
    ///
    /// let table = CsvTable::from_path("data.csv", true).unwrap();
    /// println!("Loaded {} rows", table.row_count());
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P, has_headers: bool) -> Result<Self, CsvError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader, has_headers)
    }

    /// Loads a CSV from a reader
    ///
    /// This method provides flexibility to load CSV data from any source that implements Read.
    /// Uses csv::ReaderBuilder for robust parsing.
    ///
    /// # Arguments
    ///
    /// * `reader` - Any type implementing the Read trait
    /// * `has_headers` - Whether the CSV data has a header row
    ///
    /// # Returns
    ///
    /// Returns a Result containing a CsvTable or a CsvError
    pub fn from_reader<R: Read>(reader: R, has_headers: bool) -> Result<Self, CsvError> {
        let mut csv_reader = ReaderBuilder::new()
            .has_headers(has_headers)
            .flexible(false) // Require consistent column counts
            .trim(csv::Trim::All)
            .from_reader(reader);

        let mut table = CsvTable::new();

        // Read headers if present
        if has_headers {
            let headers = csv_reader.headers()?;
            table.headers = Some(headers.iter().map(|s| s.to_string()).collect());
        }

        // Read all data rows
        for result in csv_reader.records() {
            let record = result?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();

            // Validate column count consistency
            if let Some(first_row) = table.data.first() {
                if row.len() != first_row.len() {
                    return Err(CsvError::InvalidData(format!(
                        "Inconsistent column count: expected {}, got {}",
                        first_row.len(),
                        row.len()
                    )));
                }
            }

            table.data.push(row);
        }

        // Initialize column types vector
        let col_count = table.column_count();
        table.col_types = vec![ColType::Text; col_count];

        // Infer column types from data sample
        table.infer_column_types(None);

        Ok(table)
    }

    /// Saves the CSV table to a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the CSV file will be written
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or a CsvError
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), CsvError> {
        let file = File::create(path)?;
        let mut writer = csv::Writer::from_writer(file);

        // Write headers if present
        if let Some(headers) = &self.headers {
            writer.write_record(headers)?;
        }

        // Write all data rows
        for row in &self.data {
            writer.write_record(row)?;
        }

        writer.flush()?;
        Ok(())
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

    // CSV Loading Tests

    #[test]
    fn test_load_csv_with_headers() {
        let csv_data = "Name,Age,City\nAlice,30,NYC\nBob,25,LA\nCharlie,35,Chicago";
        let table = CsvTable::from_reader(csv_data.as_bytes(), true).unwrap();

        assert_eq!(table.headers, Some(vec!["Name".to_string(), "Age".to_string(), "City".to_string()]));
        assert_eq!(table.row_count(), 3);
        assert_eq!(table.column_count(), 3);
        assert_eq!(table.get_cell(0, 0), Some("Alice"));
        assert_eq!(table.get_cell(1, 1), Some("25"));
        assert_eq!(table.get_cell(2, 2), Some("Chicago"));
    }

    #[test]
    fn test_load_csv_without_headers() {
        let csv_data = "Alice,30,NYC\nBob,25,LA\nCharlie,35,Chicago";
        let table = CsvTable::from_reader(csv_data.as_bytes(), false).unwrap();

        assert!(table.headers.is_none());
        assert_eq!(table.row_count(), 3);
        assert_eq!(table.column_count(), 3);
        assert_eq!(table.get_cell(0, 0), Some("Alice"));
    }

    #[test]
    fn test_load_csv_with_quotes() {
        let csv_data = r#"Name,Description
"Smith, John","A person named ""John"""
"Doe, Jane","Another person"#;
        let table = CsvTable::from_reader(csv_data.as_bytes(), true).unwrap();

        assert_eq!(table.row_count(), 2);
        assert_eq!(table.get_cell(0, 0), Some("Smith, John"));
        assert_eq!(table.get_cell(0, 1), Some(r#"A person named "John""#));
        assert_eq!(table.get_cell(1, 0), Some("Doe, Jane"));
    }

    #[test]
    fn test_load_csv_with_crlf() {
        let csv_data = "Name,Age\r\nAlice,30\r\nBob,25\r\n";
        let table = CsvTable::from_reader(csv_data.as_bytes(), true).unwrap();

        assert_eq!(table.row_count(), 2);
        assert_eq!(table.get_cell(0, 0), Some("Alice"));
        assert_eq!(table.get_cell(1, 0), Some("Bob"));
    }

    #[test]
    fn test_load_csv_with_trimming() {
        let csv_data = "Name,  Age  \n  Alice  ,  30  \n  Bob  ,  25  ";
        let table = CsvTable::from_reader(csv_data.as_bytes(), true).unwrap();

        // csv::Trim::All should trim whitespace
        assert_eq!(table.headers, Some(vec!["Name".to_string(), "Age".to_string()]));
        assert_eq!(table.get_cell(0, 0), Some("Alice"));
        assert_eq!(table.get_cell(0, 1), Some("30"));
    }

    #[test]
    fn test_load_csv_inconsistent_columns() {
        let csv_data = "Name,Age\nAlice,30\nBob,25,Extra";
        let result = CsvTable::from_reader(csv_data.as_bytes(), true);

        // Should fail with csv::Error due to flexible(false)
        assert!(result.is_err());
        match result {
            Err(CsvError::Csv(_)) | Err(CsvError::InvalidData(_)) => {
                // Either error is acceptable - csv crate catches it with flexible(false)
            }
            _ => panic!("Expected CSV or InvalidData error"),
        }
    }

    #[test]
    fn test_save_and_load_csv() {
        use std::io::Cursor;

        // Create a table
        let mut table = CsvTable::with_headers(vec!["Name".to_string(), "Age".to_string()]);
        table.data.push(vec!["Alice".to_string(), "30".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string()]);

        // Save to a buffer
        let mut buffer = Vec::new();
        {
            let mut writer = csv::Writer::from_writer(&mut buffer);
            if let Some(headers) = &table.headers {
                writer.write_record(headers).unwrap();
            }
            for row in &table.data {
                writer.write_record(row).unwrap();
            }
            writer.flush().unwrap();
        }

        // Load from buffer
        let cursor = Cursor::new(buffer);
        let loaded_table = CsvTable::from_reader(cursor, true).unwrap();

        assert_eq!(loaded_table.headers, table.headers);
        assert_eq!(loaded_table.row_count(), table.row_count());
        assert_eq!(loaded_table.get_cell(0, 0), Some("Alice"));
        assert_eq!(loaded_table.get_cell(1, 1), Some("25"));
    }

    #[test]
    fn test_empty_csv() {
        let csv_data = "";
        let table = CsvTable::from_reader(csv_data.as_bytes(), false).unwrap();

        assert_eq!(table.row_count(), 0);
        assert_eq!(table.column_count(), 0);
    }

    #[test]
    fn test_csv_with_only_headers() {
        let csv_data = "Name,Age,City";
        let table = CsvTable::from_reader(csv_data.as_bytes(), true).unwrap();

        assert_eq!(table.headers, Some(vec!["Name".to_string(), "Age".to_string(), "City".to_string()]));
        assert_eq!(table.row_count(), 0);
        assert_eq!(table.column_count(), 3);
    }

    #[test]
    fn test_col_types_initialized() {
        let csv_data = "Name,Age,City\nAlice,30,NYC";
        let table = CsvTable::from_reader(csv_data.as_bytes(), true).unwrap();

        assert_eq!(table.col_types.len(), 3);
        // After type inference: Name=Text, Age=Number, City=Text
        assert_eq!(table.col_types[0], ColType::Text);
        assert_eq!(table.col_types[1], ColType::Number);
        assert_eq!(table.col_types[2], ColType::Text);
    }

    // Column Type Inference Tests

    #[test]
    fn test_infer_all_numeric_column() {
        let mut table = CsvTable::new();
        table.data.push(vec!["123".to_string(), "Alice".to_string()]);
        table.data.push(vec!["456".to_string(), "Bob".to_string()]);
        table.data.push(vec!["789".to_string(), "Charlie".to_string()]);
        table.col_types = vec![ColType::Text; 2];

        table.infer_column_types(None);

        assert_eq!(table.col_types[0], ColType::Number);
        assert_eq!(table.col_types[1], ColType::Text);
    }

    #[test]
    fn test_infer_all_text_column() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string(), "NYC".to_string()]);
        table.data.push(vec!["Bob".to_string(), "LA".to_string()]);
        table.data.push(vec!["Charlie".to_string(), "Chicago".to_string()]);
        table.col_types = vec![ColType::Text; 2];

        table.infer_column_types(None);

        assert_eq!(table.col_types[0], ColType::Text);
        assert_eq!(table.col_types[1], ColType::Text);
    }

    #[test]
    fn test_infer_mixed_column_becomes_text() {
        let mut table = CsvTable::new();
        table.data.push(vec!["123".to_string()]);
        table.data.push(vec!["456".to_string()]);
        table.data.push(vec!["abc".to_string()]); // This makes it text
        table.col_types = vec![ColType::Text; 1];

        table.infer_column_types(None);

        assert_eq!(table.col_types[0], ColType::Text);
    }

    #[test]
    fn test_infer_with_empty_values() {
        let mut table = CsvTable::new();
        table.data.push(vec!["123".to_string()]);
        table.data.push(vec!["".to_string()]); // Empty should be ignored
        table.data.push(vec!["456".to_string()]);
        table.data.push(vec!["  ".to_string()]); // Whitespace should be ignored
        table.data.push(vec!["789".to_string()]);
        table.col_types = vec![ColType::Text; 1];

        table.infer_column_types(None);

        // Should still be numeric since empty values are ignored
        assert_eq!(table.col_types[0], ColType::Number);
    }

    #[test]
    fn test_infer_floating_point_numbers() {
        let mut table = CsvTable::new();
        table.data.push(vec!["123.45".to_string()]);
        table.data.push(vec!["678.90".to_string()]);
        table.data.push(vec!["-12.34".to_string()]);
        table.data.push(vec!["0.0".to_string()]);
        table.col_types = vec![ColType::Text; 1];

        table.infer_column_types(None);

        assert_eq!(table.col_types[0], ColType::Number);
    }

    #[test]
    fn test_infer_scientific_notation() {
        let mut table = CsvTable::new();
        table.data.push(vec!["1.23e10".to_string()]);
        table.data.push(vec!["4.56E-5".to_string()]);
        table.data.push(vec!["7.89e+2".to_string()]);
        table.col_types = vec![ColType::Text; 1];

        table.infer_column_types(None);

        assert_eq!(table.col_types[0], ColType::Number);
    }

    #[test]
    fn test_infer_all_empty_column() {
        let mut table = CsvTable::new();
        table.data.push(vec!["".to_string()]);
        table.data.push(vec!["  ".to_string()]);
        table.data.push(vec!["".to_string()]);
        table.col_types = vec![ColType::Text; 1];

        table.infer_column_types(None);

        // All empty should default to Number (optimistic inference)
        assert_eq!(table.col_types[0], ColType::Number);
    }

    #[test]
    fn test_infer_sample_size_limit() {
        let mut table = CsvTable::new();

        // Add 15,000 numeric rows
        for i in 0..15_000 {
            table.data.push(vec![i.to_string()]);
        }

        table.col_types = vec![ColType::Text; 1];

        // Should only sample first 10,000 rows (default)
        table.infer_column_types(None);

        assert_eq!(table.col_types[0], ColType::Number);
    }

    #[test]
    fn test_infer_custom_sample_size() {
        let mut table = CsvTable::new();

        // Add 100 numeric rows
        for i in 0..100 {
            table.data.push(vec![i.to_string()]);
        }
        // Add 1 text row
        table.data.push(vec!["text".to_string()]);

        table.col_types = vec![ColType::Text; 1];

        // Sample only first 50 rows - should be numeric
        table.infer_column_types(Some(50));
        assert_eq!(table.col_types[0], ColType::Number);

        // Re-test with full sample - should be text
        table.infer_column_types(Some(200));
        assert_eq!(table.col_types[0], ColType::Text);
    }

    #[test]
    fn test_infer_multiple_columns() {
        let csv_data = "Name,Age,Salary,City,Active\nAlice,30,50000.50,NYC,true\nBob,25,45000.00,LA,false";
        let table = CsvTable::from_reader(csv_data.as_bytes(), true).unwrap();

        assert_eq!(table.col_types.len(), 5);
        assert_eq!(table.col_types[0], ColType::Text);   // Name
        assert_eq!(table.col_types[1], ColType::Number); // Age
        assert_eq!(table.col_types[2], ColType::Number); // Salary
        assert_eq!(table.col_types[3], ColType::Text);   // City
        assert_eq!(table.col_types[4], ColType::Text);   // Active (true/false as text)
    }

    #[test]
    fn test_infer_empty_table() {
        let mut table = CsvTable::new();
        table.col_types = vec![];

        table.infer_column_types(None);

        assert_eq!(table.col_types.len(), 0);
    }
}
