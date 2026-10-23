<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import {
    Search,
    ArrowUpDown,
    ChevronLeft,
    ChevronRight,
    Eye,
    RefreshCw,
    X,
    Copy,
    Check,
    RotateCcw,
    Sparkles,
  } from "@lucide/svelte";

  let searchInput = $state(feedState.searchFilter);
  $effect(() => {
    searchInput = feedState.searchFilter;
  });
  let fieldFilterQuery = $state("");
  let copiedKey = $state<string | null>(null);

  // Column prioritization: Populated first is always turned on by default
  let orderColumnsByFill = $state(true);

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      feedState.applySearch(searchInput);
    }
  }

  function handleSearchClear() {
    searchInput = "";
    feedState.applySearch("");
  }

  // Lookup map of column fill rates from DuckDB completeness analysis
  let columnFillRates = $derived(() => {
    const map = new Map<string, number>();
    for (const c of feedState.completeness) {
      map.set(c.column_name.toLowerCase(), c.fill_rate);
      map.set(c.original_name.toLowerCase(), c.fill_rate);
    }
    return map;
  });

  // Count of columns that are 100% empty (fill_rate < 0.1%)
  let emptyColumnsCount = $derived(() => {
    if (!feedState.tableData) return 0;
    const rates = columnFillRates();
    return feedState.tableData.columns.filter((col) => {
      const rate = rates.get(col.toLowerCase());
      return rate !== undefined && rate < 0.1;
    }).length;
  });

  // Derived target column if missing: or populated: filter is active
  let filterTargetCol = $derived(() => {
    if (!feedState.searchFilter) return null;
    const trimmed = feedState.searchFilter.trim();
    if (trimmed.startsWith("missing:")) {
      return trimmed.slice("missing:".length).trim().toLowerCase();
    }
    if (trimmed.startsWith("populated:")) {
      return trimmed.slice("populated:".length).trim().toLowerCase();
    }
    return null;
  });

  // Filtered & ordered visible columns list (populated first, with pinned filter target)
  let visibleColumns = $derived(() => {
    if (!feedState.tableData) return [];
    let cols = [...feedState.tableData.columns];
    const rates = columnFillRates();

    // Sort by fill rate descending so 100% full columns appear on the left
    if (orderColumnsByFill) {
      cols.sort((a, b) => {
        const rateA = rates.get(a.toLowerCase()) ?? 50;
        const rateB = rates.get(b.toLowerCase()) ?? 50;
        return rateB - rateA;
      });
    }

    // Pin target filtered column to the front so the user sees it immediately
    const target = filterTargetCol();
    if (target) {
      const idx = cols.findIndex(
        (c) => c.toLowerCase() === target || c.toLowerCase().replace(/[^a-z0-9_]/g, "_") === target
      );
      if (idx > -1) {
        const [pinned] = cols.splice(idx, 1);
        cols.unshift(pinned);
      }
    }

    return cols;
  });

  // Row-level health calculation for the currently selected record
  let selectedRowHealth = $derived(() => {
    if (!feedState.inspectingRecord) return { rate: 0, populated: 0, total: 0 };
    const keys = Object.keys(feedState.inspectingRecord);
    const populated = keys.filter((k) => {
      const val = feedState.inspectingRecord![k];
      return val !== null && val !== undefined && String(val).trim() !== "";
    }).length;
    const rate = keys.length > 0 ? (populated / keys.length) * 100 : 0;
    return { rate, populated, total: keys.length };
  });

  // Filtered fields inside the sidecar inspector drawer
  let filteredInspectorFields = $derived(() => {
    if (!feedState.inspectingRecord) return [];
    const entries = Object.entries(feedState.inspectingRecord);
    if (!fieldFilterQuery.trim()) return entries;
    const q = fieldFilterQuery.toLowerCase();
    return entries.filter(([k, v]) =>
      k.toLowerCase().includes(q) || (v && String(v).toLowerCase().includes(q))
    );
  });

  // Currently selected row index
  let currentRowIndex = $derived(() => {
    if (!feedState.tableData || !feedState.inspectingRecord) return -1;
    return feedState.tableData.rows.indexOf(feedState.inspectingRecord);
  });

  function selectPreviousRow() {
    if (!feedState.tableData || !feedState.inspectingRecord) return;
    const idx = currentRowIndex();
    if (idx > 0) {
      feedState.inspectingRecord = feedState.tableData.rows[idx - 1];
    }
  }

  function selectNextRow() {
    if (!feedState.tableData || !feedState.inspectingRecord) return;
    const idx = currentRowIndex();
    if (idx >= 0 && idx < feedState.tableData.rows.length - 1) {
      feedState.inspectingRecord = feedState.tableData.rows[idx + 1];
    }
  }

  function copyValue(key: string, val: string) {
    navigator.clipboard.writeText(val);
    copiedKey = key;
    setTimeout(() => {
      if (copiedKey === key) copiedKey = null;
    }, 2000);
  }

  function copyRecordJson() {
    if (!feedState.inspectingRecord) return;
    navigator.clipboard.writeText(JSON.stringify(feedState.inspectingRecord, null, 2));
    feedState.showToast("Copied full record JSON to clipboard!");
  }

  function closeInspector() {
    feedState.inspectingRecord = null;
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && feedState.inspectingRecord) {
      closeInspector();
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="p-6 h-full flex flex-col space-y-4 w-full font-sans select-none">
  <!-- Controls Bar -->
  <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-3 shrink-0">
    
    <!-- Left Controls: Search Box + Clear Filter Button + Space Saver Toggles -->
    <div class="flex items-center gap-2.5 flex-wrap flex-1">
      <!-- Search Input -->
      <div class="relative w-72 sm:w-80">
        <Search class="w-3.5 h-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
        <input
          type="text"
          placeholder="Search records... (Press Enter)"
          bind:value={searchInput}
          onkeydown={handleSearchKeydown}
          class="w-full bg-slate-900/90 border border-slate-800 rounded-xl pl-9 pr-14 py-2 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 transition shadow-sm font-sans"
        />
        <span class="absolute right-2 top-1/2 -translate-y-1/2 text-[9px] font-mono text-slate-500 bg-slate-800 px-1.5 py-0.5 rounded border border-slate-700 pointer-events-none">
          ↵
        </span>
      </div>

      <!-- Filter Button -->
      <button
        onclick={() => feedState.applySearch(searchInput)}
        class="px-3.5 py-2 rounded-xl text-xs font-bold text-white bg-blue-600 hover:bg-blue-500 transition cursor-pointer shadow-md shadow-blue-600/20 shrink-0"
      >
        Filter
      </button>

      <!-- Active Filter Banner / Clear Filter Button -->
      {#if feedState.searchFilter}
        {@const isMissing = feedState.searchFilter.startsWith("missing:")}
        {@const isPopulated = feedState.searchFilter.startsWith("populated:")}
        {#if isMissing}
          <div class="px-3 py-1.5 rounded-xl text-xs font-semibold backdrop-blur-md bg-rose-500/15 border border-rose-500/30 text-rose-300 flex items-center gap-2 shadow-sm shrink-0">
            <span>Missing: <strong class="text-white font-mono">{feedState.searchFilter.slice(8)}</strong></span>
            <button onclick={handleSearchClear} class="p-0.5 hover:text-white transition cursor-pointer" title="Clear filter">
              <X class="w-3.5 h-3.5" />
            </button>
          </div>
        {:else if isPopulated}
          <div class="px-3 py-1.5 rounded-xl text-xs font-semibold backdrop-blur-md bg-sky-500/15 border border-sky-500/30 text-sky-300 flex items-center gap-2 shadow-sm shrink-0">
            <span>Populated: <strong class="text-white font-mono">{feedState.searchFilter.slice(10)}</strong></span>
            <button onclick={handleSearchClear} class="p-0.5 hover:text-white transition cursor-pointer" title="Clear filter">
              <X class="w-3.5 h-3.5" />
            </button>
          </div>
        {:else}
          <button
            onclick={handleSearchClear}
            class="px-3 py-2 rounded-xl text-xs font-bold text-rose-400 bg-rose-500/15 hover:bg-rose-500/25 border border-rose-500/30 transition cursor-pointer flex items-center gap-1.5 shrink-0"
            title="Clear search filter and show all records"
          >
            <X class="w-3.5 h-3.5" />
            <span>Clear Filter</span>
          </button>
        {/if}
      {:else if searchInput}
        <button
          onclick={handleSearchClear}
          class="px-3 py-2 rounded-xl text-xs font-bold text-slate-400 bg-slate-800/80 hover:bg-slate-700 border border-slate-700 transition cursor-pointer flex items-center gap-1.5 shrink-0"
        >
          <X class="w-3.5 h-3.5" />
          <span>Clear</span>
        </button>
      {/if}

      <!-- Column Order: Populated First Toggle (Always ON on landing) -->
      <button
        onclick={() => (orderColumnsByFill = !orderColumnsByFill)}
        class="px-3 py-2 rounded-xl text-xs font-bold transition cursor-pointer flex items-center gap-1.5 border shrink-0 {orderColumnsByFill
          ? 'bg-blue-500/15 border-blue-500/35 text-blue-300'
          : 'bg-slate-900/80 border-slate-800 text-slate-400 hover:text-white hover:bg-slate-800'}"
        title="Sort columns by fill rate so 100% populated fields appear on the left"
      >
        <span>Populated First</span>
      </button>
    </div>

    <!-- Right Controls: Rows Per Page & Segmented Pagination -->
    <div class="flex items-center gap-3 shrink-0">
      <!-- Page Size Selector -->
      <div class="flex items-center gap-1.5 text-xs text-slate-400">
        <span>Rows:</span>
        <select
          bind:value={feedState.pageSize}
          onchange={() => {
            feedState.currentPage = 1;
            feedState.fetchTablePage();
          }}
          class="bg-slate-900 border border-slate-800 text-slate-200 rounded-lg px-2.5 py-1 text-xs focus:outline-none focus:border-blue-500 font-mono"
        >
          <option value={25}>25</option>
          <option value={50}>50</option>
          <option value={100}>100</option>
          <option value={250}>250</option>
        </select>
      </div>

      <!-- Pagination Segmented Controls -->
      {#if feedState.tableData}
        <div class="flex items-center gap-1 bg-slate-900/90 border border-slate-800 rounded-xl p-1 text-xs font-mono text-slate-300">
          <button
            onclick={() => feedState.setPage(feedState.currentPage - 1)}
            disabled={feedState.currentPage <= 1 || feedState.isLoadingTable}
            class="px-2 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-slate-800 transition cursor-pointer"
            title="Previous page"
          >
            <ChevronLeft class="w-3.5 h-3.5" />
          </button>
          <span class="px-2 text-white font-bold">
            {feedState.currentPage} / {feedState.tableData.total_pages}
          </span>
          <button
            onclick={() => feedState.setPage(feedState.currentPage + 1)}
            disabled={feedState.currentPage >= feedState.tableData.total_pages || feedState.isLoadingTable}
            class="px-2 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-slate-800 transition cursor-pointer"
            title="Next page"
          >
            <ChevronRight class="w-3.5 h-3.5" />
          </button>
        </div>
      {/if}
    </div>
  </div>

  <!-- Master-Detail Workspace: Table on Left + Sliding Inspector Drawer on Right -->
  <div class="flex-1 flex gap-4 overflow-hidden relative">
    
    <!-- Left Table Container -->
    <div class="flex-1 data-card rounded-2xl overflow-hidden flex flex-col relative transition-all duration-300">
      {#if feedState.isLoadingTable}
        <div class="absolute inset-0 bg-slate-950/40 backdrop-blur-[2px] z-20 flex items-center justify-center">
          <RefreshCw class="w-6 h-6 text-blue-400 animate-spin" />
        </div>
      {/if}

      <div class="overflow-x-auto overflow-y-auto flex-1 isolate">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-[#0b101b] sticky top-0 z-20 border-b border-white/[0.08] text-slate-300 font-bold uppercase tracking-wider text-[10.5px] select-none shadow-md">
            <tr>
              <th class="py-3 px-3 w-12 text-center text-slate-500 bg-slate-950/95 font-mono">#</th>
              {#each visibleColumns() as col}
                {@const isTargetCol = filterTargetCol() !== null && (filterTargetCol() === col.toLowerCase() || filterTargetCol() === col.toLowerCase().replace(/[^a-z0-9_]/g, "_"))}
                <th
                  class="py-3 px-3.5 cursor-pointer hover:text-white whitespace-nowrap bg-slate-950/95 font-sans transition-colors {isTargetCol ? 'bg-cyan-950/40 text-cyan-300' : ''}"
                  onclick={() => feedState.setSort(col)}
                >
                  <div class="flex items-center gap-1.5">
                    <span class="{isTargetCol ? 'text-cyan-300 font-bold' : ''}">{col}</span>
                    {#if isTargetCol}
                      <span class="px-1.5 py-0.2 rounded text-[9px] font-sans font-bold bg-cyan-500/20 text-cyan-300 border border-cyan-400/30">
                        TARGET
                      </span>
                    {/if}
                    <ArrowUpDown class="w-3 h-3 text-slate-500" />
                  </div>
                </th>
              {/each}
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-800/60 text-slate-300 font-sans">
            {#if feedState.tableData && feedState.tableData.rows.length > 0}
              {#each feedState.tableData.rows as row, idx}
                {@const isSelected = feedState.inspectingRecord === row}
                <tr
                  onclick={() => (feedState.inspectingRecord = row)}
                  class="transition cursor-pointer group {isSelected
                    ? 'bg-blue-600/15 border-l-2 border-l-blue-500'
                    : 'hover:bg-slate-800/40'}"
                >
                  <!-- Row index -->
                  <td class="py-2.5 px-3 text-center text-[11px] font-mono {isSelected ? 'text-blue-400 font-bold' : 'text-slate-500 group-hover:text-blue-400'}">
                    {(feedState.currentPage - 1) * feedState.pageSize + idx + 1}
                  </td>
                  
                  <!-- Visible Columns -->
                  {#each visibleColumns() as col}
                    {@const val = row[col]}
                    {@const isTargetCol = filterTargetCol() !== null && (filterTargetCol() === col.toLowerCase() || filterTargetCol() === col.toLowerCase().replace(/[^a-z0-9_]/g, "_"))}
                    <td class="py-2.5 px-3.5 max-w-[280px] truncate text-xs font-sans {isTargetCol ? 'bg-white/[0.02]' : ''}">
                      {#if val === null || val === undefined || String(val).trim() === ""}
                        {#if isTargetCol && feedState.searchFilter.startsWith("missing:")}
                          <span class="text-rose-400/80 font-mono text-[11px] italic">missing</span>
                        {:else}
                          <span class="text-slate-600 italic text-[11px] font-mono">—</span>
                        {/if}
                      {:else if val === "false"}
                        <span class="text-slate-500 font-mono text-[11px]">false</span>
                      {:else if val === "true"}
                        <span class="text-slate-300 font-mono text-[11px]">true</span>
                      {:else if String(val).startsWith("http://") || String(val).startsWith("https://")}
                        <span class="text-cyan-400 hover:underline truncate font-mono text-[11px]" title={val}>{val}</span>
                      {:else}
                        <span class="text-slate-200 font-sans" title={val}>{val}</span>
                      {/if}
                    </td>
                  {/each}
                </tr>
              {/each}
            {:else}
              <tr>
                <td colspan={100} class="py-12 text-center text-slate-500 italic">
                  No matching records found.
                </td>
              </tr>
            {/if}
          </tbody>
        </table>
      </div>

      <!-- Table Footer with Matching Count -->
      <div class="px-4 py-2.5 bg-slate-950/90 border-t border-slate-800 flex items-center justify-between text-xs text-slate-400 shrink-0 font-sans">
        <div class="flex items-center gap-2">
          <Eye class="w-3.5 h-3.5 text-blue-400" />
          <span class="hidden sm:inline">Click any row to inspect all fields in the sidecar drawer</span>
          <span class="sm:hidden">Click row to inspect</span>
        </div>
        {#if feedState.tableData}
          <div class="font-mono flex items-center gap-2 text-xs">
            <span class="w-2 h-2 rounded-full bg-cyan-400 animate-pulse"></span>
            <span>Total Matching: <strong class="text-white font-bold">{feedState.tableData.total_records.toLocaleString()}</strong> records</span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Right: Sliding Frosted Glass Inspector Drawer (Sidecar) -->
    {#if feedState.inspectingRecord}
      <div class="w-80 lg:w-96 glass-modal-panel rounded-2xl flex flex-col overflow-hidden shadow-2xl border border-white/15 animate-in slide-in-from-right-4 duration-200 shrink-0 font-sans">
        <!-- Drawer Header -->
        <div class="px-4 py-3 border-b border-white/10 flex items-center justify-between bg-white/[0.02]">
          <div class="flex items-center gap-2">
            <span class="text-base">🔎</span>
            <div>
              <h3 class="text-xs font-bold text-white tracking-tight leading-tight">
                Record Inspector #{currentRowIndex() >= 0 ? (feedState.currentPage - 1) * feedState.pageSize + currentRowIndex() + 1 : ""}
              </h3>
              <p class="text-[10px] text-slate-400">Full field attribute breakdown</p>
            </div>
          </div>
          <div class="flex items-center gap-1.5">
            <span class="text-[9px] font-mono text-slate-400 bg-white/5 border border-white/10 px-1.5 py-0.5 rounded select-none">
              Esc
            </span>
            <button
              onclick={closeInspector}
              class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition cursor-pointer"
              title="Close Drawer (Esc)"
            >
              <X class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- Row Quality Health Score Banner -->
        <div class="px-4 py-2.5 bg-white/[0.03] border-b border-white/10 flex items-center justify-between text-xs font-sans">
          <span class="text-slate-400 text-[11px]">Row Quality:</span>
          <div class="flex items-center gap-2">
            <div class="w-16 bg-slate-800 h-1.5 rounded-full overflow-hidden">
              <div
                class="h-full rounded-full {selectedRowHealth().rate >= 90 ? 'bg-emerald-500' : selectedRowHealth().rate >= 50 ? 'bg-amber-500' : 'bg-rose-500'}"
                style="width: {selectedRowHealth().rate}%"
              ></div>
            </div>
            <span class="font-mono text-xs font-bold {selectedRowHealth().rate >= 90 ? 'text-emerald-400' : selectedRowHealth().rate >= 50 ? 'text-amber-400' : 'text-rose-400'}">
              {selectedRowHealth().rate.toFixed(0)}%
            </span>
          </div>
        </div>

        <!-- Quick Field Search Filter -->
        <div class="p-3 border-b border-white/10 bg-white/[0.01]">
          <div class="relative">
            <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              type="text"
              placeholder="Search record attributes..."
              bind:value={fieldFilterQuery}
              class="w-full bg-slate-950/80 border border-slate-800 rounded-lg pl-8 pr-3 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 font-sans transition"
            />
          </div>
        </div>

        <!-- Scrollable Fields List with Modern Typography -->
        <div class="flex-1 overflow-y-auto p-3 space-y-2 font-sans text-xs">
          {#if filteredInspectorFields().length === 0}
            <div class="py-8 text-center text-xs text-slate-500">
              No fields matching "{fieldFilterQuery}"
            </div>
          {:else}
            {#each filteredInspectorFields() as [key, value]}
              {@const isPopulated = value !== null && value !== undefined && String(value).trim() !== ""}
              <div class="p-2.5 rounded-xl bg-slate-900/60 border border-slate-800/80 hover:border-slate-700 transition flex flex-col gap-1 group">
                <div class="flex items-center justify-between text-[10.5px]">
                  <span class="font-bold {isPopulated ? 'text-blue-400' : 'text-slate-500'} truncate max-w-[200px]" title={key}>
                    {key}
                  </span>
                  {#if isPopulated}
                    <button
                      onclick={() => copyValue(key, String(value))}
                      class="text-slate-500 hover:text-white transition cursor-pointer p-0.5"
                      title="Copy value"
                    >
                      {#if copiedKey === key}
                        <Check class="w-3 h-3 text-emerald-400" />
                      {:else}
                        <Copy class="w-3 h-3" />
                      {/if}
                    </button>
                  {:else}
                    <span class="text-rose-400/80 text-[9px] font-bold font-mono">EMPTY</span>
                  {/if}
                </div>

                <div class="text-xs break-all select-all font-sans">
                  {#if isPopulated}
                    {#if String(value).startsWith("http")}
                      <a href={value} target="_blank" rel="noreferrer" class="text-cyan-400 hover:underline font-mono text-[11px]">
                        {value}
                      </a>
                    {:else}
                      <span class="text-slate-200">{value}</span>
                    {/if}
                  {:else}
                    <span class="text-slate-600 italic font-mono text-[10px]">null</span>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>

        <!-- Drawer Footer: Prev / Next / Copy JSON -->
        <div class="p-3 border-t border-white/10 bg-white/[0.02] flex items-center justify-between gap-2 font-sans">
          <div class="flex items-center gap-1 font-mono text-xs">
            <button
              onclick={selectPreviousRow}
              disabled={currentRowIndex() <= 0}
              class="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-slate-800 text-slate-200 transition cursor-pointer text-xs"
              title="Previous record in table"
            >
              ‹ Prev
            </button>
            <button
              onclick={selectNextRow}
              disabled={!feedState.tableData || currentRowIndex() >= feedState.tableData.rows.length - 1}
              class="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-slate-800 text-slate-200 transition cursor-pointer text-xs"
              title="Next record in table"
            >
              Next ›
            </button>
          </div>

          <button
            onclick={copyRecordJson}
            class="px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white font-bold text-xs flex items-center gap-1.5 transition cursor-pointer shadow-sm font-sans"
          >
            <Copy class="w-3 h-3" />
            <span>Copy JSON</span>
          </button>
        </div>
      </div>
    {/if}

  </div>
</div>
