//! Data module for CSV Navigator
//!
//! This module contains the core data structures and types for managing CSV data,
//! including the CsvTable struct for storing and manipulating CSV content.

use csv::ReaderBuilder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
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
    /// JSON serialization/deserialization error
    Json(serde_json::Error),
    /// Invalid data structure (e.g., inconsistent column counts)
    InvalidData(String),
}

impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsvError::Io(e) => write!(f, "IO error: {}", e),
            CsvError::Csv(e) => write!(f, "CSV error: {}", e),
            CsvError::Json(e) => write!(f, "JSON error: {}", e),
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

impl From<serde_json::Error> for CsvError {
    fn from(err: serde_json::Error) -> Self {
        CsvError::Json(err)
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

/// Sort order enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Ascending order (A-Z, 0-9)
    Ascending,
    /// Descending order (Z-A, 9-0)
    Descending,
}

/// Filter operator for comparing values
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOperator {
    /// Exact match (case-insensitive for text)
    Equals,
    /// Not equal (case-insensitive for text)
    NotEquals,
    /// Contains substring (case-insensitive)
    Contains,
    /// Does not contain substring (case-insensitive)
    NotContains,
    /// Starts with (case-insensitive)
    StartsWith,
    /// Ends with (case-insensitive)
    EndsWith,
    /// Greater than (numeric comparison)
    GreaterThan,
    /// Less than (numeric comparison)
    LessThan,
    /// Greater than or equal (numeric comparison)
    GreaterOrEqual,
    /// Less than or equal (numeric comparison)
    LessOrEqual,
}

/// A single filter condition
#[derive(Debug, Clone, PartialEq)]
pub struct FilterCondition {
    /// Column index to filter on
    pub column: usize,
    /// Comparison operator
    pub operator: FilterOperator,
    /// Value to compare against
    pub value: String,
}

impl FilterCondition {
    /// Creates a new filter condition
    pub fn new(column: usize, operator: FilterOperator, value: String) -> Self {
        Self {
            column,
            operator,
            value,
        }
    }

    /// Tests if a row matches this filter condition
    fn matches(&self, row: &[String], col_types: &[ColType]) -> bool {
        let cell_value = match row.get(self.column) {
            Some(v) => v,
            None => return false,
        };

        let col_type = col_types.get(self.column).copied().unwrap_or(ColType::Text);

        match (&self.operator, col_type) {
            (FilterOperator::Equals, ColType::Text) => {
                cell_value.to_lowercase() == self.value.to_lowercase()
            }
            (FilterOperator::Equals, ColType::Number) => {
                let cell_num = cell_value.parse::<f64>().ok();
                let filter_num = self.value.parse::<f64>().ok();
                cell_num.is_some() && cell_num == filter_num
            }
            (FilterOperator::NotEquals, ColType::Text) => {
                cell_value.to_lowercase() != self.value.to_lowercase()
            }
            (FilterOperator::NotEquals, ColType::Number) => {
                let cell_num = cell_value.parse::<f64>().ok();
                let filter_num = self.value.parse::<f64>().ok();
                cell_num.is_none() || cell_num != filter_num
            }
            (FilterOperator::Contains, _) => cell_value
                .to_lowercase()
                .contains(&self.value.to_lowercase()),
            (FilterOperator::NotContains, _) => {
                !cell_value
                    .to_lowercase()
                    .contains(&self.value.to_lowercase())
            }
            (FilterOperator::StartsWith, _) => cell_value
                .to_lowercase()
                .starts_with(&self.value.to_lowercase()),
            (FilterOperator::EndsWith, _) => cell_value
                .to_lowercase()
                .ends_with(&self.value.to_lowercase()),
            (FilterOperator::GreaterThan, _) => {
                let cell_num = cell_value.parse::<f64>().ok();
                let filter_num = self.value.parse::<f64>().ok();
                match (cell_num, filter_num) {
                    (Some(c), Some(f)) => c > f,
                    _ => false,
                }
            }
            (FilterOperator::LessThan, _) => {
                let cell_num = cell_value.parse::<f64>().ok();
                let filter_num = self.value.parse::<f64>().ok();
                match (cell_num, filter_num) {
                    (Some(c), Some(f)) => c < f,
                    _ => false,
                }
            }
            (FilterOperator::GreaterOrEqual, _) => {
                let cell_num = cell_value.parse::<f64>().ok();
                let filter_num = self.value.parse::<f64>().ok();
                match (cell_num, filter_num) {
                    (Some(c), Some(f)) => c >= f,
                    _ => false,
                }
            }
            (FilterOperator::LessOrEqual, _) => {
                let cell_num = cell_value.parse::<f64>().ok();
                let filter_num = self.value.parse::<f64>().ok();
                match (cell_num, filter_num) {
                    (Some(c), Some(f)) => c <= f,
                    _ => false,
                }
            }
        }
    }
}

/// Logic for combining multiple filter conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterLogic {
    /// All conditions must match (AND)
    All,
    /// Any condition can match (OR)
    Any,
}

