# CSV Navigator — Developer Guide

A practical, step‑by‑step reference for building, running, and contributing to CSV Navigator. Targets **Rust + Slint** with a consistent environment via **Dev Containers (Podman)**. Cross‑platform for 64‑bit Windows, macOS, Linux. Performance target: **3M rows** with fast sort/filter and responsive UI.

---

## 1) Architecture & Project Structure

### High‑level
- **Core (Rust):** CSV parsing, type inference, sort/filter, edit & undo/redo, JSON/XLSX export.
- **UI (Slint):** Declarative `.slint` for layout; Rust bindings provide the data model and callbacks.
- **Model:** Virtualized table model returns only visible rows; filtering uses index projection.

### Layout
```
csv-navigator/
├─ .devcontainer/           # Podman Dev Container setup
├─ .github/workflows/       # CI
├─ docs/                    # Guides & images
├─ src/
│  ├─ main.rs               # App entry; UI bootstrapping & callbacks
│  ├─ data.rs               # CsvTable; load/save; sort/filter; type sniffing
│  ├─ edit.rs               # History (undo/redo); grouped actions
│  ├─ ui.rs                 # Slint model adapter & glue
│  └─ csv_navigator.slint   # UI definition
├─ tests/                   # Integration/unit tests
├─ Cargo.toml
├─ build.rs                 # slint-build
└─ README.md
```

---

## 2) Prerequisites

- **Rust**: stable via `rustup` (e.g., `rustup default stable`).
- **VS Code**: with Dev Containers extension.
- **Container runtime**: Podman (Desktop on macOS/Windows; rootless on Linux).
- **Optional native deps** (if building outside container): `libgtk-3-dev`, `pkg-config`, `cmake` (Linux).

---

## 3) Dev Container (Podman) Setup

### VS Code setting
Set Podman as the Docker path:
```json
"dev.containers.dockerPath": "podman"
```

### `.devcontainer/devcontainer.json`
```json
{
  "name": "CSV Navigator",
  "build": { "dockerfile": "Containerfile", "context": ".." },
  "settings": { "dev.containers.dockerPath": "podman" },
  "runArgs": ["--userns=keep-id"],
  "containerUser": "developer",
  "extensions": [
    "rust-lang.rust-analyzer",
    "vadimcn.vscode-lldb",
    "tamasfe.even-better-toml",
    "slint.slint"
  ],
  "remoteUser": "developer"
}
```

### `.devcontainer/Containerfile`
```dockerfile
FROM rust:latest

RUN apt-get update && apt-get install -y \
    libgtk-3-dev libssl-dev pkg-config cmake \
    && apt-get clean

RUN useradd -ms /bin/bash developer
USER developer
WORKDIR /home/developer/workspace
```

### Open in container
1. `git clone ... && cd csv-navigator`
2. **VS Code →** “Reopen in Container”
3. `cargo run --release -- /path/to/large.csv`

---

## 4) Build, Run, and Scripts

- **Build (release):** `cargo build --release`
- **Run:** `cargo run --release -- /path/to/file.csv`
- **Scripts:**
  - `./build.sh` → release build
  - `./run.sh /path/to/file.csv` → run with args

> For GUI testing from inside a Linux container, prefer building inside the container and running the binary on the **host** (to avoid X11/Wayland forwarding friction).

---

## 5) Dependencies

**Cargo.toml (key crates)**
```toml
[dependencies]
csv = "1.3"
slint = "1.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rayon = "1.10"
calamine = "0.20"
rust_xlsxwriter = "0.65"

[build-dependencies]
slint-build = "1.5"
```

**build.rs**
```rust
fn main() {
    slint_build::compile("src/csv_navigator.slint").unwrap();
}
```

---

## 6) Core Implementation Notes

### Data model
```rust
pub struct CsvTable {
    pub headers: Option<Vec<String>>,
    pub data: Vec<Vec<String>>,
    pub filtered_indices: Option<Vec<usize>>,
    pub col_types: Vec<ColType>,
}
pub enum ColType { Text, Number }
```

### Loading CSV
- Use `csv::ReaderBuilder` (buffered, robust).
- Offload file IO to a worker thread; update UI when ready.
- Infer column types from a sample window (e.g., first 10k rows).

### Sorting
- Numeric vs text paths based on `col_types`.
- Rayon `par_sort_unstable_by` for multi‑million rows.
- After sort, **bulk refresh** the UI model; avoid per‑row signals.

### Filtering
- Maintain `filtered_indices: Option<Vec<usize>>`.
- Evaluate conditions per row (AND = all, OR = any); parallelize when beneficial.
- Expose **All/Any** toggle in UI for AND/OR.

