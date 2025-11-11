/// UI module for CSV Navigator
/// Provides the glue code between Rust backend and Slint frontend

use crate::data::{CsvTable, FilterCondition, FilterLogic, FilterOperator, SortOrder};
use crate::AppWindow;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

/// Converts a FilterOperator enum to a string for UI display
fn operator_to_string(op: FilterOperator) -> &'static str {
    match op {
        FilterOperator::Equals => "Equals",
        FilterOperator::NotEquals => "NotEquals",
        FilterOperator::Contains => "Contains",
        FilterOperator::NotContains => "NotContains",
        FilterOperator::StartsWith => "StartsWith",
        FilterOperator::EndsWith => "EndsWith",
        FilterOperator::GreaterThan => "GreaterThan",
        FilterOperator::LessThan => "LessThan",
        FilterOperator::GreaterOrEqual => "GreaterOrEqual",
        FilterOperator::LessOrEqual => "LessOrEqual",
    }
}

/// Converts a string from UI to a FilterOperator enum
fn string_to_operator(s: &str) -> FilterOperator {
    match s {
        "Equals" => FilterOperator::Equals,
        "NotEquals" => FilterOperator::NotEquals,
        "Contains" => FilterOperator::Contains,
        "NotContains" => FilterOperator::NotContains,
        "StartsWith" => FilterOperator::StartsWith,
        "EndsWith" => FilterOperator::EndsWith,
        "GreaterThan" => FilterOperator::GreaterThan,
        "LessThan" => FilterOperator::LessThan,
        "GreaterOrEqual" => FilterOperator::GreaterOrEqual,
        "LessOrEqual" => FilterOperator::LessOrEqual,
        _ => FilterOperator::Equals,
    }
}

/// Updates the UI with data from the CsvTable
fn update_table_display(ui: &AppWindow, table: &CsvTable) {
    // Update headers
    if let Some(headers) = &table.headers {
        let header_model: Vec<SharedString> = headers
            .iter()
            .map(|h| SharedString::from(h.as_str()))
            .collect();
        ui.set_column_headers(ModelRc::new(VecModel::from(header_model)));
    }

    // Update visible data (respects filtering)
    let mut row_data: Vec<slint::ModelRc<SharedString>> = Vec::new();

    for row_index in table.visible_indices() {
        if row_index < table.data.len() {
            let row_strings: Vec<SharedString> = table.data[row_index]
                .iter()
                .map(|cell| SharedString::from(cell.as_str()))
                .collect();
            row_data.push(ModelRc::new(VecModel::from(row_strings)));
        }
    }

    ui.set_csv_data(ModelRc::new(VecModel::from(row_data)));

    // Update status
    ui.set_total_rows(table.row_count() as i32);
    ui.set_filtered_rows(table.visible_row_count() as i32);
    ui.set_has_filter(table.is_filtered());
}