/// Represents a single edit action that can be undone/redone
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditAction {
    /// Set a single cell value
    /// (row, col, old_value, new_value)
    SetCell {
        row: usize,
        col: usize,
        old_value: String,
        new_value: String,
    },
    /// Set multiple cells at once (grouped action)
    /// Vector of (row, col, old_value, new_value) tuples
    MultiSet(Vec<(usize, usize, String, String)>),
}

impl EditAction {
    /// Creates a SetCell action
    pub fn set_cell(row: usize, col: usize, old_value: String, new_value: String) -> Self {
        EditAction::SetCell {
            row,
            col,
            old_value,
            new_value,
        }
    }

    /// Creates a MultiSet action from a vector of cell changes
    pub fn multi_set(changes: Vec<(usize, usize, String, String)>) -> Self {
        EditAction::MultiSet(changes)
    }

    /// Returns the inverse action (for redo after undo)
    fn inverse(&self) -> Self {
        match self {
            EditAction::SetCell {
                row,
                col,
                old_value,
                new_value,
            } => EditAction::SetCell {
                row: *row,
                col: *col,
                old_value: new_value.clone(),
                new_value: old_value.clone(),
            },
            EditAction::MultiSet(changes) => {
                let inverted: Vec<_> = changes
                    .iter()
                    .map(|(r, c, old, new)| (*r, *c, new.clone(), old.clone()))
                    .collect();
                EditAction::MultiSet(inverted)
            }
        }
    }

    /// Applies this action to a table (used for redo)
    fn apply(&self, table: &mut CsvTable) -> Result<(), String> {
        match self {
            EditAction::SetCell {
                row,
                col,
                new_value,
                ..
            } => {
                if *row >= table.data.len() {
                    return Err(format!("Row {} out of bounds", row));
                }
                if *col >= table.data[*row].len() {
                    return Err(format!("Column {} out of bounds in row {}", col, row));
                }
                table.data[*row][*col] = new_value.clone();
                Ok(())
            }
            EditAction::MultiSet(changes) => {
                for (row, col, _, new_value) in changes {
                    if *row >= table.data.len() {
                        return Err(format!("Row {} out of bounds", row));
                    }
                    if *col >= table.data[*row].len() {
                        return Err(format!("Column {} out of bounds in row {}", col, row));
                    }
                    table.data[*row][*col] = new_value.clone();
                }
                Ok(())
            }
        }
    }
}

/// Manages undo/redo history for table edits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    /// Stack of actions that can be undone
    undo_stack: Vec<EditAction>,
    /// Stack of actions that can be redone
    redo_stack: Vec<EditAction>,
    /// Maximum number of actions to keep in history
    max_history: usize,
}

impl History {
    /// Creates a new history with the specified maximum size
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Adds an action to the undo stack and clears the redo stack
    pub fn push(&mut self, action: EditAction) {
        self.undo_stack.push(action);

        // Limit stack size
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }

        // New action invalidates redo stack
        self.redo_stack.clear();
    }

    /// Returns true if there are actions that can be undone
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true if there are actions that can be redone
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Pops an action from the undo stack
    fn pop_undo(&mut self) -> Option<EditAction> {
        self.undo_stack.pop()
    }

    /// Pops an action from the redo stack
    fn pop_redo(&mut self) -> Option<EditAction> {
        self.redo_stack.pop()
    }

    /// Pushes an action to the redo stack
    fn push_redo(&mut self, action: EditAction) {
        self.redo_stack.push(action);

        // Limit stack size
        if self.redo_stack.len() > self.max_history {
            self.redo_stack.remove(0);
        }
    }

    /// Pushes an action to the undo stack (used after redo)
    fn push_undo(&mut self, action: EditAction) {
        self.undo_stack.push(action);

        // Limit stack size
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Clears both undo and redo stacks
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new(100) // Default to 100 actions
    }
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

    /// Edit history for undo/redo functionality
    /// Tracks all edit actions to enable undo/redo
    pub history: History,
}