### Editing & Undo/Redo
- **EditAction** (SetCell, MultiSet).  
- `History { undo, redo, max }`, record deltas only; group multi‑cell edits.  
- Cap history length/size; warn on massive ops.

### Export
- **CSV:** `csv::Writer` (handles quoting).  
- **JSON:** array of objects; keys = headers or `ColumnN`.  
- **XLSX:** `rust_xlsxwriter` (values only).  
- **XLSX read:** `calamine` (first sheet).

---

## 7) Slint UI Integration

### `.slint` excerpt
```slint
import { StandardTableView, TableColumn, HorizontalBox, VerticalBox, ComboBox, LineEdit, Button, Text } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "CSV Navigator";
    width: 1100px; height: 700px;

    property <[string]> column_names;
    property <int> total_rows;
    property <int> filtered_rows;
    property <model> table_model;

    callback filterRequested(column_index: int, op: string, value: string);
    callback clearFilterRequested();
    callback sortRequested(column: int, order: string);

    // Filter bar + table ...
}
```

### Rust glue (conceptual)
- Set `table_model` to a `slint::VecModel` or custom model that maps rows/cells.
- Hook callbacks:
  - `on_sort_requested` → background sort → refresh model.
  - `on_filter_requested` → compute `filtered_indices` → refresh model.
- Keep UI updates on the main thread (use Slint timers or event-loop helpers to bridge worker thread completion).

---

## 8) Testing Strategy

### Unit
- **Parsing:** headers/no‑headers; quotes; CRLF.
- **Sorting:** numeric/text; asc/desc; stability on small sets.
- **Filtering:** single + multi (AND/OR).
- **Editing/Undo:** single & grouped; redo stack invalidation.

### Integration
- Open → sort → filter → edit → save → export (validate order and content).
- Excel read/write (small fixtures).

### Performance
- Manual benches on synthetic ~3M rows: load/sort/filter timings; memory sampling.
- Profile with `perf` (Linux) / Instruments (macOS).

### CI (GitHub Actions)
```yaml
name: CI
on: [push, pull_request]
jobs:
  build-test:
    strategy: { matrix: { os: [ubuntu-latest, macos-latest, windows-latest] } }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
      - run: cargo test --all --verbose
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy, rustfmt }
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --all -- --check
```

---

## 9) Contribution Workflow

- **Branching:** feature branches → PRs to `main` with CI green.  
- **Issues:** label `feature`, `bug`, `perf`, `ui`, `good first issue`.  
- **Templates:** bug/feature issue templates; PR checklist (tests, clippy, fmt, docs updated).  
- **Style:** `cargo fmt`, `cargo clippy -D warnings`.  
- **Docs:** Update `docs/` with any user‑facing changes.

---

## 10) Release Process

1. Bump version in `Cargo.toml` (follow semver).  
2. Tag: `git tag v0.x.y && git push --tags`.  
3. CI builds artifacts (Win exe/zip, macOS app/dmg TBD, Linux tar/AppImage).  
4. Draft release notes (CHANGELOG highlights).  
5. Verify downloads on all platforms.  
6. Announce (README badge, Discussions).

> Consider `cargo install csv-navigator` publishing for users with Rust, noting GUI deps.

---

## 11) Licensing & Third‑Party

- **Project:** MIT.  
- **Slint:** GPL‑3.0 (community). Distributing the GUI implies GPL‑3.0 terms.  
- **Crates:** Prefer permissive (MIT/Apache). Document licenses in an “About” dialog and/or third‑party notices.

---

## 12) Roadmap (Initial)

- v0.1: MVP open/view, single‑cell edit, save, sort, basic filter, undo/redo.  
- v0.2: Multi‑cell ops, AND/OR filtering, JSON/XLSX export, perf pass.  
- v0.3+: Find, copy/paste blocks, multi‑column sort, column typing UI, preferences, packaging polish.

---

## 13) Appendix

### Sample helper scripts
**build.sh**
```bash
#!/usr/bin/env bash
set -euo pipefail
cargo build --release
```

**run.sh**
```bash
#!/usr/bin/env bash
set -euo pipefail
cargo run --release -- "$@"
```

### Sample code: Undo/Redo skeleton
```rust
pub enum EditAction {
    SetCell { r: usize, c: usize, old: String, newv: String },
    MultiSet { cells: Vec<(usize,usize,String,String)> }
}
pub struct History { undo: Vec<EditAction>, redo: Vec<EditAction>, max: usize }
// ... record/undo/redo & apply functions ...
```

Happy hacking! Open issues for questions or proposals.
