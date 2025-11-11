#!/usr/bin/env bash
# build.sh - Build script for CSV Navigator
#
# Builds the CSV Navigator application in release mode with optimizations.
# Can build for multiple platforms if cross-compilation is set up.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

# Display help message
show_help() {
    cat << EOF
Usage: ./build.sh [OPTIONS]

Build CSV Navigator in release mode with optimizations.

OPTIONS:
    -h, --help          Show this help message
    -d, --debug         Build in debug mode instead of release
    -t, --target TARGET Build for specific target (e.g., x86_64-pc-windows-gnu)
    -c, --clean         Clean before building
    --no-strip          Don't strip symbols from binary
    --features FEATURES Build with specific features

EXAMPLES:
    ./build.sh                    # Build release version
    ./build.sh --debug            # Build debug version
    ./build.sh --clean            # Clean and build
    ./build.sh --target x86_64-pc-windows-gnu

EOF
}

# Parse command line arguments
BUILD_MODE="release"
CLEAN=false
TARGET=""
STRIP=true
FEATURES=""

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
        -t|--target)
            TARGET="$2"
            shift 2
            ;;
        -c|--clean)
            CLEAN=true
            shift
            ;;
        --no-strip)
            STRIP=false
            shift
            ;;
        --features)
            FEATURES="$2"
            shift 2
            ;;
        *)
            print_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Main build process
main() {
    print_info "Building CSV Navigator..."

    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust from https://rustup.rs/"
        exit 1
    fi

    # Clean if requested
    if [ "$CLEAN" = true ]; then
        print_info "Cleaning previous build..."
        cargo clean
    fi

    # Build command
    BUILD_CMD="cargo build"

    # Add release flag
    if [ "$BUILD_MODE" = "release" ]; then
        BUILD_CMD="$BUILD_CMD --release"
    fi

    # Add target if specified
    if [ -n "$TARGET" ]; then
        BUILD_CMD="$BUILD_CMD --target $TARGET"
    fi

    # Add features if specified
    if [ -n "$FEATURES" ]; then
        BUILD_CMD="$BUILD_CMD --features $FEATURES"
    fi

    # Execute build
    print_info "Running: $BUILD_CMD"
    if eval "$BUILD_CMD"; then
        print_info "Build completed successfully!"

        # Determine binary location
        if [ -n "$TARGET" ]; then
            BINARY_PATH="target/$TARGET/$BUILD_MODE/csv-navigator"
        else
            BINARY_PATH="target/$BUILD_MODE/csv-navigator"
        fi

        # Add .exe extension for Windows
        if [[ "$TARGET" == *"windows"* ]] || [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
            BINARY_PATH="${BINARY_PATH}.exe"
        fi

        # Strip binary in release mode if requested
        if [ "$BUILD_MODE" = "release" ] && [ "$STRIP" = true ]; then
            if [ -f "$BINARY_PATH" ]; then
                if command -v strip &> /dev/null; then
                    print_info "Stripping binary..."
                    strip "$BINARY_PATH" 2>/dev/null || print_warn "Could not strip binary (may not be necessary)"
                fi
            fi
        fi

        # Display binary info
        if [ -f "$BINARY_PATH" ]; then
            BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
            print_info "Binary location: $BINARY_PATH"
            print_info "Binary size: $BINARY_SIZE"
        else
            print_warn "Binary not found at expected location: $BINARY_PATH"
        fi

    else
        print_error "Build failed!"
        exit 1
    fi
}

# Run main
main
