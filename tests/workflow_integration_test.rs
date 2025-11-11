/// Integration tests for complete CSV Navigator workflows
/// Tests the end-to-end operations: load -> sort -> filter -> edit -> save -> export

#[cfg(test)]
mod workflow_integration_tests {
    use csv_navigator::data::{CsvTable, FilterCondition, FilterLogic, FilterOperator, SortOrder};
    use tempfile::tempdir;

    /// Test complete workflow: Load CSV -> Sort -> Filter -> Edit -> Save
    #[test]
    fn test_complete_csv_workflow() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("input.csv");
        let output_path = temp_dir.path().join("output.csv");

        // Create initial CSV file
        let mut table = CsvTable::with_headers(vec![
            "Name".to_string(),
            "Age".to_string(),
            "Score".to_string(),
        ]);
        table.data.push(vec!["Charlie".to_string(), "35".to_string(), "85".to_string()]);
        table.data.push(vec!["Alice".to_string(), "30".to_string(), "95".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string(), "75".to_string()]);
        table.save_to_path(&input_path).unwrap();

        // Load CSV
        let mut loaded = CsvTable::from_path(&input_path, true).unwrap();
        assert_eq!(loaded.row_count(), 3);

        // Sort by Age (ascending)
        loaded.sort_by_column(1, SortOrder::Ascending).unwrap();
        assert_eq!(loaded.data[0][0], "Bob");  // Bob is 25
        assert_eq!(loaded.data[1][0], "Alice"); // Alice is 30
        assert_eq!(loaded.data[2][0], "Charlie"); // Charlie is 35

        // Filter: Score >= 80
        let filter = FilterCondition::new(2, FilterOperator::GreaterOrEqual, "80".to_string());
        loaded.apply_filters(&[filter], FilterLogic::All);
        assert_eq!(loaded.visible_row_count(), 2); // Alice (95) and Charlie (85)

        // Clear filter before editing (editing works on actual indices, not filtered)
        loaded.clear_filter();
        assert_eq!(loaded.visible_row_count(), 3);

        // Edit a cell - after sorting, Bob is row 0, Alice is row 1, Charlie is row 2
        loaded.set_cell(2, 2, "88".to_string()); // Change Charlie's score

        // Save modified data
        loaded.save_to_path(&output_path).unwrap();