impl CsvTable {
    /// Creates a new empty CsvTable
    pub fn new() -> Self {
        Self {
            headers: None,
            data: Vec::new(),
            filtered_indices: None,
            col_types: Vec::new(),
            history: History::default(),
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
            history: History::default(),
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
                let old_value = cell.clone();
                *cell = value.clone();

                // Record the edit action in history
                let action = EditAction::set_cell(row, col, old_value, value);
                self.history.push(action);

                return true;
            }
        }
        false
    }

    /// Sets multiple cells at once (grouped action)
    /// Records as a single undoable action
    /// Returns the number of successfully updated cells
    pub fn set_cells(&mut self, changes: Vec<(usize, usize, String)>) -> usize {
        let mut successful_changes = Vec::new();
        let mut count = 0;

        for (row, col, new_value) in changes {
            if let Some(row_data) = self.data.get_mut(row) {
                if let Some(cell) = row_data.get_mut(col) {
                    let old_value = cell.clone();
                    *cell = new_value.clone();
                    successful_changes.push((row, col, old_value, new_value));
                    count += 1;
                }
            }
        }

        if !successful_changes.is_empty() {
            let action = EditAction::multi_set(successful_changes);
            self.history.push(action);
        }

        count
    }

    /// Undoes the last edit action
    /// Returns true if an action was undone, false if undo stack is empty
    pub fn undo(&mut self) -> Result<(), String> {
        if let Some(action) = self.history.pop_undo() {
            // Apply the inverse action (swap old and new values)
            let inverse = action.inverse();
            inverse.apply(self)?;

            // Push the original action to redo stack
            self.history.push_redo(action);

            Ok(())
        } else {
            Err("Nothing to undo".to_string())
        }
    }

    /// Redoes the last undone action
    /// Returns true if an action was redone, false if redo stack is empty
    pub fn redo(&mut self) -> Result<(), String> {
        if let Some(action) = self.history.pop_redo() {
            // Apply the action
            action.apply(self)?;

            // Push the action back to undo stack
            self.history.push_undo(action);

            Ok(())
        } else {
            Err("Nothing to redo".to_string())
        }
    }

    /// Returns true if there are actions that can be undone
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Returns true if there are actions that can be redone
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
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

    /// Sorts the table by a specified column
    ///
    /// Uses Rayon's parallel sorting for better performance on large datasets.
    /// Sorting is done in-place and respects the column's inferred type
    /// (numeric vs text sorting).
    ///
    /// # Arguments
    ///
    /// * `column` - Zero-based column index to sort by
    /// * `order` - Sort order (Ascending or Descending)
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or Err if column index is out of bounds
    ///
    /// # Examples
    ///
    /// ```
    /// # use csv_navigator::data::{CsvTable, SortOrder};
    /// let mut table = CsvTable::new();
    /// table.data.push(vec!["30".to_string(), "Alice".to_string()]);
    /// table.data.push(vec!["25".to_string(), "Bob".to_string()]);
    /// table.col_types = vec![csv_navigator::data::ColType::Number, csv_navigator::data::ColType::Text];
    ///
    /// table.sort_by_column(0, SortOrder::Ascending).unwrap();
    /// assert_eq!(table.get_cell(0, 0), Some("25"));
    /// assert_eq!(table.get_cell(1, 0), Some("30"));
    /// ```
    pub fn sort_by_column(&mut self, column: usize, order: SortOrder) -> Result<(), String> {
        // For empty tables, sorting is a no-op (always succeeds)
        if self.data.is_empty() {
            return Ok(());
        }

        if column >= self.column_count() {
            return Err(format!(
                "Column index {} out of bounds (table has {} columns)",
                column,
                self.column_count()
            ));
        }

        let col_type = self.col_types.get(column).copied().unwrap_or(ColType::Text);

        // Clear any active filter as sorting changes row order
        self.filtered_indices = None;

        // Use parallel sorting for better performance on large datasets
        match col_type {
            ColType::Number => {
                self.data.par_sort_unstable_by(|a, b| {
                    let val_a = a.get(column).and_then(|s| s.parse::<f64>().ok());
                    let val_b = b.get(column).and_then(|s| s.parse::<f64>().ok());

                    let cmp = match (val_a, val_b) {
                        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
                        (Some(_), None) => Ordering::Less,  // Numbers before non-numbers
                        (None, Some(_)) => Ordering::Greater,
                        (None, None) => Ordering::Equal,
                    };

                    match order {
                        SortOrder::Ascending => cmp,
                        SortOrder::Descending => cmp.reverse(),
                    }
                });
            }
            ColType::Text => {
                self.data.par_sort_unstable_by(|a, b| {
                    let val_a = a.get(column).map(|s| s.as_str()).unwrap_or("");
                    let val_b = b.get(column).map(|s| s.as_str()).unwrap_or("");

                    let cmp = val_a.cmp(val_b);

                    match order {
                        SortOrder::Ascending => cmp,
                        SortOrder::Descending => cmp.reverse(),
                    }
                });
            }
        }

        Ok(())
    }

    /// Applies filters to the table and updates filtered_indices
    ///
    /// This method evaluates all conditions against each row and stores the indices
    /// of rows that match. The filtered_indices field is updated accordingly.
    ///
    /// # Arguments
    ///
    /// * `conditions` - Vector of filter conditions to apply
    /// * `logic` - How to combine conditions (All for AND, Any for OR)
    ///
    /// # Examples
    ///
    /// ```
    /// # use csv_navigator::data::{CsvTable, FilterCondition, FilterOperator, FilterLogic};
    /// let mut table = CsvTable::new();
    /// table.data.push(vec!["Alice".to_string(), "30".to_string()]);
    /// table.data.push(vec!["Bob".to_string(), "25".to_string()]);
    /// table.col_types = vec![csv_navigator::data::ColType::Text, csv_navigator::data::ColType::Number];
    ///
    /// let conditions = vec![
    ///     FilterCondition::new(1, FilterOperator::GreaterThan, "26".to_string())
    /// ];
    /// table.apply_filters(&conditions, FilterLogic::All);
    /// assert_eq!(table.visible_row_count(), 1);
    /// ```
    pub fn apply_filters(&mut self, conditions: &[FilterCondition], logic: FilterLogic) {
        if conditions.is_empty() {
            self.filtered_indices = None;
            return;
        }

        let matching_indices: Vec<usize> = (0..self.data.len())
            .filter(|&idx| {
                let row = &self.data[idx];
                match logic {
                    FilterLogic::All => conditions.iter().all(|c| c.matches(row, &self.col_types)),
                    FilterLogic::Any => conditions.iter().any(|c| c.matches(row, &self.col_types)),
                }
            })
            .collect();

        self.filtered_indices = if matching_indices.is_empty() || matching_indices.len() == self.data.len() {
            None // No filter needed if all or no rows match
        } else {
            Some(matching_indices)
        };
    }

    /// Loads a CSV file from a file path
    ///
    /// This method uses buffered IO for efficient reading of large datasets.
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

    /// Exports the table to JSON as an array of objects
    ///
    /// Each row becomes a JSON object with header names as keys.
    /// If no headers are present, uses generic column names (col0, col1, etc.).
    ///
    /// # Arguments
    ///
    /// * `pretty` - If true, formats the JSON with indentation for readability
    ///
    /// # Returns
    ///
    /// Returns a JSON string on success or an error
    ///
    /// # Examples
    ///
    /// ```
    /// # use csv_navigator::data::CsvTable;
    /// let table = CsvTable::with_headers(vec!["Name".to_string(), "Age".to_string()]);
    /// // ... add data ...
    /// let json = table.to_json(false).unwrap();
    /// ```
    pub fn to_json(&self, pretty: bool) -> Result<String, CsvError> {
        let mut result = Vec::new();

        // Determine headers (use existing or generate generic ones)
        let headers: Vec<String> = match &self.headers {
            Some(h) => h.clone(),
            None => {
                let col_count = self.column_count();
                (0..col_count).map(|i| format!("col{}", i)).collect()
            }
        };

        // Convert each row to a JSON object
        for row in &self.data {
            let mut obj = serde_json::Map::new();
            for (i, value) in row.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    obj.insert(header.clone(), serde_json::Value::String(value.clone()));
                }
            }
            result.push(serde_json::Value::Object(obj));
        }

        // Serialize to JSON
        let json = if pretty {
            serde_json::to_string_pretty(&result)?
        } else {
            serde_json::to_string(&result)?
        };

        Ok(json)
    }

    /// Exports the table to a JSON file as an array of objects
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the JSON file will be written
    /// * `pretty` - If true, formats the JSON with indentation for readability
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use csv_navigator::data::CsvTable;
    /// let table = CsvTable::with_headers(vec!["Name".to_string(), "Age".to_string()]);
    /// // ... add data ...
    /// table.to_json_file("output.json", true).unwrap();
    /// ```
    pub fn to_json_file<P: AsRef<Path>>(&self, path: P, pretty: bool) -> Result<(), CsvError> {
        let json = self.to_json(pretty)?;
        std::fs::write(path, json)?;
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

    // Sorting Tests

    #[test]
    fn test_sort_numeric_ascending() {
        let mut table = CsvTable::new();
        table.data.push(vec!["30".to_string()]);
        table.data.push(vec!["10".to_string()]);
        table.data.push(vec!["20".to_string()]);
        table.col_types = vec![ColType::Number];

        table.sort_by_column(0, SortOrder::Ascending).unwrap();

        assert_eq!(table.get_cell(0, 0), Some("10"));
        assert_eq!(table.get_cell(1, 0), Some("20"));
        assert_eq!(table.get_cell(2, 0), Some("30"));
    }

    #[test]
    fn test_sort_numeric_descending() {
        let mut table = CsvTable::new();
        table.data.push(vec!["10".to_string()]);
        table.data.push(vec!["30".to_string()]);
        table.data.push(vec!["20".to_string()]);
        table.col_types = vec![ColType::Number];

        table.sort_by_column(0, SortOrder::Descending).unwrap();

        assert_eq!(table.get_cell(0, 0), Some("30"));
        assert_eq!(table.get_cell(1, 0), Some("20"));
        assert_eq!(table.get_cell(2, 0), Some("10"));
    }

    #[test]
    fn test_sort_text_ascending() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Charlie".to_string()]);
        table.data.push(vec!["Alice".to_string()]);
        table.data.push(vec!["Bob".to_string()]);
        table.col_types = vec![ColType::Text];

        table.sort_by_column(0, SortOrder::Ascending).unwrap();

        assert_eq!(table.get_cell(0, 0), Some("Alice"));
        assert_eq!(table.get_cell(1, 0), Some("Bob"));
        assert_eq!(table.get_cell(2, 0), Some("Charlie"));
    }

    #[test]
    fn test_sort_text_descending() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string()]);
        table.data.push(vec!["Charlie".to_string()]);
        table.data.push(vec!["Bob".to_string()]);
        table.col_types = vec![ColType::Text];

        table.sort_by_column(0, SortOrder::Descending).unwrap();

        assert_eq!(table.get_cell(0, 0), Some("Charlie"));
        assert_eq!(table.get_cell(1, 0), Some("Bob"));
        assert_eq!(table.get_cell(2, 0), Some("Alice"));
    }

    #[test]
    fn test_sort_preserves_row_integrity() {
        let mut table = CsvTable::new();
        table.data.push(vec!["30".to_string(), "Alice".to_string()]);
        table.data.push(vec!["10".to_string(), "Charlie".to_string()]);
        table.data.push(vec!["20".to_string(), "Bob".to_string()]);
        table.col_types = vec![ColType::Number, ColType::Text];

        table.sort_by_column(0, SortOrder::Ascending).unwrap();

        // Check that rows stayed together
        assert_eq!(table.get_cell(0, 0), Some("10"));
        assert_eq!(table.get_cell(0, 1), Some("Charlie"));
        assert_eq!(table.get_cell(1, 0), Some("20"));
        assert_eq!(table.get_cell(1, 1), Some("Bob"));
        assert_eq!(table.get_cell(2, 0), Some("30"));
        assert_eq!(table.get_cell(2, 1), Some("Alice"));
    }

    #[test]
    fn test_sort_invalid_column() {
        let mut table = CsvTable::new();
        table.data.push(vec!["1".to_string()]);
        table.col_types = vec![ColType::Number];

        let result = table.sort_by_column(5, SortOrder::Ascending);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_sort_empty_table() {
        let mut table = CsvTable::new();
        table.col_types = vec![ColType::Text];

        let result = table.sort_by_column(0, SortOrder::Ascending);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sort_clears_filter() {
        let mut table = CsvTable::new();
        table.data.push(vec!["30".to_string()]);
        table.data.push(vec!["10".to_string()]);
        table.filtered_indices = Some(vec![0]);
        table.col_types = vec![ColType::Number];

        assert!(table.is_filtered());
        table.sort_by_column(0, SortOrder::Ascending).unwrap();
        assert!(!table.is_filtered());
    }

    // Filtering tests
    #[test]
    fn test_filter_equals_text() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string(), "30".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string()]);
        table.data.push(vec!["alice".to_string(), "28".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::Equals,
            "alice".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 2); // Case insensitive match
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![0, 2]);
    }

    #[test]
    fn test_filter_equals_number() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string(), "30".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string()]);
        table.data.push(vec!["Charlie".to_string(), "30".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number];

        let conditions = vec![FilterCondition::new(
            1,
            FilterOperator::Equals,
            "30".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 2);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![0, 2]);
    }

    #[test]
    fn test_filter_contains() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice Smith".to_string()]);
        table.data.push(vec!["Bob Jones".to_string()]);
        table.data.push(vec!["Charlie Smith".to_string()]);
        table.col_types = vec![ColType::Text];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::Contains,
            "smith".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 2);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![0, 2]);
    }

    #[test]
    fn test_filter_not_contains() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice Smith".to_string()]);
        table.data.push(vec!["Bob Jones".to_string()]);
        table.data.push(vec!["Charlie Smith".to_string()]);
        table.col_types = vec![ColType::Text];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::NotContains,
            "Smith".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 1);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![1]);
    }

    #[test]
    fn test_filter_starts_with() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string()]);
        table.data.push(vec!["Bob".to_string()]);
        table.data.push(vec!["Alex".to_string()]);
        table.col_types = vec![ColType::Text];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::StartsWith,
            "al".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 2);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![0, 2]);
    }

    #[test]
    fn test_filter_ends_with() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string()]);
        table.data.push(vec!["Bob".to_string()]);
        table.data.push(vec!["Eve".to_string()]);
        table.col_types = vec![ColType::Text];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::EndsWith,
            "e".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 2);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![0, 2]);
    }

    #[test]
    fn test_filter_greater_than() {
        let mut table = CsvTable::new();
        table.data.push(vec!["30".to_string()]);
        table.data.push(vec!["25".to_string()]);
        table.data.push(vec!["35".to_string()]);
        table.col_types = vec![ColType::Number];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::GreaterThan,
            "28".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 2);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![0, 2]);
    }

    #[test]
    fn test_filter_less_than() {
        let mut table = CsvTable::new();
        table.data.push(vec!["30".to_string()]);
        table.data.push(vec!["25".to_string()]);
        table.data.push(vec!["35".to_string()]);
        table.col_types = vec![ColType::Number];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::LessThan,
            "28".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 1);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![1]);
    }

    #[test]
    fn test_filter_greater_or_equal() {
        let mut table = CsvTable::new();
        table.data.push(vec!["30".to_string()]);
        table.data.push(vec!["25".to_string()]);
        table.data.push(vec!["30".to_string()]);
        table.col_types = vec![ColType::Number];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::GreaterOrEqual,
            "30".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 2);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![0, 2]);
    }

    #[test]
    fn test_filter_less_or_equal() {
        let mut table = CsvTable::new();
        table.data.push(vec!["30".to_string()]);
        table.data.push(vec!["25".to_string()]);
        table.data.push(vec!["25".to_string()]);
        table.col_types = vec![ColType::Number];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::LessOrEqual,
            "25".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 2);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![1, 2]);
    }

    #[test]
    fn test_filter_multiple_conditions_all() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string(), "30".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string()]);
        table.data.push(vec!["Alice".to_string(), "25".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number];

        let conditions = vec![
            FilterCondition::new(0, FilterOperator::Equals, "Alice".to_string()),
            FilterCondition::new(1, FilterOperator::GreaterThan, "26".to_string()),
        ];
        table.apply_filters(&conditions, FilterLogic::All);

        assert_eq!(table.visible_row_count(), 1);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![0]);
    }

    #[test]
    fn test_filter_multiple_conditions_any() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string(), "30".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string()]);
        table.data.push(vec!["Charlie".to_string(), "35".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number];

        let conditions = vec![
            FilterCondition::new(0, FilterOperator::Equals, "Alice".to_string()),
            FilterCondition::new(1, FilterOperator::GreaterThan, "32".to_string()),
        ];
        table.apply_filters(&conditions, FilterLogic::Any);

        assert_eq!(table.visible_row_count(), 2);
        let visible: Vec<usize> = table.visible_indices().collect();
        assert_eq!(visible, vec![0, 2]);
    }

    #[test]
    fn test_filter_empty_conditions() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string()]);
        table.data.push(vec!["Bob".to_string()]);
        table.col_types = vec![ColType::Text];
        table.filtered_indices = Some(vec![0]);

        table.apply_filters(&[], FilterLogic::All);

        assert!(!table.is_filtered());
        assert_eq!(table.visible_row_count(), 2);
    }

    #[test]
    fn test_filter_no_matches() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string()]);
        table.data.push(vec!["Bob".to_string()]);
        table.col_types = vec![ColType::Text];

        let conditions = vec![FilterCondition::new(
            0,
            FilterOperator::Equals,
            "Charlie".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        // No filter set when no rows match (treated as "show nothing" via empty filtered_indices)
        assert!(!table.is_filtered());
        assert_eq!(table.visible_row_count(), 2);
    }

    #[test]
    fn test_filter_all_rows_match() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string(), "30".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number];

        let conditions = vec![FilterCondition::new(
            1,
            FilterOperator::GreaterThan,
            "20".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        // No filter set when all rows match
        assert!(!table.is_filtered());
        assert_eq!(table.visible_row_count(), 2);
    }

    #[test]
    fn test_filter_invalid_column() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string()]);
        table.data.push(vec!["Bob".to_string()]);
        table.col_types = vec![ColType::Text];

        let conditions = vec![FilterCondition::new(
            5,
            FilterOperator::Equals,
            "test".to_string(),
        )];
        table.apply_filters(&conditions, FilterLogic::All);

        // Invalid column means no rows match
        assert!(!table.is_filtered());
        assert_eq!(table.visible_row_count(), 2);
    }

    // Edit and Undo/Redo tests
    #[test]
    fn test_edit_single_cell() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string(), "30".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number];

        assert_eq!(table.get_cell(0, 0), Some("Alice"));
        table.set_cell(0, 0, "Bob".to_string());
        assert_eq!(table.get_cell(0, 0), Some("Bob"));
    }

    #[test]
    fn test_undo_single_edit() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string()]);
        table.col_types = vec![ColType::Text];

        assert!(!table.can_undo());
        table.set_cell(0, 0, "Bob".to_string());
        assert!(table.can_undo());

        assert_eq!(table.get_cell(0, 0), Some("Bob"));
        table.undo().unwrap();
        assert_eq!(table.get_cell(0, 0), Some("Alice"));
    }

    #[test]
    fn test_redo_single_edit() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string()]);
        table.col_types = vec![ColType::Text];

        table.set_cell(0, 0, "Bob".to_string());
        assert!(!table.can_redo());

        table.undo().unwrap();
        assert!(table.can_redo());

        assert_eq!(table.get_cell(0, 0), Some("Alice"));
        table.redo().unwrap();
        assert_eq!(table.get_cell(0, 0), Some("Bob"));
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut table = CsvTable::new();
        table.data.push(vec!["A".to_string()]);
        table.col_types = vec![ColType::Text];

        table.set_cell(0, 0, "B".to_string());
        table.set_cell(0, 0, "C".to_string());
        table.set_cell(0, 0, "D".to_string());

        assert_eq!(table.get_cell(0, 0), Some("D"));

        table.undo().unwrap();
        assert_eq!(table.get_cell(0, 0), Some("C"));

        table.undo().unwrap();
        assert_eq!(table.get_cell(0, 0), Some("B"));

        table.undo().unwrap();
        assert_eq!(table.get_cell(0, 0), Some("A"));

        // Redo
        table.redo().unwrap();
        assert_eq!(table.get_cell(0, 0), Some("B"));

        table.redo().unwrap();
        assert_eq!(table.get_cell(0, 0), Some("C"));
    }

    #[test]
    fn test_new_edit_clears_redo_stack() {
        let mut table = CsvTable::new();
        table.data.push(vec!["A".to_string()]);
        table.col_types = vec![ColType::Text];

        table.set_cell(0, 0, "B".to_string());
        table.set_cell(0, 0, "C".to_string());

        table.undo().unwrap();
        assert!(table.can_redo());

        // New edit should clear redo stack
        table.set_cell(0, 0, "D".to_string());
        assert!(!table.can_redo());
    }

    #[test]
    fn test_multi_cell_edit() {
        let mut table = CsvTable::new();
        table.data.push(vec!["A".to_string(), "1".to_string()]);
        table.data.push(vec!["B".to_string(), "2".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number];

        let changes = vec![
            (0, 0, "X".to_string()),
            (0, 1, "10".to_string()),
            (1, 0, "Y".to_string()),
        ];

        let count = table.set_cells(changes);
        assert_eq!(count, 3);

        assert_eq!(table.get_cell(0, 0), Some("X"));
        assert_eq!(table.get_cell(0, 1), Some("10"));
        assert_eq!(table.get_cell(1, 0), Some("Y"));
    }

    #[test]
    fn test_undo_multi_cell_edit() {
        let mut table = CsvTable::new();
        table.data.push(vec!["A".to_string(), "1".to_string()]);
        table.data.push(vec!["B".to_string(), "2".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number];

        let changes = vec![
            (0, 0, "X".to_string()),
            (0, 1, "10".to_string()),
            (1, 0, "Y".to_string()),
        ];

        table.set_cells(changes);

        // Undo should revert all changes at once
        table.undo().unwrap();

        assert_eq!(table.get_cell(0, 0), Some("A"));
        assert_eq!(table.get_cell(0, 1), Some("1"));
        assert_eq!(table.get_cell(1, 0), Some("B"));
    }

    #[test]
    fn test_redo_multi_cell_edit() {
        let mut table = CsvTable::new();
        table.data.push(vec!["A".to_string(), "1".to_string()]);
        table.data.push(vec!["B".to_string(), "2".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number];

        let changes = vec![
            (0, 0, "X".to_string()),
            (0, 1, "10".to_string()),
            (1, 0, "Y".to_string()),
        ];

        table.set_cells(changes);
        table.undo().unwrap();

        // Redo should reapply all changes at once
        table.redo().unwrap();

        assert_eq!(table.get_cell(0, 0), Some("X"));
        assert_eq!(table.get_cell(0, 1), Some("10"));
        assert_eq!(table.get_cell(1, 0), Some("Y"));
    }

    #[test]
    fn test_history_limit() {
        let mut table = CsvTable::new();
        table.data.push(vec!["A".to_string()]);
        table.col_types = vec![ColType::Text];
        table.history = History::new(3); // Limit to 3 actions

        table.set_cell(0, 0, "B".to_string());
        table.set_cell(0, 0, "C".to_string());
        table.set_cell(0, 0, "D".to_string());
        table.set_cell(0, 0, "E".to_string()); // This pushes out the first action

        assert_eq!(table.get_cell(0, 0), Some("E"));

        // Can only undo 3 times (not 4)
        table.undo().unwrap();
        table.undo().unwrap();
        table.undo().unwrap();

        // Should be at "B" now, not "A"
        assert_eq!(table.get_cell(0, 0), Some("B"));

        // Can't undo anymore
        assert!(!table.can_undo());
    }

    #[test]
    fn test_undo_redo_with_no_changes() {
        let mut table = CsvTable::new();
        table.data.push(vec!["A".to_string()]);
        table.col_types = vec![ColType::Text];

        assert!(table.undo().is_err());
        assert!(table.redo().is_err());
    }

    #[test]
    fn test_edit_and_undo_preserves_row_integrity() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Number, ColType::Text];

        table.set_cell(0, 1, "25".to_string());

        assert_eq!(table.get_cell(0, 0), Some("Alice"));
        assert_eq!(table.get_cell(0, 1), Some("25"));
        assert_eq!(table.get_cell(0, 2), Some("NYC"));

        table.undo().unwrap();

        assert_eq!(table.get_cell(0, 0), Some("Alice"));
        assert_eq!(table.get_cell(0, 1), Some("30"));
        assert_eq!(table.get_cell(0, 2), Some("NYC"));
    }

    #[test]
    fn test_grouped_edit_is_single_undo_action() {
        let mut table = CsvTable::new();
        table.data.push(vec!["A".to_string(), "B".to_string()]);
        table.col_types = vec![ColType::Text, ColType::Text];

        let changes = vec![(0, 0, "X".to_string()), (0, 1, "Y".to_string())];
        table.set_cells(changes);

        assert_eq!(table.get_cell(0, 0), Some("X"));
        assert_eq!(table.get_cell(0, 1), Some("Y"));

        // Single undo should revert both changes
        table.undo().unwrap();

        assert_eq!(table.get_cell(0, 0), Some("A"));
        assert_eq!(table.get_cell(0, 1), Some("B"));

        // No more undo actions
        assert!(!table.can_undo());
    }

    // JSON export tests
    #[test]
    fn test_json_export_with_headers() {
        let mut table = CsvTable::with_headers(vec![
            "Name".to_string(),
            "Age".to_string(),
            "City".to_string(),
        ]);
        table.data.push(vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string(), "LA".to_string()]);

        let json = table.to_json(false).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_array());
        let array = parsed.as_array().unwrap();
        assert_eq!(array.len(), 2);

        // Check first row
        assert_eq!(array[0]["Name"], "Alice");
        assert_eq!(array[0]["Age"], "30");
        assert_eq!(array[0]["City"], "NYC");

        // Check second row
        assert_eq!(array[1]["Name"], "Bob");
        assert_eq!(array[1]["Age"], "25");
        assert_eq!(array[1]["City"], "LA");
    }

    #[test]
    fn test_json_export_without_headers() {
        let mut table = CsvTable::new();
        table.data.push(vec!["Alice".to_string(), "30".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string()]);

        let json = table.to_json(false).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_array());
        let array = parsed.as_array().unwrap();
        assert_eq!(array.len(), 2);

        // Should use generic column names
        assert_eq!(array[0]["col0"], "Alice");
        assert_eq!(array[0]["col1"], "30");
        assert_eq!(array[1]["col0"], "Bob");
        assert_eq!(array[1]["col1"], "25");
    }

    #[test]
    fn test_json_export_empty_table() {
        let table = CsvTable::new();
        let json = table.to_json(false).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_array());
        let array = parsed.as_array().unwrap();
        assert_eq!(array.len(), 0);
    }

    #[test]
    fn test_json_export_pretty() {
        let mut table = CsvTable::with_headers(vec!["Name".to_string()]);
        table.data.push(vec!["Alice".to_string()]);

        let json = table.to_json(true).unwrap();

        // Pretty JSON should have newlines and indentation
        assert!(json.contains('\n'));
        assert!(json.contains("  "));
    }

    #[test]
    fn test_json_export_special_characters() {
        let mut table = CsvTable::with_headers(vec!["Text".to_string()]);
        table.data.push(vec!["Hello \"World\"".to_string()]);
        table.data.push(vec!["Line1\nLine2".to_string()]);

        let json = table.to_json(false).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        let array = parsed.as_array().unwrap();
        assert_eq!(array[0]["Text"], "Hello \"World\"");
        assert_eq!(array[1]["Text"], "Line1\nLine2");
    }
}
