/// Integration tests for Excel import/export operations
/// Tests the complete round-trip workflow and data integrity

#[cfg(test)]
mod excel_integration_tests {
    use csv_navigator::data::CsvTable;
    use std::fs;
    use tempfile::tempdir;

    /// Test complete round-trip: Create table -> Export XLSX -> Import XLSX -> Verify
    #[test]
    fn test_excel_round_trip_with_headers() {
        let temp_dir = tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("roundtrip_test.xlsx");

        // Create original table with sample data
        let mut original = CsvTable::with_headers(vec![
            "Name".to_string(),
            "Age".to_string(),
            "City".to_string(),
            "Score".to_string(),
        ]);

        original.data.push(vec![
            "Alice".to_string(),
            "30".to_string(),
            "New York".to_string(),
            "95.5".to_string(),
        ]);
        original.data.push(vec![
            "Bob".to_string(),
            "25".to_string(),
            "San Francisco".to_string(),
            "87.3".to_string(),
        ]);
        original.data.push(vec![
            "Charlie".to_string(),
            "35".to_string(),
            "Boston".to_string(),
            "92.8".to_string(),
        ]);

        // Export to XLSX
        original.to_xlsx_file(&xlsx_path).expect("Failed to export XLSX");

        // Import back from XLSX
        let imported = CsvTable::from_xlsx_file(&xlsx_path, true).expect("Failed to import XLSX");

        // Verify headers
        assert_eq!(imported.headers, original.headers);

        // Verify row count
        assert_eq!(imported.data.len(), original.data.len());

        // Verify all data
        for (i, row) in imported.data.iter().enumerate() {
            assert_eq!(row, &original.data[i], "Row {} mismatch", i);
        }
    }

    /// Test round-trip without headers
    #[test]
    fn test_excel_round_trip_without_headers() {
        let temp_dir = tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("roundtrip_no_headers.xlsx");

        // Create table without headers
        let mut original = CsvTable::new();
        original.data.push(vec!["Alice".to_string(), "30".to_string()]);
        original.data.push(vec!["Bob".to_string(), "25".to_string()]);

        // Export to XLSX
        original.to_xlsx_file(&xlsx_path).expect("Failed to export XLSX");

        // Import back without headers
        let imported = CsvTable::from_xlsx_file(&xlsx_path, false).expect("Failed to import XLSX");

        // Verify no headers
        assert_eq!(imported.headers, None);

        // Verify data
        assert_eq!(imported.data.len(), original.data.len());
        for (i, row) in imported.data.iter().enumerate() {
            assert_eq!(row, &original.data[i], "Row {} mismatch", i);
        }
    }

    /// Test round-trip preserves special characters
    #[test]
    fn test_excel_round_trip_special_characters() {
        let temp_dir = tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("special_chars.xlsx");

        let mut original = CsvTable::with_headers(vec!["Text".to_string(), "Symbols".to_string()]);
        original.data.push(vec![
            "Hello \"World\"".to_string(),
            "Special: @#$%^&*()".to_string(),
        ]);
        original.data.push(vec![
            "Line1\nLine2".to_string(),
            "Comma, semicolon;".to_string(),
        ]);

        // Export and import
        original.to_xlsx_file(&xlsx_path).unwrap();
        let imported = CsvTable::from_xlsx_file(&xlsx_path, true).unwrap();

        // Verify special characters preserved
        assert_eq!(imported.data[0][0], "Hello \"World\"");
        assert_eq!(imported.data[0][1], "Special: @#$%^&*()");
        assert_eq!(imported.data[1][0], "Line1\nLine2");
        assert_eq!(imported.data[1][1], "Comma, semicolon;");
    }

    /// Test round-trip with numeric values
    #[test]
    fn test_excel_round_trip_numeric_data() {
        let temp_dir = tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("numeric_data.xlsx");

        let mut original = CsvTable::with_headers(vec![
            "ID".to_string(),
            "Integer".to_string(),
            "Float".to_string(),
            "Negative".to_string(),
        ]);

        original.data.push(vec!["1".to_string(), "100".to_string(), "3.14159".to_string(), "-42".to_string()]);
        original.data.push(vec!["2".to_string(), "200".to_string(), "2.71828".to_string(), "-99".to_string()]);

        // Export and import
        original.to_xlsx_file(&xlsx_path).unwrap();
        let imported = CsvTable::from_xlsx_file(&xlsx_path, true).unwrap();

        // Verify numeric data preserved as strings
        assert_eq!(imported.data.len(), 2);
        assert_eq!(imported.data[0], original.data[0]);
        assert_eq!(imported.data[1], original.data[1]);
    }

    /// Test round-trip with large dataset
    #[test]
    fn test_excel_round_trip_large_dataset() {
        let temp_dir = tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("large_data.xlsx");

        // Create table with 10,000 rows
        let mut original = CsvTable::with_headers(vec![
            "ID".to_string(),
            "Name".to_string(),
            "Value".to_string(),
        ]);

        for i in 0..10_000 {
            original.data.push(vec![
                i.to_string(),
                format!("Person_{}", i),
                (i as f64 * 1.5).to_string(),
            ]);
        }

        // Export and import
        original.to_xlsx_file(&xlsx_path).unwrap();
        let imported = CsvTable::from_xlsx_file(&xlsx_path, true).unwrap();

        // Verify row count
        assert_eq!(imported.data.len(), 10_000);

        // Spot check some rows
        assert_eq!(imported.data[0], original.data[0]);
        assert_eq!(imported.data[5000], original.data[5000]);
        assert_eq!(imported.data[9999], original.data[9999]);
    }

