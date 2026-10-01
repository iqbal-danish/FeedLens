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
  RecentFeed,
} from "./types";

export class FeedLensState {
  currentFile = $state<SelectedFileInfo | null>(null);
  progress = $state<IngestionProgress | null>(null);
  isIngesting = $state(false);
  hasActiveDataset = $state(false);
  stats = $state<OverviewStats | null>(null);
  completeness = $state<ColumnCompleteness[]>([]);
  activeTab = $state<"overview" | "completeness" | "explorer" | "deepdive">("overview");

  // Recent Analyzed Feeds
  recentFeeds = $state<RecentFeed[]>([]);
  isLoadingRecent = $state(false);

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

  // Status notification & errors
  toastMessage = $state<string | null>(null);
  lastError = $state<string | null>(null);

  constructor() {
    this.setupListeners();
    this.fetchRecentFeeds();
  }

  clearError() {
    this.lastError = null;
  }

  showToast(msg: string) {
    this.toastMessage = msg;
    setTimeout(() => {
      if (this.toastMessage === msg) {
        this.toastMessage = null;
      }
    }, 5000);
  }

  async setupListeners() {
    await listen<IngestionProgress>("ingest-progress", (event) => {
      this.progress = event.payload;
      if (event.payload.status === "ready" || event.payload.status === "cancelled" || event.payload.status === "error") {
        this.isIngesting = false;
      } else if (event.payload.status === "indexing") {
        if (!this.stats && this.isIngesting) {
          this.hasActiveDataset = true;
        }
      }
    });

    await listen("ingest-complete", async () => {
      this.isIngesting = false;
      if (this.progress) {
        this.progress.status = "ready";
        this.progress.percentage = 100.0;
      }
      this.lastError = null;
      this.showToast("Feed successfully indexed into DuckDB!");
      await this.refreshAllData();
      await this.fetchRecentFeeds();
      this.isIngesting = false;
      this.hasActiveDataset = true;
      this.activeTab = "overview";
    });

    await listen<string>("ingest-error", (event) => {
      this.isIngesting = false;
      this.lastError = event.payload;
      if (!this.stats) {
        this.hasActiveDataset = false;
      }
      this.showToast(`Error: ${event.payload}`);
    });
  }

  async fetchRecentFeeds() {
    this.isLoadingRecent = true;
    try {
      this.recentFeeds = await invoke<RecentFeed[]>("get_recent_feeds");
    } catch (e: any) {
      console.warn("Could not fetch recent feeds:", e);
    } finally {
      this.isLoadingRecent = false;
    }
  }

  async loadRecentFeed(feed: RecentFeed) {
    try {
      this.showToast(`Loading ${feed.filename}...`);
      const stats = await invoke<OverviewStats>("load_recent_feed", { taskId: feed.id });
      this.stats = stats;
      this.hasActiveDataset = true;
      this.currentFile = {
        path: feed.db_path,
        filename: feed.filename,
        size_bytes: 0,
        format: feed.filename.endsWith(".json") ? "JSON" : "XML",
      };
      await this.refreshAllData();
      this.activeTab = "overview";
      this.showToast(`Loaded ${feed.filename} instantly!`);
    } catch (e: any) {
      this.showToast(`Failed to load feed: ${e.message || e}`);
    }
  }

  async deleteRecentFeed(feedId: string) {
    try {
      await invoke("delete_recent_feed", { taskId: feedId });
      this.recentFeeds = this.recentFeeds.filter((f) => f.id !== feedId);
      this.showToast("Deleted feed entry.");
    } catch (e: any) {
      this.showToast(`Failed to delete: ${e.message || e}`);
    }
  }

  resetToLanding() {
    this.hasActiveDataset = false;
    this.currentFile = null;
    this.stats = null;
    this.completeness = [];
    this.tableData = null;
    this.columnDistribution = [];
    this.duplicateEntries = [];
    this.fetchRecentFeeds();
  }

  async pickFeedFile(): Promise<SelectedFileInfo | null> {
    try {
      const file = await invoke<SelectedFileInfo | null>("pick_feed_file");
      return file;
    } catch (e: any) {
      this.showToast(`Failed to pick file: ${e.message || e}`);
      return null;
    }
  }

  async openFile() {
    const file = await this.pickFeedFile();
    if (file) {
      this.currentFile = file;
      await this.startIngest("file", file.path, null, "extreme");
    }
  }

  async startIngest(
    sourceType: "file" | "url",
    filePath: string,
    recordTag: string | null = null,
    ingestionMode: string = "extreme"
  ) {
    this.lastError = null;
    this.hasActiveDataset = true;
    this.isIngesting = true;
    this.stats = null;
    this.completeness = [];
    this.tableData = null;
    this.columnDistribution = [];
    this.duplicateEntries = [];
    this.currentPage = 1;

    try {
      await invoke("start_ingestion", {
        sourceType,
        filePath,
        recordTag: recordTag && recordTag !== "Auto" && recordTag.trim() !== "" ? recordTag.trim() : null,
        ingestionMode,
      });
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

  async exportColumnFrequency(columnName: string) {
    try {
      const cleanCol = columnName.replace(/[^a-zA-Z0-9_]/g, "_");
      const defaultName = `${cleanCol}_frequency_${Date.now()}.csv`;
      const path = await invoke<string | null>("pick_export_file", { defaultName, isJson: false });
      if (path) {
        this.showToast(`Exporting frequency for ${columnName}...`);
        const count = await invoke<number>("export_column_frequency", {
          columnName,
          exportPath: path,
        });
        this.showToast(`Exported ${count.toLocaleString()} unique values to CSV!`);
      }
    } catch (e: any) {
      this.showToast(`Frequency export failed: ${e.message || e}`);
    }
  }
}

export const feedState = new FeedLensState();
