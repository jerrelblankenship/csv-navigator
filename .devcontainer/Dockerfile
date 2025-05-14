# .devcontainer/Dockerfile
# Dev container for CSV Navigator (Rust + Slint application)
# Designed to work with Podman on both M4 MacBook Pro and Windows 11
FROM debian:bookworm-slim

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive

# Install basic dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    libx11-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libxkbcommon-x11-dev \
    cmake \
    ninja-build \
    libfontconfig-dev \
    wget \
    python3 \
    unzip \
    lld \
    libclang-dev \
    libasound2-dev \
    libgtk-3-dev \
    libwayland-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user with sudo access
RUN groupadd --gid 1000 developer \
    && useradd --uid 1000 --gid 1000 -m developer

# Set up Rust with the default stable toolchain
USER developer
ENV PATH="/home/developer/.cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN rustup component add rust-src rustfmt clippy rust-analyzer

# Install cargo tools we recommend
RUN cargo install cargo-edit cargo-watch cargo-expand cargo-insta cargo-audit

# Install Slint CLI tools
RUN cargo install slint-viewer cargo-generate

# Switch back to root for final setup
USER root

WORKDIR /workspace
CMD ["/bin/bash"]
