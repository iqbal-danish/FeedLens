import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  SelectedFileInfo,
  IngestionProgress,
  OverviewStats,
  ColumnCompleteness,
  ValueFrequency,
  DuplicateEntry,
  QueryResultPage,
} from "./types";

export class FeedLensState {
  currentFile = $state<SelectedFileInfo | null>(null);
  progress = $state<IngestionProgress | null>(null);
  isIngesting = $state(false);
  stats = $state<OverviewStats | null>(null);
  completeness = $state<ColumnCompleteness[]>([]);
  activeTab = $state<"overview" | "completeness" | "explorer" | "deepdive">("overview");

  // Deep dive selection
  selectedColumn = $state<string>("");
  columnDistribution = $state<ValueFrequency[]>([]);
  duplicateEntries = $state<DuplicateEntry[]>([]);
  isLoadingDeepDive = $state(false);

  // Data Explorer table state
  tableData = $state<QueryResultPage | null>(null);
  searchFilter = $state("");
  currentPage = $state(1);
  pageSize = $state(50);
  sortColumn = $state<string | null>(null);
  sortDesc = $state(false);
  isLoadingTable = $state(false);

  // Inspector modal
  inspectingRecord = $state<Record<string, string | null> | null>(null);

  // Status notification
  toastMessage = $state<string | null>(null);

  constructor() {
    this.setupListeners();
  }

  showToast(msg: string) {
    this.toastMessage = msg;
    setTimeout(() => {
      if (this.toastMessage === msg) {
        this.toastMessage = null;
      }
    }, 4000);
  }

  async setupListeners() {
    await listen<IngestionProgress>("ingest-progress", (event) => {
      this.progress = event.payload;
      if (event.payload.status === "indexing") {
        this.isIngesting = true;
      }
    });

    await listen("ingest-complete", async () => {
      this.isIngesting = false;
      this.showToast("Feed successfully indexed into DuckDB!");
      await this.refreshAllData();
    });

    await listen<string>("ingest-error", (event) => {
      this.isIngesting = false;
      this.showToast(`Error: ${event.payload}`);
    });
  }

  async openFile() {
    try {
      const file = await invoke<SelectedFileInfo | null>("pick_feed_file");
      if (file) {
        this.currentFile = file;
        await this.startIngest(file.path);
      }
    } catch (e: any) {
      this.showToast(`Failed to open file: ${e.message || e}`);
    }
  }

  async loadDemo(count = 25000) {
    try {
      this.showToast(`Generating ${count.toLocaleString()} sample job records...`);
      const file = await invoke<SelectedFileInfo>("generate_demo_feed", { recordCount: count });
      this.currentFile = file;
      await this.startIngest(file.path);
    } catch (e: any) {
      this.showToast(`Failed to generate demo: ${e.message || e}`);
    }
  }

  async startIngest(filePath: string) {
    this.isIngesting = true;
    this.stats = null;
    this.completeness = [];
    this.tableData = null;
    this.columnDistribution = [];
    this.duplicateEntries = [];
    this.currentPage = 1;

    try {
      await invoke("start_ingestion", { filePath, recordTag: null });
    } catch (e: any) {
      this.isIngesting = false;
      this.showToast(`Ingestion failed: ${e.message || e}`);
    }
  }

  async cancelIngest() {
    try {
      await invoke("cancel_ingestion");
      this.isIngesting = false;
      this.showToast("Ingestion cancelled");
      await this.refreshAllData();
    } catch (e: any) {
      this.showToast(`Failed to cancel: ${e.message || e}`);
    }
  }

  async refreshAllData() {
    try {
      const [stats, matrix] = await Promise.all([
        invoke<OverviewStats>("get_overview_stats"),
        invoke<ColumnCompleteness[]>("get_completeness_matrix"),
      ]);

      this.stats = stats;
      this.completeness = matrix;

      if (matrix.length > 0 && !this.selectedColumn) {
        this.selectedColumn = matrix[0].column_name;
      }

      await Promise.all([
        this.fetchTablePage(),
        this.selectedColumn ? this.fetchDeepDive(this.selectedColumn) : Promise.resolve(),
      ]);
    } catch (e: any) {
      console.error("Refresh error:", e);
    }
  }

  async fetchTablePage() {
    this.isLoadingTable = true;
    try {
      const page = await invoke<QueryResultPage>("query_records", {
        page: this.currentPage,
        pageSize: this.pageSize,
        sortCol: this.sortColumn,
        sortDesc: this.sortDesc,
        filterText: this.searchFilter || null,
      });
      this.tableData = page;
    } catch (e: any) {
      console.error("Table query error:", e);
    } finally {
      this.isLoadingTable = false;
    }
  }

  async setPage(p: number) {
    if (this.tableData && p >= 1 && p <= this.tableData.total_pages) {
      this.currentPage = p;
      await this.fetchTablePage();
    }
  }

  async setSort(col: string) {
    if (this.sortColumn === col) {
      this.sortDesc = !this.sortDesc;
    } else {
      this.sortColumn = col;
      this.sortDesc = false;
    }
    this.currentPage = 1;
    await this.fetchTablePage();
  }

  async applySearch(query: string) {
    this.searchFilter = query;
    this.currentPage = 1;
    await this.fetchTablePage();
  }

  async fetchDeepDive(colName: string) {
    this.selectedColumn = colName;
    this.isLoadingDeepDive = true;
    try {
      const [dist, dups] = await Promise.all([
        invoke<ValueFrequency[]>("get_column_distribution", { columnName: colName, limit: 50 }),
        invoke<DuplicateEntry[]>("get_duplicate_records", { columnName: colName, limit: 30 }),
      ]);
      this.columnDistribution = dist;
      this.duplicateEntries = dups;
    } catch (e: any) {
      console.error("Deep dive error:", e);
    } finally {
      this.isLoadingDeepDive = false;
    }
  }

  async exportData(isJson = false) {
    try {
      const ext = isJson ? "json" : "csv";
      const defaultName = `feedlens_export_${Date.now()}.${ext}`;
      const path = await invoke<string | null>("pick_export_file", { defaultName, isJson });
      if (path) {
        this.showToast(`Exporting to ${path}...`);
        const count = await invoke<number>("export_records", {
          exportPath: path,
          isJson,
          filterText: this.searchFilter || null,
        });
        this.showToast(`Successfully exported ${count.toLocaleString()} records!`);
      }
    } catch (e: any) {
      this.showToast(`Export failed: ${e.message || e}`);
    }
  }
}

export const feedState = new FeedLensState();