/// Sets up all callbacks for the UI
pub fn setup_callbacks(ui: &AppWindow) {
    let table: Rc<RefCell<CsvTable>> = Rc::new(RefCell::new(CsvTable::new()));
    let current_file: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

    // Open file callback
    {
        let ui_weak = ui.as_weak();
        let table = table.clone();
        let current_file = current_file.clone();

        ui.on_open_file(move || {
            let ui = ui_weak.unwrap();

            // Use native file dialog
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CSV Files", &["csv"])
                    .pick_file()
                {
                    match CsvTable::from_path(&path, true) {
                        Ok(loaded_table) => {
                            *table.borrow_mut() = loaded_table;
                            *current_file.borrow_mut() = Some(path.clone());
                            update_table_display(&ui, &table.borrow());
                            ui.set_status_message(SharedString::from(format!(
                                "Loaded: {}",
                                path.file_name().unwrap().to_string_lossy()
                            )));
                        }
                        Err(e) => {
                            ui.set_status_message(SharedString::from(format!(
                                "Error loading file: {}",
                                e
                            )));
                        }
                    }
                }
            }
        });
    }

    // Save file callback
    {
        let ui_weak = ui.as_weak();
        let table = table.clone();
        let current_file = current_file.clone();

        ui.on_save_file(move || {
            let ui = ui_weak.unwrap();

            let path_to_save = if let Some(ref path) = *current_file.borrow() {
                Some(path.clone())
            } else {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    rfd::FileDialog::new()
                        .add_filter("CSV Files", &["csv"])
                        .save_file()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    None
                }
            };

            if let Some(path) = path_to_save {
                match table.borrow().save_to_path(&path) {
                    Ok(_) => {
                        *current_file.borrow_mut() = Some(path.clone());
                        ui.set_status_message(SharedString::from(format!(
                            "Saved: {}",
                            path.file_name().unwrap().to_string_lossy()
                        )));
                    }
                    Err(e) => {
                        ui.set_status_message(SharedString::from(format!("Error saving: {}", e)));
                    }
                }
            }
        });
    }

    // Import XLSX callback
    {
        let ui_weak = ui.as_weak();
        let table = table.clone();
        let current_file = current_file.clone();

        ui.on_import_xlsx(move || {
            let ui = ui_weak.unwrap();

            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Excel Files", &["xlsx"])
                    .pick_file()
                {
                    match CsvTable::from_xlsx_file(&path, true) {
                        Ok(loaded_table) => {
                            *table.borrow_mut() = loaded_table;
                            *current_file.borrow_mut() = None; // Clear current file since it's from XLSX
                            update_table_display(&ui, &table.borrow());
                            ui.set_status_message(SharedString::from(format!(
                                "Imported from XLSX: {}",
                                path.file_name().unwrap().to_string_lossy()
                            )));
                        }
                        Err(e) => {
                            ui.set_status_message(SharedString::from(format!(
                                "Error importing XLSX: {}",
                                e
                            )));
                        }
                    }
                }
            }
        });
    }

    // Export XLSX callback
    {
        let ui_weak = ui.as_weak();
        let table = table.clone();

        ui.on_export_xlsx(move || {
            let ui = ui_weak.unwrap();

            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Excel Files", &["xlsx"])
                    .save_file()
                {
                    match table.borrow().to_xlsx_file(&path) {
                        Ok(_) => {
                            ui.set_status_message(SharedString::from(format!(
                                "Exported to XLSX: {}",
                                path.file_name().unwrap().to_string_lossy()
                            )));
                        }
                        Err(e) => {
                            ui.set_status_message(SharedString::from(format!(
                                "Error exporting XLSX: {}",
                                e
                            )));
                        }
                    }
                }
            }
        });
    }

    // Export JSON callback
    {
        let ui_weak = ui.as_weak();
        let table = table.clone();

        ui.on_export_json(move || {
            let ui = ui_weak.unwrap();

            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON Files", &["json"])
                    .save_file()
                {
                    match table.borrow().to_json_file(&path, true) {
                        Ok(_) => {
                            ui.set_status_message(SharedString::from(format!(
                                "Exported to JSON: {}",
                                path.file_name().unwrap().to_string_lossy()
                            )));
                        }
                        Err(e) => {
                            ui.set_status_message(SharedString::from(format!(
                                "Error exporting JSON: {}",
                                e
                            )));
                        }
                    }
                }
            }
        });
    }

    // Apply filter callback
    {
        let ui_weak = ui.as_weak();
        let table = table.clone();

        ui.on_apply_filter(move || {
            let ui = ui_weak.unwrap();

            let col_index = ui.get_filter_column_index();
            let operator_str = ui.get_filter_operator();
            let value = ui.get_filter_value();
            let logic_all = ui.get_filter_logic_all();

            if col_index >= 0 && !value.is_empty() {
                let operator = string_to_operator(&operator_str);
                let filter =
                    FilterCondition::new(col_index as usize, operator, value.to_string());
                let logic = if logic_all {
                    FilterLogic::All
                } else {
                    FilterLogic::Any
                };

                table.borrow_mut().apply_filters(&[filter], logic);
                update_table_display(&ui, &table.borrow());
                ui.set_status_message(SharedString::from("Filter applied"));
            }
        });
    }

    // Clear filter callback
    {
        let ui_weak = ui.as_weak();
        let table = table.clone();

        ui.on_clear_filter(move || {
            let ui = ui_weak.unwrap();
            table.borrow_mut().clear_filter();
            update_table_display(&ui, &table.borrow());
            ui.set_status_message(SharedString::from("Filter cleared"));
        });
    }

    // Sort column callback
    {
        let ui_weak = ui.as_weak();
        let table = table.clone();

        ui.on_sort_column(move |column_index| {
            let ui = ui_weak.unwrap();

            if column_index >= 0 {
                let current_sort_col = ui.get_sort_column_index();
                let current_sort_asc = ui.get_sort_ascending();

                // Toggle sort order if clicking the same column
                let sort_order = if current_sort_col == column_index && current_sort_asc {
                    SortOrder::Descending
                } else {
                    SortOrder::Ascending
                };

                match table.borrow_mut().sort_by_column(column_index as usize, sort_order) {
                    Ok(_) => {
                        ui.set_sort_column_index(column_index);
                        ui.set_sort_ascending(matches!(sort_order, SortOrder::Ascending));
                        update_table_display(&ui, &table.borrow());
                        ui.set_status_message(SharedString::from(format!(
                            "Sorted by column {}",
                            column_index
                        )));
                    }
                    Err(e) => {
                        ui.set_status_message(SharedString::from(format!("Sort error: {}", e)));
                    }
                }
            }
        });
    }
}