    /// Test CSV to XLSX to CSV round-trip maintains data integrity
    #[test]
    fn test_csv_xlsx_csv_round_trip() {
        let temp_dir = tempdir().unwrap();
        let csv_path1 = temp_dir.path().join("original.csv");
        let xlsx_path = temp_dir.path().join("intermediate.xlsx");
        let csv_path2 = temp_dir.path().join("final.csv");

        // Create and save original CSV
        let mut original = CsvTable::with_headers(vec![
            "Product".to_string(),
            "Price".to_string(),
            "Quantity".to_string(),
        ]);
        original.data.push(vec!["Widget".to_string(), "19.99".to_string(), "100".to_string()]);
        original.data.push(vec!["Gadget".to_string(), "29.99".to_string(), "50".to_string()]);
        original.save_to_path(&csv_path1).unwrap();

        // Load CSV
        let from_csv = CsvTable::from_path(&csv_path1, true).unwrap();

        // Export to XLSX
        from_csv.to_xlsx_file(&xlsx_path).unwrap();

        // Import from XLSX
        let from_xlsx = CsvTable::from_xlsx_file(&xlsx_path, true).unwrap();

        // Export back to CSV
        from_xlsx.save_to_path(&csv_path2).unwrap();

        // Load final CSV
        let final_csv = CsvTable::from_path(&csv_path2, true).unwrap();

        // Verify data integrity through entire round-trip
        assert_eq!(final_csv.headers, original.headers);
        assert_eq!(final_csv.data, original.data);
    }

    /// Test importing and re-exporting maintains consistency
    #[test]
    fn test_import_export_consistency() {
        let temp_dir = tempdir().unwrap();
        let xlsx_path1 = temp_dir.path().join("export1.xlsx");
        let xlsx_path2 = temp_dir.path().join("export2.xlsx");

        // Create original table
        let mut table = CsvTable::with_headers(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
        table.data.push(vec!["1".to_string(), "2".to_string(), "3".to_string()]);
        table.data.push(vec!["4".to_string(), "5".to_string(), "6".to_string()]);

        // First export
        table.to_xlsx_file(&xlsx_path1).unwrap();

        // Import and re-export
        let imported = CsvTable::from_xlsx_file(&xlsx_path1, true).unwrap();
        imported.to_xlsx_file(&xlsx_path2).unwrap();

        // Import both and compare
        let table1 = CsvTable::from_xlsx_file(&xlsx_path1, true).unwrap();
        let table2 = CsvTable::from_xlsx_file(&xlsx_path2, true).unwrap();

        assert_eq!(table1.headers, table2.headers);
        assert_eq!(table1.data, table2.data);
    }

    /// Test empty table round-trip
    #[test]
    fn test_excel_round_trip_empty_table() {
        let temp_dir = tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("empty.xlsx");

        let original = CsvTable::new();

        // Export empty table
        original.to_xlsx_file(&xlsx_path).unwrap();

        // Import back
        let imported = CsvTable::from_xlsx_file(&xlsx_path, false).unwrap();

        assert_eq!(imported.data.len(), 0);
        assert_eq!(imported.headers, None);
    }

    /// Test round-trip with Unicode characters
    #[test]
    fn test_excel_round_trip_unicode() {
        let temp_dir = tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("unicode.xlsx");

        let mut original = CsvTable::with_headers(vec![
            "Language".to_string(),
            "Greeting".to_string(),
        ]);
        original.data.push(vec!["English".to_string(), "Hello".to_string()]);
        original.data.push(vec!["Japanese".to_string(), "こんにちは".to_string()]);
        original.data.push(vec!["Arabic".to_string(), "مرحبا".to_string()]);
        original.data.push(vec!["Emoji".to_string(), "👋🌍".to_string()]);

        // Export and import
        original.to_xlsx_file(&xlsx_path).unwrap();
        let imported = CsvTable::from_xlsx_file(&xlsx_path, true).unwrap();

        // Verify Unicode preserved
        assert_eq!(imported.data[1][1], "こんにちは");
        assert_eq!(imported.data[2][1], "مرحبا");
        assert_eq!(imported.data[3][1], "👋🌍");
    }

    /// Test file size is reasonable for large datasets
    #[test]
    fn test_excel_export_file_size() {
        let temp_dir = tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("size_test.xlsx");

        // Create table with 1000 rows
        let mut table = CsvTable::with_headers(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
        for i in 0..1000 {
            table.data.push(vec![
                format!("Value{}", i),
                format!("{}", i * 2),
                format!("{}.{}", i, i),
            ]);
        }

        table.to_xlsx_file(&xlsx_path).unwrap();

        // Check file was created and has reasonable size
        let metadata = fs::metadata(&xlsx_path).unwrap();
        assert!(metadata.len() > 0);
        // XLSX files should be reasonably compressed - expect < 1MB for 1000 rows
        assert!(metadata.len() < 1_000_000);
    }
}
