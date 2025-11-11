/// Performance benchmarks for large dataset operations
/// Tests load, sort, and filter performance on datasets up to 3M rows

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use csv_navigator::data::{CsvTable, FilterCondition, FilterLogic, FilterOperator, SortOrder};
use tempfile::tempdir;

/// Helper function to create a CSV file with specified number of rows
fn create_large_csv(path: &std::path::Path, row_count: usize) {
    let mut wtr = csv::Writer::from_path(path).unwrap();

    // Write headers
    wtr.write_record(&["ID", "Name", "Age", "Department", "Salary", "Score"]).unwrap();

    // Write data rows
    for i in 0..row_count {
        let department = match i % 5 {
            0 => "Engineering",
            1 => "Sales",
            2 => "Marketing",
            3 => "HR",
            _ => "Operations",
        };
        wtr.write_record(&[
            &i.to_string(),
            &format!("Person_{}", i),
            &((i % 50) + 20).to_string(),  // Age 20-69
            department,
            &((i % 100) * 1000 + 30000).to_string(),  // Salary 30k-130k
            &((i % 100) as f64 * 0.7 + 20.0).to_string(),  // Score 20-90
        ]).unwrap();
    }

    wtr.flush().unwrap();
}

/// Helper function to create an in-memory CsvTable with specified number of rows
fn create_large_table(row_count: usize) -> CsvTable {
    let mut table = CsvTable::with_headers(vec![
        "ID".to_string(),
        "Name".to_string(),
        "Age".to_string(),
        "Department".to_string(),
        "Salary".to_string(),
        "Score".to_string(),
    ]);

    for i in 0..row_count {
        table.data.push(vec![
            i.to_string(),
            format!("Person_{}", i),
            ((i % 50) + 20).to_string(),
            match i % 5 {
                0 => "Engineering".to_string(),
                1 => "Sales".to_string(),
                2 => "Marketing".to_string(),
                3 => "HR".to_string(),
                _ => "Operations".to_string(),
            },
            ((i % 100) * 1000 + 30000).to_string(),
            ((i % 100) as f64 * 0.7 + 20.0).to_string(),
        ]);
    }

    table
}

/// Benchmark CSV loading from file
fn bench_load_csv(c: &mut Criterion) {
    let mut group = c.benchmark_group("csv_load");

    for size in [100_000, 500_000, 1_000_000, 3_000_000].iter() {
        let temp_dir = tempdir().unwrap();
        let csv_path = temp_dir.path().join("benchmark.csv");
        create_large_csv(&csv_path, *size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.iter(|| {
                let table = CsvTable::from_path(&csv_path, true).unwrap();
                black_box(table);
            });
        });
    }

    group.finish();
}

/// Benchmark sorting operations on large datasets
fn bench_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort");

    for size in [100_000, 500_000, 1_000_000, 3_000_000].iter() {
        // Sort by string column (Name)
        let table = create_large_table(*size);
        group.bench_with_input(
            BenchmarkId::new("string_column", size),
            size,
            |b, &_size| {
                b.iter_batched(
                    || table.clone(),
                    |mut t| {
                        t.sort_by_column(1, SortOrder::Ascending).unwrap();
                        black_box(t);
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );

        // Sort by numeric column (Age)
        let table = create_large_table(*size);
        group.bench_with_input(
            BenchmarkId::new("numeric_column", size),
            size,
            |b, &_size| {
                b.iter_batched(
                    || table.clone(),
                    |mut t| {
                        t.sort_by_column(2, SortOrder::Ascending).unwrap();
                        black_box(t);
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark filtering operations
fn bench_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter");

    for size in [100_000, 500_000, 1_000_000, 3_000_000].iter() {
        // Single filter: Age > 40
        let table = create_large_table(*size);
        group.bench_with_input(
            BenchmarkId::new("single_condition", size),
            size,
            |b, &_size| {
                b.iter_batched(
                    || table.clone(),
                    |mut t| {
                        let filter = FilterCondition::new(2, FilterOperator::GreaterThan, "40".to_string());
                        t.apply_filters(&[filter], FilterLogic::All);
                        black_box(t);
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );

        // Multiple filters with AND logic: Age > 40 AND Department = Engineering
        let table = create_large_table(*size);
        group.bench_with_input(
            BenchmarkId::new("multiple_and", size),
            size,
            |b, &_size| {
                b.iter_batched(
                    || table.clone(),
                    |mut t| {
                        let filters = vec![
                            FilterCondition::new(2, FilterOperator::GreaterThan, "40".to_string()),
                            FilterCondition::new(3, FilterOperator::Equals, "Engineering".to_string()),
                        ];
                        t.apply_filters(&filters, FilterLogic::All);
                        black_box(t);
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );

        // Multiple filters with OR logic: Salary > 100000 OR Score > 80
        let table = create_large_table(*size);
        group.bench_with_input(
            BenchmarkId::new("multiple_or", size),
            size,
            |b, &_size| {
                b.iter_batched(
                    || table.clone(),
                    |mut t| {
                        let filters = vec![
                            FilterCondition::new(4, FilterOperator::GreaterThan, "100000".to_string()),
                            FilterCondition::new(5, FilterOperator::GreaterThan, "80".to_string()),
                        ];
                        t.apply_filters(&filters, FilterLogic::Any);
                        black_box(t);
                    },
                    criterion::BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark CSV saving to file
fn bench_save_csv(c: &mut Criterion) {
    let mut group = c.benchmark_group("csv_save");

    for size in [100_000, 500_000, 1_000_000, 3_000_000].iter() {
        let table = create_large_table(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.iter_batched(
                || {
                    let temp_dir = tempdir().unwrap();
                    let csv_path = temp_dir.path().join("save_benchmark.csv");
                    (table.clone(), csv_path, temp_dir)
                },
                |(t, path, _temp_dir)| {
                    t.save_to_path(&path).unwrap();
                    black_box(());
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

/// Benchmark complete workflow: load -> sort -> filter -> save
fn bench_complete_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("complete_workflow");

    for size in [100_000, 500_000, 1_000_000].iter() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("input.csv");
        let output_path = temp_dir.path().join("output.csv");
        create_large_csv(&input_path, *size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.iter(|| {
                // Load
                let mut table = CsvTable::from_path(&input_path, true).unwrap();

                // Sort by Age
                table.sort_by_column(2, SortOrder::Ascending).unwrap();

                // Filter: Age > 40 AND Department = Engineering
                let filters = vec![
                    FilterCondition::new(2, FilterOperator::GreaterThan, "40".to_string()),
                    FilterCondition::new(3, FilterOperator::Equals, "Engineering".to_string()),
                ];
                table.apply_filters(&filters, FilterLogic::All);

                // Clear filter (editing requires unfiltered data)
                table.clear_filter();

                // Save
                table.save_to_path(&output_path).unwrap();

                black_box(());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_load_csv,
    bench_sort,
    bench_filter,
    bench_save_csv,
    bench_complete_workflow
);
criterion_main!(benches);
