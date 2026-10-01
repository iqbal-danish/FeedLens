# FeedLens: Ultra-High-Performance Feed Analytics & Inspection

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB.svg?logo=tauri)](https://tauri.app/)
[![DuckDB](https://img.shields.io/badge/DuckDB-1.2-FFF000.svg?logo=duckdb)](https://duckdb.org/)
[![Svelte](https://img.shields.io/badge/Svelte-5.0-FF3E00.svg?logo=svelte)](https://svelte.dev/)
[![TailwindCSS](https://img.shields.io/badge/TailwindCSS-v4-38B2AC.svg?logo=tailwind-css)](https://tailwindcss.com/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**FeedLens** is a standalone, ultra-high-performance desktop application engineered to ingest, stream-parse, and analyze massive XML, JSON, and NDJSON feeds (100 GB+, 20M–30M+ records) with near-instant cold startups, constant memory footprints, and sub-150ms analytical queries.

---

## ⚡ Core Capabilities & Architectural Highlights

### 1. Ingestion & Streaming Core (Rust)
- **Zero-Copy Streaming XML Engine**: Powered by `quick-xml` with SIMD-accelerated tokenization, automatic repeating element detection (`detect_xml_record_tag`), namespace stripping, and resilient recovery for malformed tags.
- **Universal JSON & NDJSON Parser**: Supports root JSON arrays (`[{...}]`), newline-delimited JSON (`NDJSON` / `JSONL`), and object-wrapped payloads (`{"data": [...]}`, `{"jobs": [...]}`) with automatic array discovery and flattening.
- **On-the-Fly Streaming Decompressors**: Direct streaming readers for `.gz` / `.gzip`, `.bz2`, `.zip`, and `.tar.gz` with zero intermediate disk extraction.
- **Network URL Streaming**: Hardened HTTP client (`ureq`) with browser user-agent emulation, automatic gzip magic-byte (`0x1F, 0x8B`) sniffing, connection timeouts, and descriptive status code diagnostics.
- **Pre-Buffered Tag Sniffing**: 512 KB stream pre-buffering (`Cursor::new(&buf).chain(reader)`) enabling automatic XML tag detection (`<job>`, `<vacancy>`, `<item>`, `<posting>`, etc.) on live HTTP streams without duplicate requests.

### 2. High-Throughput Ingestion Presets
- 🚀 **Extreme Fast Mode (~25,000+ rec/s)**: Bypasses long HTML descriptions and bulky CDATA content to maximize throughput for schema inspection and metrics.
- ⚡ **Fast Mode (~12,000 rec/s)**: Skips descriptions while preserving raw tags and structured attributes.
- 🔍 **Full Inspection Mode (~5,000 rec/s)**: Retains full text descriptions and complete raw content for thorough forensics.

### 3. Columnar Database & Analytics (DuckDB OLAP)
- **Vectorized Appender**: Ingests records in dynamic high-throughput batches (>500,000 rec/s into DuckDB memory).
- **Out-of-Core Memory Safety**: Analyzes 50 GB–100 GB feeds on standard 8 GB/16 GB laptops without memory exhaustion.
- **Instant Vectorized Queries**:
  - **Completeness Matrix**: Fill rates, valid non-empty counts, null/empty counts, and distinct unique values.
  - **Deep Dive Frequency Distribution**: Top 50–100 value distributions with interactive Apache ECharts visualizations.
  - **Duplicate & Collision Detector**: Instant identification of non-unique job references, duplicate IDs, or colliding SKUs.
  - **Fast Data Explorer**: Paginated table exploration with multi-column sorting and instant search filtering.
  - **Native Exporters**: 1-click streaming CSV and JSON exports powered directly by DuckDB's vectorized `COPY` engine.
  - **Persistent Feed Catalog**: Automatic background DuckDB file persistence and recent feeds catalog for zero-wait subsequent reloads.

### 4. Modern Desktop UI (Svelte 5 + Tailwind CSS)
- **Svelte 5 Runes**: Built with `$state`, `$derived`, and `$effect` for ultra-responsive UI updates.
- **Modern Searchable Attribute Selector**: Floating glass dropdown featuring real-time search filtering, health-colored fill rate badges (🟢 100%, 🟡 60–99%, 🔴 <60%), distinct unique counts, and quick filter tabs.
- **Native Desktop Shell**: Tauri 2.0 with Windows DWM dark title bar integration and custom branded icons.

---

## 🖥️ Workspaces & Interface

| Workspace | Purpose |
|---|---|
| **Overview & Health** | High-level dataset KPIs, column count, average fill rate, record throughput, and attribute completeness matrix distribution. |
| **Completeness Matrix** | Detailed per-column data quality grid showing fill rate %, valid records, empty/null values, and sample data. |
| **Data Explorer** | High-speed paginated data grid with column sorting, live search filtering, and detailed JSON record inspector modal. |
| **Deep Dive & Distribution** | Interactive bar and donut distribution charts (Apache ECharts) and collision key duplicate tracker with distinct uniqueness counts. |

---

## 🛠️ Development & Building

### Prerequisites
- [Rust](https://rustup.rs/) (1.80+)
- [Node.js](https://nodejs.org/) (20+) and `npm`

### Installation & Setup

```bash
# Clone the repository
git clone https://github.com/iqbal-danish/FeedLens.git
cd FeedLens

# Install frontend dependencies
npm install
```

### Running in Development

```bash
# Start FeedLens desktop in Tauri dev mode
npm run tauri dev
```

### Running Tests

```bash
# Run backend engine and DuckDB integration tests
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
```

### Building Production Release

```bash
# Build frontend assets and release binary
npm run build
npx tauri build --no-bundle
```
The compiled standalone executable will be generated at `src-tauri/target/release/tauri-app.exe` (or `FeedLens.exe`).

---

## 📁 Repository Structure

```
FeedLens/
├── src-tauri/
│   ├── src/
│   │   ├── engine/
│   │   │   ├── progress.rs       # Atomic progress tracker & throughput calculator
│   │   │   ├── decompressor.rs   # Streaming URL & archive decompressors (.gz, .bz2, .zip, .tar.gz)
│   │   │   ├── xml_stream.rs     # Zero-copy streaming XML parser with auto-tag discovery
│   │   │   ├── json_stream.rs    # Streaming JSON & NDJSON parser with nested array discovery
│   │   │   └── mod.rs            # Feed format sniffing and abstractions
│   │   ├── db/
│   │   │   └── mod.rs            # DuckDB manager, vectorized Appender, analytics & catalog
│   │   ├── commands.rs           # Tauri IPC commands & ingestion lifecycle management
│   │   ├── tests.rs              # Automated unit and integration test suite
│   │   ├── lib.rs                # Tauri app setup and state initialization
│   │   └── main.rs               # Desktop executable entry point
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── lib/
│   │   ├── components/
│   │   │   ├── Header.svelte                 # Top navigation, active feed stats & actions
│   │   │   ├── ProgressBanner.svelte         # Live ingestion throughput, MB/s, rec/s & ETA
│   │   │   ├── DropZone.svelte               # Drag & drop file / remote URL ingestion form
│   │   │   ├── OverviewTab.svelte            # High-level KPIs & completeness distribution
│   │   │   ├── CompletenessMatrixTab.svelte  # Column quality grid with search & sort
│   │   │   ├── DataExplorerTab.svelte        # Searchable paginated data explorer table
│   │   │   ├── DeepDiveTab.svelte            # Searchable attribute selector & frequency charts
│   │   │   └── RecordInspectorModal.svelte   # Row inspector modal with 1-click JSON copy
│   │   ├── state.svelte.ts       # Global state management using Svelte 5 Runes
│   │   └── types.ts              # TypeScript type interfaces
│   ├── routes/
│   │   └── +page.svelte          # Main application coordinator
│   └── app.css
├── static/                       # Production application branding & icons
├── .gitignore                    # Optimized gitignore excluding build caches & local databases
├── package.json
└── README.md
```

---

## 📄 License
This project is licensed under the MIT License.