        // Verify saved file
        let final_table = CsvTable::from_path(&output_path, true).unwrap();
        assert_eq!(final_table.row_count(), 3);
        // Charlie's score should be updated to 88
        assert_eq!(final_table.data[2][2], "88");
    }

    /// Test workflow with undo/redo operations
    #[test]
    fn test_workflow_with_undo_redo() {
        let mut table = CsvTable::with_headers(vec!["Value".to_string()]);
        table.data.push(vec!["10".to_string()]);
        table.data.push(vec!["20".to_string()]);

        // Edit cell
        table.set_cell(0, 0, "15".to_string());
        assert_eq!(table.data[0][0], "15");

        // Undo
        table.undo().unwrap();
        assert_eq!(table.data[0][0], "10");

        // Redo
        table.redo().unwrap();
        assert_eq!(table.data[0][0], "15");

        // Make another edit
        table.set_cell(1, 0, "25".to_string());
        assert_eq!(table.data[1][0], "25");

        // Undo twice
        table.undo().unwrap();
        table.undo().unwrap();
        assert_eq!(table.data[0][0], "10");
        assert_eq!(table.data[1][0], "20");
    }

    /// Test workflow with multiple filters (AND logic)
    #[test]
    fn test_workflow_multiple_filters_and() {
        let mut table = CsvTable::with_headers(vec![
            "Name".to_string(),
            "Age".to_string(),
            "Department".to_string(),
        ]);
        table.data.push(vec!["Alice".to_string(), "30".to_string(), "Engineering".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string(), "Engineering".to_string()]);
        table.data.push(vec!["Charlie".to_string(), "35".to_string(), "Sales".to_string()]);
        table.data.push(vec!["Diana".to_string(), "28".to_string(), "Engineering".to_string()]);

        // Filter: Age >= 28 AND Department = Engineering
        let filters = vec![
            FilterCondition::new(1, FilterOperator::GreaterOrEqual, "28".to_string()),
            FilterCondition::new(2, FilterOperator::Equals, "Engineering".to_string()),
        ];

        table.apply_filters(&filters, FilterLogic::All);

        // Should match Alice (30, Engineering) and Diana (28, Engineering)
        assert_eq!(table.visible_row_count(), 2);

        let visible_names: Vec<String> = table
            .visible_indices()
            .map(|idx| table.data[idx][0].clone())
            .collect();
        assert!(visible_names.contains(&"Alice".to_string()));
        assert!(visible_names.contains(&"Diana".to_string()));
    }

    /// Test workflow with multiple filters (OR logic)
    #[test]
    fn test_workflow_multiple_filters_or() {
        let mut table = CsvTable::with_headers(vec![
            "Product".to_string(),
            "Price".to_string(),
            "Category".to_string(),
        ]);
        table.data.push(vec!["Widget".to_string(), "10".to_string(), "Tools".to_string()]);
        table.data.push(vec!["Gadget".to_string(), "150".to_string(), "Electronics".to_string()]);
        table.data.push(vec!["Doohickey".to_string(), "5".to_string(), "Tools".to_string()]);
        table.data.push(vec!["Gizmo".to_string(), "200".to_string(), "Electronics".to_string()]);

        // Filter: Price > 100 OR Category = Tools
        let filters = vec![
            FilterCondition::new(1, FilterOperator::GreaterThan, "100".to_string()),
            FilterCondition::new(2, FilterOperator::Equals, "Tools".to_string()),
        ];

        table.apply_filters(&filters, FilterLogic::Any);

        // Should match: Widget (Tools), Gadget (150), Doohickey (Tools), Gizmo (200)
        assert_eq!(table.visible_row_count(), 4);
    }

    /// Test full export workflow: CSV -> Sort -> Filter -> Export JSON
    #[test]
    fn test_csv_to_json_workflow() {
        let temp_dir = tempdir().unwrap();
        let csv_path = temp_dir.path().join("data.csv");
        let json_path = temp_dir.path().join("data.json");

        // Create and save CSV
        let mut table = CsvTable::with_headers(vec!["Name".to_string(), "Score".to_string()]);
        table.data.push(vec!["Charlie".to_string(), "85".to_string()]);
        table.data.push(vec!["Alice".to_string(), "95".to_string()]);
        table.data.push(vec!["Bob".to_string(), "75".to_string()]);
        table.save_to_path(&csv_path).unwrap();

        // Load and sort
        let mut loaded = CsvTable::from_path(&csv_path, true).unwrap();
        loaded.sort_by_column(1, SortOrder::Descending).unwrap(); // Sort by Score descending

        // Filter: Score > 80
        let filter = FilterCondition::new(1, FilterOperator::GreaterThan, "80".to_string());
        loaded.apply_filters(&[filter], FilterLogic::All);

        // Export to JSON
        loaded.to_json_file(&json_path, true).unwrap();

        // Verify JSON file was created
        assert!(json_path.exists());

        // Read and parse JSON
        let json_content = std::fs::read_to_string(&json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_content).unwrap();

        // Note: JSON export includes all data, not just filtered
        assert!(parsed.is_array());
    }

    /// Test multi-cell editing workflow
    #[test]
    fn test_workflow_multi_cell_edit() {
        let mut table = CsvTable::with_headers(vec!["A".to_string(), "B".to_string()]);
        table.data.push(vec!["1".to_string(), "2".to_string()]);
        table.data.push(vec!["3".to_string(), "4".to_string()]);
        table.data.push(vec!["5".to_string(), "6".to_string()]);

        // Multi-cell edit
        let changes = vec![
            (0, 0, "10".to_string()),
            (1, 1, "40".to_string()),
            (2, 0, "50".to_string()),
        ];

        let count = table.set_cells(changes);
        assert_eq!(count, 3);

        // Verify changes
        assert_eq!(table.data[0][0], "10");
        assert_eq!(table.data[1][1], "40");
        assert_eq!(table.data[2][0], "50");

        // Undo should revert all changes at once
        table.undo().unwrap();
        assert_eq!(table.data[0][0], "1");
        assert_eq!(table.data[1][1], "4");
        assert_eq!(table.data[2][0], "5");
    }

    /// Test sorting after filtering
    #[test]
    fn test_workflow_sort_after_filter() {
        let mut table = CsvTable::with_headers(vec!["Name".to_string(), "Age".to_string()]);
        table.data.push(vec!["Alice".to_string(), "30".to_string()]);
        table.data.push(vec!["Bob".to_string(), "25".to_string()]);
        table.data.push(vec!["Charlie".to_string(), "35".to_string()]);
        table.data.push(vec!["Diana".to_string(), "28".to_string()]);

        // Filter: Age > 27
        let filter = FilterCondition::new(1, FilterOperator::GreaterThan, "27".to_string());
        table.apply_filters(&[filter], FilterLogic::All);
        assert_eq!(table.visible_row_count(), 3); // Alice, Charlie, Diana

        // Sort by Age - should clear filter
        table.sort_by_column(1, SortOrder::Ascending).unwrap();

        // After sort, filter should be cleared
        assert!(!table.is_filtered());
        assert_eq!(table.visible_row_count(), 4);

        // Verify sorted order
        assert_eq!(table.data[0][0], "Bob");    // 25
        assert_eq!(table.data[1][0], "Diana");  // 28
        assert_eq!(table.data[2][0], "Alice");  // 30
        assert_eq!(table.data[3][0], "Charlie"); // 35
    }

    /// Test column type inference workflow
    #[test]
    fn test_workflow_with_type_inference() {
        let temp_dir = tempdir().unwrap();
        let csv_path = temp_dir.path().join("typed_data.csv");

        // Create CSV with mixed types
        let mut table = CsvTable::with_headers(vec!["Name".to_string(), "Count".to_string(), "Value".to_string()]);
        table.data.push(vec!["Item1".to_string(), "100".to_string(), "12.5".to_string()]);
        table.data.push(vec!["Item2".to_string(), "200".to_string(), "25.3".to_string()]);
        table.data.push(vec!["Item3".to_string(), "150".to_string(), "18.7".to_string()]);
        table.save_to_path(&csv_path).unwrap();

        // Load with type inference
        let mut loaded = CsvTable::from_path(&csv_path, true).unwrap();

        // Column types should be inferred
        assert_eq!(loaded.col_types.len(), 3);
        // Name should be Text, Count and Value should be Number
        use csv_navigator::data::ColType;
        assert_eq!(loaded.col_types[0], ColType::Text);
        assert_eq!(loaded.col_types[1], ColType::Number);
        assert_eq!(loaded.col_types[2], ColType::Number);

        // Sort by numeric column should work correctly
        loaded.sort_by_column(1, SortOrder::Ascending).unwrap();
        assert_eq!(loaded.data[0][1], "100");
        assert_eq!(loaded.data[1][1], "150");
        assert_eq!(loaded.data[2][1], "200");
    }

    /// Test complete round-trip with all operations
    #[test]
    fn test_complete_round_trip_all_operations() {
        let temp_dir = tempdir().unwrap();
        let csv_path1 = temp_dir.path().join("original.csv");
        let xlsx_path = temp_dir.path().join("exported.xlsx");
        let csv_path2 = temp_dir.path().join("final.csv");

        // 1. Create initial data
        let mut table = CsvTable::with_headers(vec![
            "ID".to_string(),
            "Name".to_string(),
            "Value".to_string(),
        ]);
        table.data.push(vec!["3".to_string(), "Charlie".to_string(), "300".to_string()]);
        table.data.push(vec!["1".to_string(), "Alice".to_string(), "100".to_string()]);
        table.data.push(vec!["2".to_string(), "Bob".to_string(), "200".to_string()]);

        // 2. Save to CSV
        table.save_to_path(&csv_path1).unwrap();

        // 3. Load from CSV
        let mut loaded = CsvTable::from_path(&csv_path1, true).unwrap();

        // 4. Sort by ID
        loaded.sort_by_column(0, SortOrder::Ascending).unwrap();

        // 5. Edit a value
        loaded.set_cell(1, 2, "250".to_string()); // Change Bob's value

        // 6. Export to XLSX
        loaded.to_xlsx_file(&xlsx_path).unwrap();

        // 7. Import from XLSX
        let from_xlsx = CsvTable::from_xlsx_file(&xlsx_path, true).unwrap();

        // 8. Save back to CSV
        from_xlsx.save_to_path(&csv_path2).unwrap();

        // 9. Load final CSV and verify
        let final_table = CsvTable::from_path(&csv_path2, true).unwrap();

        // Verify data integrity through all operations
        assert_eq!(final_table.headers, loaded.headers);
        assert_eq!(final_table.row_count(), 3);
        // Bob's value should be 250
        assert_eq!(final_table.data[1][0], "2");
        assert_eq!(final_table.data[1][1], "Bob");
        assert_eq!(final_table.data[1][2], "250");
    }

    /// Test performance with moderately sized dataset
    #[test]
    fn test_workflow_moderate_dataset() {
        let mut table = CsvTable::with_headers(vec![
            "ID".to_string(),
            "Value".to_string(),
            "Category".to_string(),
        ]);

        // Add 5000 rows
        for i in 0..5000 {
            table.data.push(vec![
                i.to_string(),
                (i % 100).to_string(),
                if i % 2 == 0 { "Even".to_string() } else { "Odd".to_string() },
            ]);
        }

        // Sort
        table.sort_by_column(1, SortOrder::Ascending).unwrap();

        // Filter
        let filter = FilterCondition::new(2, FilterOperator::Equals, "Even".to_string());
        table.apply_filters(&[filter], FilterLogic::All);
        assert_eq!(table.visible_row_count(), 2500);

        // Edit a cell
        table.set_cell(0, 1, "999".to_string());

        // Undo
        table.undo().unwrap();

        // All operations should complete quickly
        assert_eq!(table.row_count(), 5000);
    }
}
