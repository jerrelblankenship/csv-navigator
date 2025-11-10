# CSV Navigator

A modern, cross-platform CSV file viewer and editor built with Rust and Slint.

## Overview

CSV Navigator is a desktop application that provides an intuitive interface for viewing and editing CSV files. Built using Rust for performance and reliability, and Slint for a native-looking, responsive UI across all platforms.

## Features

- **CSV File Viewing**: Display CSV files in a clean, tabular format
- **Modern UI**: Native-looking interface powered by Slint
- **Cross-Platform**: Runs on Linux, macOS, and Windows
- **Fast and Reliable**: Built with Rust for optimal performance
- **TDD Approach**: Developed following Test-Driven Development principles

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2021 or later)
- Cargo (comes with Rust)

## Installation

### From Source

1. Clone the repository:
```bash
git clone https://github.com/jerrelblankenship/csv-navigator.git
cd csv-navigator
```

2. Build the project:
```bash
cargo build --release
```

3. Run the application:
```bash
cargo run --release
```

## Usage

Launch the application and use the toolbar buttons:
- **Open CSV**: Load a CSV file from your system
- **Save**: Save changes to the current file

The main window displays your CSV data in a scrollable table with headers.

## Development

### Project Structure

```
csv-navigator/
├── src/
│   ├── main.rs         # Application entry point
│   └── lib.rs          # Core application logic and config
├── ui/
│   └── main.slint      # UI definitions and layouts
├── tests/              # Integration tests
├── build.rs            # Build script for Slint compilation
└── Cargo.toml          # Project dependencies
```

### Building

```bash
# Debug build
cargo build

# Release build with optimizations
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name
```

### Development Container

This project includes a dev container configuration for consistent development environments. Open the project in VS Code with the Remote-Containers extension to get started.

## Architecture

CSV Navigator follows clean architecture principles:

- **AppConfig**: Configuration structure for window settings and preferences
- **AppWindow**: Main UI component defined in Slint
- **CsvTable**: Reusable table component for displaying CSV data

The application uses Slint's reactive UI framework, allowing for efficient updates and a smooth user experience.

## Dependencies

- **[Slint](https://slint.dev/)** (v1.11.0): UI framework for building native interfaces

## Testing

The project follows TDD principles with comprehensive test coverage:

- Unit tests in `src/lib.rs`
- Integration tests in `tests/` directory
- UI integration tests for component verification

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Roadmap

Future features planned:
- CSV editing capabilities
- Column sorting and filtering
- Search functionality
- Export to different formats
- Drag-and-drop file loading

## Author

Jerrel Blankenship

## Acknowledgments

- Built with [Slint](https://slint.dev/), a declarative UI toolkit for Rust
- Inspired by the need for a simple, reliable CSV viewer
