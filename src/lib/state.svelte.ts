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
  CrossTabulationResult,
} from "./types";
import {
  mockStats,
  mockCompleteness,
  mockDistribution,
  mockDuplicates,
  mockTablePage,
} from "./mockData";

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

  // Cross-Tabulation Pivot Matrix state
  pivotSecondaryCol = $state<string | null>(null);
  pivotResult = $state<CrossTabulationResult | null>(null);
  isLoadingPivot = $state(false);

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
    if (typeof window !== "undefined" && !(window as any).__TAURI_INTERNALS__) {
      this.initMockMode();
      return;
    }
    this.setupListeners();
    this.fetchRecentFeeds();
  }

  initMockMode() {
    this.currentFile = {
      path: "/mock/jobs.xml",
      filename: "jobs.xml",
      size_bytes: 4200000,
      format: "XML",
    };
    this.stats = mockStats;
    this.completeness = mockCompleteness;
    this.hasActiveDataset = true;
    this.activeTab = "overview";
    this.selectedColumn = "job_title";
    this.columnDistribution = mockDistribution;
    this.duplicateEntries = mockDuplicates;
    this.tableData = mockTablePage;
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
      if (event.payload.status === "cancelled" || event.payload.status === "error") {
        this.isIngesting = false;
      }
    });

    await listen("ingest-complete", async () => {
      if (this.progress) {
        this.progress.status = "indexing";
        this.progress.current_phase = "Finalizing Completeness Matrix & Health Metrics...";
        this.progress.percentage = 100.0;
      }
      this.lastError = null;
      try {
        await this.refreshAllData();
        await this.fetchRecentFeeds();
      } finally {
        this.isIngesting = false;
        this.hasActiveDataset = true;
        if (this.progress) {
          this.progress.status = "ready";
        }
      }
      this.showToast("Feed successfully indexed into DuckDB!");
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
    if (typeof window !== "undefined" && !(window as any).__TAURI_INTERNALS__) {
      let filteredRows = mockTablePage.rows;
      if (this.searchFilter) {
        const q = this.searchFilter.trim();
        if (q.startsWith("missing:")) {
          const col = q.slice(8).trim().toLowerCase();
          filteredRows = mockTablePage.rows.filter((r) => !r[col] || r[col]?.trim() === "");
        } else if (q.startsWith("populated:")) {
          const col = q.slice(10).trim().toLowerCase();
          filteredRows = mockTablePage.rows.filter((r) => r[col] && r[col]?.trim() !== "");
        } else {
          filteredRows = mockTablePage.rows.filter((r) =>
            Object.values(r).some((v) => v?.toLowerCase().includes(q.toLowerCase()))
          );
        }
      }
      this.tableData = {
        ...mockTablePage,
        rows: filteredRows,
        total_records: filteredRows.length,
        total_pages: Math.max(1, Math.ceil(filteredRows.length / this.pageSize)),
      };
      return;
    }
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

  filterByMissing(columnName: string) {
    this.searchFilter = `missing:${columnName}`;
    this.activeTab = "explorer";
    this.applySearch(this.searchFilter);
    this.showToast(`Filtered records missing "${columnName}"`);
  }

  filterByPopulated(columnName: string) {
    this.searchFilter = `populated:${columnName}`;
    this.activeTab = "explorer";
    this.applySearch(this.searchFilter);
    this.showToast(`Filtered records with populated "${columnName}"`);
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
    if (typeof window !== "undefined" && !(window as any).__TAURI_INTERNALS__) {
      this.columnDistribution = mockDistribution;
      this.duplicateEntries = mockDuplicates;
      return;
    }
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

  async fetchCrossTabulation(primaryCol: string, secondaryCol: string) {
    if (!primaryCol || !secondaryCol) {
      this.pivotResult = null;
      return;
    }
    this.isLoadingPivot = true;
    try {
      const res = await invoke<CrossTabulationResult>("get_cross_tabulation", {
        primaryCol,
        secondaryCol,
        limit: 150,
        filterText: this.searchFilter || null,
      });
      this.pivotResult = res;
    } catch (e: any) {
      console.error("Pivot error:", e);
      this.showToast(`Pivot calculation failed: ${e.message || e}`);
    } finally {
      this.isLoadingPivot = false;
    }
  }

  async exportCrossTabulation(primaryCol: string, secondaryCol: string) {
    try {
      const pClean = primaryCol.replace(/[^a-zA-Z0-9_]/g, "_");
      const sClean = secondaryCol.replace(/[^a-zA-Z0-9_]/g, "_");
      const defaultName = `pivot_${pClean}_by_${sClean}_${Date.now()}.csv`;
      const path = await invoke<string | null>("pick_export_file", { defaultName, isJson: false });
      if (path) {
        this.showToast(`Exporting pivot matrix to ${path}...`);
        const count = await invoke<number>("export_cross_tabulation", {
          primaryCol,
          secondaryCol,
          exportPath: path,
          filterText: this.searchFilter || null,
        });
        this.showToast(`Successfully exported pivot matrix for ${count.toLocaleString()} entries!`);
      }
    } catch (e: any) {
      this.showToast(`Pivot export failed: ${e.message || e}`);
    }
  }

  async exportBreakdownPairs(primaryCol: string, secondaryCol: string) {
    try {
      const pClean = primaryCol.replace(/[^a-zA-Z0-9_]/g, "_");
      const sClean = secondaryCol.replace(/[^a-zA-Z0-9_]/g, "_");
      const defaultName = `breakdown_${pClean}_by_${sClean}_${Date.now()}.csv`;
      const path = await invoke<string | null>("pick_export_file", { defaultName, isJson: false });
      if (path) {
        this.showToast(`Exporting complete 3-column breakdown to ${path}...`);
        const count = await invoke<number>("export_breakdown_pairs", {
          primaryCol,
          secondaryCol,
          exportPath: path,
          filterText: this.searchFilter || null,
        });
        this.showToast(`Successfully exported ${count.toLocaleString()} distinct pairs to CSV!`);
      }
    } catch (e: any) {
      this.showToast(`Breakdown export failed: ${e.message || e}`);
    }
  }

  async exportCustomColumns(columns: string[], deduplicate = true, includeCounts = false, useFilter = true) {
    if (!columns || columns.length === 0) {
      this.showToast("No columns selected for export.");
      return;
    }
    try {
      const colPrefix = columns.length === 1 ? columns[0].replace(/[^a-zA-Z0-9_]/g, "_") : `custom_${columns.length}_cols`;
      const countSuffix = includeCounts ? "counts_" : "";
      const defaultName = `${colPrefix}_${deduplicate || includeCounts ? "unique_" : ""}${countSuffix}${Date.now()}.csv`;
      const path = await invoke<string | null>("pick_export_file", { defaultName, isJson: false });
      if (path) {
        this.showToast(`Exporting ${columns.length} columns to ${path}...`);
        const count = await invoke<number>("export_custom_columns", {
          columns,
          exportPath: path,
          deduplicate,
          includeCounts,
          filterText: useFilter ? (this.searchFilter || null) : null,
        });
        this.showToast(`Successfully exported ${count.toLocaleString()} rows to CSV!`);
      }
    } catch (e: any) {
      this.showToast(`Export failed: ${e.message || e}`);
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

  async exportColumnFrequency(columnName: string, useFilter = false, customDefaultName?: string) {
    try {
      const cleanCol = columnName.replace(/[^a-zA-Z0-9_]/g, "_");
      const defaultName = customDefaultName || `${cleanCol}_unique_${Date.now()}.csv`;
      const path = await invoke<string | null>("pick_export_file", { defaultName, isJson: false });
      if (path) {
        this.showToast(`Exporting unique values for ${columnName}...`);
        const count = await invoke<number>("export_column_frequency", {
          columnName,
          exportPath: path,
          filterText: useFilter ? (this.searchFilter || null) : null,
        });
        this.showToast(`Exported ${count.toLocaleString()} unique values to CSV!`);
      }
    } catch (e: any) {
      this.showToast(`Frequency export failed: ${e.message || e}`);
    }
  }

  async exportUniqueCompanies(useFilter = true) {
    const cols = (this.tableData?.columns && this.tableData.columns.length > 0)
      ? this.tableData.columns
      : this.completeness.map((c) => c.original_name || c.column_name);

    if (!cols || cols.length === 0) {
      this.showToast("No columns available to export.");
      return;
    }

    // High-priority company column candidates
    const directCandidates = [
      "source.name",
      "source_name",
      "company",
      "company_name",
      "employer",
      "employer_name",
      "brand_name",
      "brand",
      "name",
    ];

    let compCol = cols.find((c) => {
      const low = c.toLowerCase();
      return directCandidates.includes(low);
    });

    // Fallback: search for columns containing company/employer or ending with .name / _name
    if (!compCol) {
      compCol = cols.find((c) => {
        const low = c.toLowerCase();
        return (
          (low.includes("company") ||
            low.includes("employer") ||
            low.endsWith(".name") ||
            low.endsWith("_name")) &&
          !low.includes("user") &&
          !low.includes("file") &&
          !low.includes("first") &&
          !low.includes("last")
        );
      });
    }

    if (!compCol) {
      this.showToast("Could not find a company column in this feed.");
      return;
    }

    const defaultName = `unique_companies_${Date.now()}.csv`;
    await this.exportColumnFrequency(compCol, useFilter, defaultName);
  }
}

export const feedState = new FeedLensState();
