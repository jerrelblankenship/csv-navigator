slint::include_modules!();

use slint::{ModelRc, SharedString, VecModel};
use std::rc::Rc;

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    // Sample data to display as a CSV
    let sample_headers = vec![
        SharedString::from("Name"),
        SharedString::from("Age"),
        SharedString::from("City"),
        SharedString::from("Occupation"),
    ];

    let sample_data = vec![
        vec![
            SharedString::from("Alice"),
            SharedString::from("32"),
            SharedString::from("New York"),
            SharedString::from("Engineer"),
        ],
        vec![
            SharedString::from("Bob"),
            SharedString::from("28"),
            SharedString::from("San Francisco"),
            SharedString::from("Designer"),
        ],
        vec![
            SharedString::from("Carol"),
            SharedString::from("45"),
            SharedString::from("Chicago"),
            SharedString::from("Manager"),
        ],
        vec![
            SharedString::from("Dave"),
            SharedString::from("33"),
            SharedString::from("Boston"),
            SharedString::from("Developer"),
        ],
        vec![
            SharedString::from("Eve"),
            SharedString::from("29"),
            SharedString::from("Seattle"),
            SharedString::from("Analyst"),
        ],
    ];

    // Set up our model data
    let headers_model = Rc::new(VecModel::from(sample_headers));
    ui.set_column_headers(ModelRc::from(headers_model));

    // Create the rows model
    let mut rows_vec = Vec::new();
    for row in sample_data {
        let row_model = Rc::new(VecModel::from(row));
        rows_vec.push(ModelRc::from(row_model));
    }

    let rows_model = Rc::new(VecModel::from(rows_vec));
    ui.set_csv_data(ModelRc::from(rows_model));

    // Handle file open button
    let ui_handle = ui.as_weak();
    ui.on_open_file(move || {
        let ui = ui_handle.unwrap();
        ui.set_status_message(SharedString::from(
            "File open button clicked! (File dialog would open here)",
        ));
    });

    // Handle save button
    let ui_handle = ui.as_weak();
    ui.on_save_file(move || {
        let ui = ui_handle.unwrap();
        ui.set_status_message(SharedString::from(
            "Save button clicked! (Save functionality would execute here)",
        ));
    });

    ui.run()
}
