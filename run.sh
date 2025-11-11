#!/usr/bin/env bash
# run.sh - Run script for CSV Navigator
#
# Convenience script for running CSV Navigator with command-line arguments.
# Automatically detects whether to use debug or release build.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_debug() {
    echo -e "${BLUE}[DEBUG]${NC} $1"
}

# Display help message
show_help() {
    cat << EOF
Usage: ./run.sh [OPTIONS] [-- [APP_ARGS]]

Run CSV Navigator application with optional arguments.

OPTIONS:
    -h, --help          Show this help message
    -d, --debug         Run debug build (default: release)
    -r, --release       Run release build (default)
    -b, --build         Build before running
    --no-build          Don't build, just run
    -v, --verbose       Verbose output

APP_ARGS:
    Any arguments after -- are passed directly to the application.
    For application-specific options, run: ./run.sh -- --help

EXAMPLES:
    ./run.sh                          # Run release build
    ./run.sh --debug                  # Run debug build
    ./run.sh --build                  # Build and run
    ./run.sh -- data.csv              # Run with CSV file argument
    ./run.sh --debug -- --help        # Show application help

EOF
}

# Parse command line arguments
BUILD_MODE="release"
DO_BUILD=false
VERBOSE=false
APP_ARGS=()

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -d|--debug)
            BUILD_MODE="debug"
            shift
            ;;
        -r|--release)
            BUILD_MODE="release"
            shift
            ;;
        -b|--build)
            DO_BUILD=true
            shift
            ;;
        --no-build)
            DO_BUILD=false
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        --)
            shift
            APP_ARGS=("$@")
            break
            ;;
        *)
            # Unknown option, pass to application
            APP_ARGS+=("$1")
            shift
            ;;
    esac
done

# Main run process
main() {
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust from https://rustup.rs/"
        exit 1
    fi

    # Determine binary path
    BINARY_PATH="target/$BUILD_MODE/csv-navigator"

    # Add .exe extension for Windows
    if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
        BINARY_PATH="${BINARY_PATH}.exe"
    fi

    # Build if requested or if binary doesn't exist
    if [ "$DO_BUILD" = true ] || [ ! -f "$BINARY_PATH" ]; then
        if [ ! -f "$BINARY_PATH" ]; then
            print_info "Binary not found, building..."
        else
            print_info "Building CSV Navigator..."
        fi

        BUILD_CMD="cargo build"
        if [ "$BUILD_MODE" = "release" ]; then
            BUILD_CMD="$BUILD_CMD --release"
        fi

        if [ "$VERBOSE" = true ]; then
            print_debug "Running: $BUILD_CMD"
        fi

        if ! eval "$BUILD_CMD"; then
            print_error "Build failed!"
            exit 1
        fi
        print_info "Build completed successfully!"
    fi

    # Check if binary exists
    if [ ! -f "$BINARY_PATH" ]; then
        print_error "Binary not found at: $BINARY_PATH"
        print_info "Try running with --build flag to build first"
        exit 1
    fi

    # Run the application
    if [ ${#APP_ARGS[@]} -eq 0 ]; then
        print_info "Running CSV Navigator ($BUILD_MODE mode)..."
        if [ "$VERBOSE" = true ]; then
            print_debug "Command: $BINARY_PATH"
        fi
        exec "$BINARY_PATH"
    else
        print_info "Running CSV Navigator ($BUILD_MODE mode) with arguments..."
        if [ "$VERBOSE" = true ]; then
            print_debug "Command: $BINARY_PATH ${APP_ARGS[*]}"
        fi
        exec "$BINARY_PATH" "${APP_ARGS[@]}"
    fi
}

# Run main
main
