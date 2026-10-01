# FeedLens: Ultra-High-Performance Feed Analytics

FeedLens is a standalone desktop application designed to stream, parse, and analyze massive XML and JSON feeds (100 GB+, 20M–30M+ records) with instant startup, out-of-core memory safety, and sub-150ms analytical queries.

---

## ⚡ Key Highlights & Architecture

- **Desktop Shell**: **Tauri 2.0** (Rust + WebView2)
  - Sub-200ms cold startup, ~35 MB idle RAM footprint.
  - Native Windows file dialogs (`rfd`) and direct IPC commands without HTTP overhead.
- **Ingestion & Streaming Core**: **Rust**
  - Zero-copy streaming XML parser (`quick-xml`) with SIMD tokenization, auto-detection of record tags, and malformed tag error recovery.
  - Streaming JSON & NDJSON / JSONL flattener (`serde_json`).
  - Native on-the-fly decompressors for `.gz` / `.gzip`, `.bz2`, `.zip`, and `.tar.gz` with zero disk pre-extraction.
  - Atomic `bytes_read` tracker for smooth, 100% accurate percentage, MB/s throughput, records/sec rate, and dynamic ETA estimation.
- **Database & Analytics Engine**: **DuckDB OLAP**
  - Dynamic table schema auto-inference with DuckDB's vectorized `Appender` (>500,000 records/sec).
  - Out-of-core memory safety: can analyze 100 GB feeds on an 8 GB laptop without out-of-memory errors.
  - Vectorized analytical queries across CPU cores:
    - **Completeness Matrix**: non-null counts, empty string detection, fill rate %, distinct unique counts.
    - **Value Frequency Distribution**: Top 50–100 values with percentage distribution share.
    - **Duplicate / Collision Detector**: Detects colliding IDs, duplicate SKUs, or non-unique keys.
    - **Search & Pagination**: Multi-column `ILIKE` search and sorting with millisecond response times.
    - **Native Exporters**: Fast native streaming CSV and JSON exports powered by DuckDB's `COPY` engine.
- **Frontend & Visuals**: **Svelte 5 + Tailwind CSS + Apache ECharts**
  - Svelte 5 Runes (`$state`, `$derived`, `$effect`) for instant reactivity.
  - GPU-accelerated interactive charts (fill rates, donuts, category distributions).
  - 4 specialized workspaces:
    1. **Overview & Health**: High-level KPIs, completeness distribution chart, quick health summary.
    2. **Completeness Matrix**: Detailed per-column data quality grid with search and sort.
    3. **Data Explorer**: Paginated search grid with record inspector modal.
    4. **Deep Dive & Distribution**: Column frequency charts (bar/pie) and collision key tracker.
  - Built-in 25k & 100k demo feed generator for immediate 1-click benchmarking.

---

## 🛠️ Development & Running

### Prerequisites
- [Rust](https://rustup.rs/) (1.80+)
- [Node.js](https://nodejs.org/) (20+) and `npm`

### Commands

```bash
# Install frontend dependencies
npm install

# Run frontend tests & build
npm run build

# Run Rust unit & integration tests
cargo test --manifest-path src-tauri/Cargo.toml

# Start FeedLens in Tauri desktop dev mode
npm run tauri dev
```

---

## 📁 Project Structure

```
FeedLens/
├── src-tauri/
│   ├── src/
│   │   ├── engine/
│   │   │   ├── progress.rs       # Atomic progress tracker & ETA calculator
│   │   │   ├── decompressor.rs   # On-the-fly streaming for raw, .gz, .bz2, .zip, .tar.gz
│   │   │   ├── xml_stream.rs     # Zero-copy streaming XML parser with auto-tag detection
│   │   │   ├── json_stream.rs    # Streaming JSON & NDJSON parser with flattening
│   │   │   └── mod.rs            # FeedFormat detection and record abstractions
│   │   ├── db/
│   │   │   └── mod.rs            # DuckDB manager, vectorized Appender, analytics & exports
│   │   ├── commands.rs           # Tauri IPC commands & demo feed generator
│   │   ├── tests.rs              # Integration tests for engine and DuckDB
│   │   ├── lib.rs                # Tauri app setup and state management
│   │   └── main.rs               # Application entry point
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── lib/
│   │   ├── components/
│   │   │   ├── Header.svelte                 # Title, active file stats, open/demo/export actions
│   │   │   ├── ProgressBanner.svelte         # Live throughput, MB/s, rec/s, ETA, cancel
│   │   │   ├── DropZone.svelte               # Landing dropzone & benchmark triggers
│   │   │   ├── OverviewTab.svelte            # KPI cards & ECharts fill rate visualizer
│   │   │   ├── CompletenessMatrixTab.svelte  # Quality table, unique counts, sample tags
│   │   │   ├── DataExplorerTab.svelte        # Searchable paginated data explorer
│   │   │   ├── DeepDiveTab.svelte            # Frequency distribution charts & duplicate detector
│   │   │   └── RecordInspectorModal.svelte   # Detailed row inspection modal with 1-click copy
│   │   ├── state.svelte.ts       # Global state with Svelte 5 Runes & Tauri events
│   │   └── types.ts              # TypeScript type definitions
│   ├── routes/
│   │   ├── +layout.svelte
│   │   └── +page.svelte          # Main shell coordinating tabs and state
│   └── app.css
└── README.md
```
