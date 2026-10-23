<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import {
    Search,
    ArrowUpDown,
    BarChart2,
    Download,
    ChevronDown,
    ChevronRight,
    Copy,
    Check,
    Key,
    Eye,
    X,
    SlidersHorizontal,
  } from "@lucide/svelte";

  let searchFilter = $state("");
  let currentTier = $state<"all" | "complete" | "warning" | "critical" | "keys">("all");
  let sortBy = $state<"name" | "type" | "fill" | "uniques" | "valid" | "missing" | "length">("fill");
  let sortAsc = $state(false);

  // Range Slider for Fill Rate
  let showSlider = $state(false);
  let minFillFilter = $state(0);
  let maxFillFilter = $state(100);

  // Expanded Row Accordion
  let expandedRow = $state<string | null>(null);
  let copiedText = $state<string | null>(null);

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    copiedText = text;
    feedState.showToast(`Copied "${text}" to clipboard`);
    setTimeout(() => {
      if (copiedText === text) copiedText = null;
    }, 2000);
  }

  function toggleRow(name: string) {
    expandedRow = expandedRow === name ? null : name;
  }

  function getMissingCount(col: { empty_count?: number; total_records: number; valid_count: number }): number {
    if (col.empty_count !== undefined && col.empty_count !== null && col.empty_count > 0) {
      return col.empty_count;
    }
    return Math.max(0, col.total_records - col.valid_count);
  }

  let completeCount = $derived(feedState.completeness.filter((c) => c.fill_rate >= 99.9).length);
  let warningCount = $derived(feedState.completeness.filter((c) => c.fill_rate >= 10.0 && c.fill_rate < 99.9).length);
  let criticalCount = $derived(feedState.completeness.filter((c) => c.fill_rate < 10.0).length);
  let keyCount = $derived(feedState.completeness.filter((c) => c.is_candidate_key).length);

  let filteredColumns = $derived(() => {
    let list = feedState.completeness.filter((c) => {
      // Text Search
      if (searchFilter.trim()) {
        const q = searchFilter.toLowerCase().trim();
        const matchesName = c.original_name.toLowerCase().includes(q) || c.column_name.toLowerCase().includes(q);
        const matchesType = (c.inferred_type || "").toLowerCase().includes(q);
        if (!matchesName && !matchesType) return false;
      }

      // Range Filter
      if (c.fill_rate < minFillFilter || c.fill_rate > maxFillFilter) {
        return false;
      }

      // Category Tiers
      if (currentTier === "complete") return c.fill_rate >= 99.9;
      if (currentTier === "warning") return c.fill_rate >= 10.0 && c.fill_rate < 99.9;
      if (currentTier === "critical") return c.fill_rate < 10.0;
      if (currentTier === "keys") return c.is_candidate_key;
      return true;
    });

    list.sort((a, b) => {
      let valA: any = a.fill_rate;
      let valB: any = b.fill_rate;
      if (sortBy === "name") {
        valA = a.original_name.toLowerCase();
        valB = b.original_name.toLowerCase();
      } else if (sortBy === "type") {
        valA = (a.inferred_type || "").toLowerCase();
        valB = (b.inferred_type || "").toLowerCase();
      } else if (sortBy === "uniques") {
        valA = a.unique_count;
        valB = b.unique_count;
      } else if (sortBy === "valid") {
        valA = a.valid_count;
        valB = b.valid_count;
      } else if (sortBy === "missing") {
        valA = getMissingCount(a);
        valB = getMissingCount(b);
      } else if (sortBy === "length") {
        valA = a.avg_length || 0;
        valB = b.avg_length || 0;
      }

      if (valA < valB) return sortAsc ? -1 : 1;
      if (valA > valB) return sortAsc ? 1 : -1;
      return 0;
    });

    return list;
  });

  function toggleSort(field: "name" | "type" | "fill" | "uniques" | "valid" | "missing" | "length") {
    if (sortBy === field) {
      sortAsc = !sortAsc;
    } else {
      sortBy = field;
      sortAsc = false;
    }
  }

  function setTier(tier: "all" | "complete" | "warning" | "critical" | "keys") {
    currentTier = tier;
  }

  function getHealthColor(rate: number) {
    if (rate >= 99.9) {
      return {
        bar: "bg-cyan-400",
        text: "text-cyan-400",
      };
    } else if (rate >= 10.0) {
      return {
        bar: "bg-indigo-400",
        text: "text-indigo-300",
      };
    } else {
      return {
        bar: "bg-rose-400",
        text: "text-rose-400",
      };
    }
  }

  function exportMatrixCsv() {
    if (feedState.completeness.length === 0) return;
    const headers = [
      "Attribute",
      "Inferred Type",
      "Fill Rate (%)",
      "Valid Rows",
      "Missing Rows",
      "Distinct Uniques",
      "Avg Length",
      "Min Length",
      "Max Length",
      "Is Candidate Key",
    ];
    const rows = feedState.completeness.map((c) => [
      `"${c.original_name.replace(/"/g, '""')}"`,
      `"${c.inferred_type || "Text"}"`,
      c.fill_rate.toFixed(1),
      c.valid_count,
      getMissingCount(c),
      c.unique_count,
      c.avg_length || 0,
      c.min_length || 0,
      c.max_length || 0,
      c.is_candidate_key ? "YES" : "NO",
    ]);
    const csvContent = [headers.join(","), ...rows.map((r) => r.join(","))].join("\n");
    const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.setAttribute("href", url);
    link.setAttribute("download", `feedlens_matrix_${Date.now()}.csv`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
    feedState.showToast("Exported completeness matrix CSV");
  }
</script>

<div class="p-6 h-full flex flex-col space-y-4 w-full">
  <!-- Controls Bar (Frosted Glass Header) -->
  <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-3 shrink-0">
    <div class="flex items-center gap-2 flex-wrap flex-1">
      <!-- Search Input -->
      <div class="relative w-64 sm:w-72">
        <Search class="w-3.5 h-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
        <input
          type="text"
          placeholder="Filter {feedState.completeness.length} attributes..."
          bind:value={searchFilter}
          class="w-full backdrop-blur-md bg-white/[0.04] border border-white/[0.08] hover:border-white/[0.14] rounded-xl pl-9 pr-8 py-2 text-xs text-white placeholder-slate-400 focus:outline-none focus:border-white/20 transition shadow-inner font-sans"
        />
        {#if searchFilter}
          <button
            onclick={() => (searchFilter = "")}
            class="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-white p-0.5 rounded transition cursor-pointer"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        {/if}
      </div>

      <!-- Clean Aesthetic Segmented Filter Tabs (No loud surrounded colors) -->
      <div class="flex items-center gap-1.5 flex-wrap">
        <!-- All -->
        <button
          onclick={() => setTier("all")}
          class="px-3 py-1.5 rounded-full text-xs font-medium flex items-center gap-1.5 transition cursor-pointer {currentTier === 'all'
            ? 'backdrop-blur-md bg-white/[0.12] border border-white/20 text-white shadow-sm'
            : 'backdrop-blur-md bg-white/[0.03] border border-white/[0.06] text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]'}"
        >
          <span>All</span>
          <span class="text-[10px] font-mono opacity-60">
            {feedState.completeness.length}
          </span>
        </button>

        <!-- Complete -->
        <button
          onclick={() => setTier("complete")}
          class="px-3 py-1.5 rounded-full text-xs font-medium flex items-center gap-1.5 transition cursor-pointer {currentTier === 'complete'
            ? 'backdrop-blur-md bg-white/[0.12] border border-white/20 text-white shadow-sm'
            : 'backdrop-blur-md bg-white/[0.03] border border-white/[0.06] text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]'}"
        >
          <span>100% Complete</span>
          <span class="text-[10px] font-mono opacity-60">
            {completeCount}
          </span>
        </button>

        <!-- Partial -->
        <button
          onclick={() => setTier("warning")}
          class="px-3 py-1.5 rounded-full text-xs font-medium flex items-center gap-1.5 transition cursor-pointer {currentTier === 'warning'
            ? 'backdrop-blur-md bg-white/[0.12] border border-white/20 text-white shadow-sm'
            : 'backdrop-blur-md bg-white/[0.03] border border-white/[0.06] text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]'}"
        >
          <span>Partial</span>
          <span class="text-[10px] font-mono opacity-60">
            {warningCount}
          </span>
        </button>

        <!-- Critical -->
        <button
          onclick={() => setTier("critical")}
          class="px-3 py-1.5 rounded-full text-xs font-medium flex items-center gap-1.5 transition cursor-pointer {currentTier === 'critical'
            ? 'backdrop-blur-md bg-white/[0.12] border border-white/20 text-white shadow-sm'
            : 'backdrop-blur-md bg-white/[0.03] border border-white/[0.06] text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]'}"
        >
          <span>Critical (&lt;10%)</span>
          <span class="text-[10px] font-mono opacity-60">
            {criticalCount}
          </span>
        </button>

        <!-- Candidate Keys -->
        {#if keyCount > 0}
          <button
            onclick={() => setTier("keys")}
            class="px-3 py-1.5 rounded-full text-xs font-medium flex items-center gap-1.5 transition cursor-pointer {currentTier === 'keys'
              ? 'backdrop-blur-md bg-white/[0.12] border border-white/20 text-white shadow-sm'
              : 'backdrop-blur-md bg-white/[0.03] border border-white/[0.06] text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]'}"
          >
            <Key class="w-3 h-3 text-slate-400" />
            <span>Keys</span>
            <span class="text-[10px] font-mono opacity-60">
              {keyCount}
            </span>
          </button>
        {/if}

        <!-- Fill Rate Range Slider Toggle -->
        <button
          onclick={() => (showSlider = !showSlider)}
          class="px-2.5 py-1.5 rounded-xl text-xs font-medium border transition cursor-pointer flex items-center gap-1.5 {showSlider || minFillFilter > 0 || maxFillFilter < 100 ? 'backdrop-blur-md bg-white/[0.12] border-white/20 text-white' : 'backdrop-blur-md bg-white/[0.03] border-white/[0.06] text-slate-400 hover:text-slate-200'}"
          title="Filter by Fill Rate % range"
        >
          <SlidersHorizontal class="w-3.5 h-3.5" />
          <span>{minFillFilter > 0 || maxFillFilter < 100 ? `${minFillFilter}% - ${maxFillFilter}%` : 'Threshold'}</span>
        </button>
      </div>
    </div>

    <!-- Right Controls: Count & Export -->
    <div class="flex items-center gap-3 shrink-0">
      <div class="text-xs text-slate-400 font-mono">
        Showing <span class="text-white font-medium">{filteredColumns().length}</span> of {feedState.completeness.length} attributes
      </div>
      <button
        onclick={exportMatrixCsv}
        class="px-3 py-1.5 rounded-xl backdrop-blur-md bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] hover:border-white/20 text-xs font-medium text-slate-200 hover:text-white flex items-center gap-2 transition cursor-pointer shadow-sm"
        title="Download matrix as CSV"
      >
        <Download class="w-3.5 h-3.5 text-slate-400" />
        <span>Export Matrix</span>
      </button>
    </div>
  </div>

  <!-- Interactive Threshold Range Slider Panel -->
  {#if showSlider}
    <div class="p-3.5 rounded-2xl backdrop-blur-xl bg-slate-900/50 border border-white/[0.08] flex flex-wrap items-center justify-between gap-4 shadow-xl">
      <div class="flex items-center gap-4 flex-1 min-w-[280px]">
        <span class="text-xs font-medium text-slate-300 shrink-0">Fill Rate Filter:</span>
        <div class="flex items-center gap-3 flex-1">
          <span class="text-xs font-mono text-slate-400 w-10 text-right">{minFillFilter}%</span>
          <input
            type="range"
            min="0"
            max="100"
            bind:value={minFillFilter}
            class="w-full accent-slate-300 h-1 bg-white/[0.08] rounded-lg cursor-pointer"
          />
          <span class="text-xs text-slate-500">to</span>
          <input
            type="range"
            min="0"
            max="100"
            bind:value={maxFillFilter}
            class="w-full accent-slate-300 h-1 bg-white/[0.08] rounded-lg cursor-pointer"
          />
          <span class="text-xs font-mono text-slate-400 w-10">{maxFillFilter}%</span>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <button
          onclick={() => { minFillFilter = 0; maxFillFilter = 100; }}
          class="px-2.5 py-1 rounded-lg text-[11px] font-medium text-slate-300 hover:text-white backdrop-blur-md bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.08] transition cursor-pointer"
        >
          Reset Range
        </button>
        <button
          onclick={() => (showSlider = false)}
          class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-white/[0.08] transition cursor-pointer"
          title="Close slider"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>
    </div>
  {/if}

  <!-- Frosted Glass Matrix Table Container -->
  <div class="flex-1 backdrop-blur-xl bg-slate-900/40 border border-white/[0.08] rounded-2xl overflow-hidden shadow-2xl flex flex-col min-h-0">
    <div class="overflow-x-auto overflow-y-auto flex-1 isolate">
      <table class="w-full text-left text-xs border-collapse">
        <thead class="bg-[#0b101b] sticky top-0 z-20 border-b border-white/[0.08] text-slate-400 font-semibold uppercase tracking-wider text-[10px] select-none shadow-md">
          <tr>
            <th class="py-3 px-4 cursor-pointer hover:text-white" onclick={() => toggleSort("name")}>
              <div class="flex items-center gap-1.5">
                <span>Attribute</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-3 cursor-pointer hover:text-white w-24" onclick={() => toggleSort("type")}>
              <div class="flex items-center gap-1.5">
                <span>Type</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-4 text-right cursor-pointer hover:text-white w-40" onclick={() => toggleSort("fill")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Fill Rate</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-4 text-right cursor-pointer hover:text-white w-28" onclick={() => toggleSort("valid")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Valid Rows</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-4 text-right cursor-pointer hover:text-white w-28" onclick={() => toggleSort("missing")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Missing</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-4 text-right cursor-pointer hover:text-white w-28" onclick={() => toggleSort("uniques")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Uniques</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-4 text-right cursor-pointer hover:text-white w-28" onclick={() => toggleSort("length")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Length</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-3">Sample Values</th>
            <th class="py-3 px-3 text-center w-24">Actions</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-white/[0.04] font-mono text-slate-300">
          {#if filteredColumns().length === 0}
            <tr>
              <td colspan="9" class="py-12 text-center text-slate-500 italic font-sans">
                No attributes match the selected filter or range criteria.
              </td>
            </tr>
          {:else}
            {#each filteredColumns() as col}
              {@const health = getHealthColor(col.fill_rate)}
              {@const isExpanded = expandedRow === col.column_name}
              {@const missingCount = getMissingCount(col)}

              <!-- Master Row -->
              <tr
                class="hover:bg-white/[0.035] transition-colors cursor-pointer {isExpanded ? 'bg-white/[0.04]' : ''}"
                onclick={() => toggleRow(col.column_name)}
              >
                <!-- Attribute Name with Expand Icon and Clean Key Icon -->
                <td class="py-2.5 px-4 font-medium text-white">
                  <div class="flex items-center gap-2">
                    <span class="text-slate-500 hover:text-white transition">
                      {#if isExpanded}
                        <ChevronDown class="w-3.5 h-3.5 text-slate-300" />
                      {:else}
                        <ChevronRight class="w-3.5 h-3.5" />
                      {/if}
                    </span>
                    <span class="font-mono text-slate-200 hover:text-white transition truncate max-w-xs sm:max-w-sm" title={col.original_name}>
                      {col.original_name}
                    </span>

                    <!-- Minimalist Candidate Key Indicator (No heavy colored badge) -->
                    {#if col.is_candidate_key}
                      <span class="text-slate-400 font-mono text-[10px] flex items-center gap-1 shrink-0" title="100% Unique: Candidate Primary Key">
                        <Key class="w-2.5 h-2.5" />
                        <span>key</span>
                      </span>
                    {/if}
                  </div>
                </td>

                <!-- Inferred Type (Clean typography, no surrounded colored boxes) -->
                <td class="py-2.5 px-3">
                  <span class="text-slate-400 font-mono text-xs">
                    {col.inferred_type || "Text"}
                  </span>
                </td>

                <!-- Fill Rate Progress Bar + Percentage (Vibrant Colors + Smooth Animation) -->
                <td class="py-2.5 px-4 text-right">
                  <div class="flex items-center justify-end gap-2.5">
                    <div class="w-16 bg-white/[0.08] rounded-full h-1.5 overflow-hidden hidden sm:block">
                      <div
                        class="h-full rounded-full {health.bar} animate-fill-bar transition-all duration-300"
                        style="--fill-width: {Math.max(2, col.fill_rate)}%; width: {Math.max(2, col.fill_rate)}%;"
                      ></div>
                    </div>
                    <span class="font-bold {health.text} font-mono text-xs w-12 text-right">
                      {col.fill_rate.toFixed(1)}%
                    </span>
                  </div>
                </td>

                <!-- Valid Rows (Clean typography, clickable) -->
                <td class="py-2.5 px-4 text-right">
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      feedState.filterByPopulated(col.original_name);
                    }}
                    class="font-mono text-slate-300 hover:text-white hover:underline transition cursor-pointer"
                    title="Click to view all {col.valid_count.toLocaleString()} populated rows in Data Explorer"
                  >
                    {col.valid_count.toLocaleString()}
                  </button>
                </td>

                <!-- Missing Rows (Clean typography, no surrounded red pill box) -->
                <td class="py-2.5 px-4 text-right font-mono">
                  {#if missingCount > 0}
                    <button
                      type="button"
                      onclick={(e) => {
                        e.stopPropagation();
                        feedState.filterByMissing(col.original_name);
                      }}
                      class="font-mono text-slate-400 hover:text-rose-300 hover:underline transition cursor-pointer"
                      title="Click to view all {missingCount.toLocaleString()} missing jobs in Data Explorer"
                    >
                      {missingCount.toLocaleString()}
                    </button>
                  {:else}
                    <span class="text-slate-600">—</span>
                  {/if}
                </td>

                <!-- Distinct Uniques -->
                <td class="py-2.5 px-4 text-right font-mono">
                  <span class="text-slate-400">
                    {col.unique_count.toLocaleString()}
                  </span>
                </td>

                <!-- Length Profile -->
                <td class="py-2.5 px-4 text-right font-mono text-xs">
                  {#if col.avg_length && col.avg_length > 0}
                    <span class="text-slate-400" title="Min: {col.min_length} chars, Max: {col.max_length} chars, Avg: {col.avg_length} chars">
                      {col.avg_length.toFixed(0)} <span class="text-slate-600 text-[10px]">avg</span>
                    </span>
                  {:else}
                    <span class="text-slate-600">—</span>
                  {/if}
                </td>

                <!-- Sample Values Tags -->
                <td class="py-2.5 px-3 max-w-xs truncate font-sans text-xs">
                  <div class="flex items-center gap-1.5 overflow-hidden">
                    {#if col.sample_values && col.sample_values.length > 0}
                      {#each col.sample_values.slice(0, 2) as sample}
                        <span class="text-slate-400 text-[11px] font-mono truncate max-w-[120px]" title={sample}>
                          {sample}
                        </span>
                      {/each}
                    {:else}
                      <span class="text-slate-600 font-mono text-xs">—</span>
                    {/if}
                  </div>
                </td>

                <!-- Actions: Eye Jump + Deep Dive (Clean subtle icons) -->
                <td class="py-2.5 px-3 text-center">
                  <div class="flex items-center justify-center gap-1.5">
                    {#if missingCount > 0}
                      <button
                        onclick={(e) => {
                          e.stopPropagation();
                          feedState.filterByMissing(col.original_name);
                        }}
                        class="p-1.5 rounded-lg hover:bg-white/[0.08] text-slate-500 hover:text-rose-300 transition cursor-pointer"
                        title="Filter {missingCount.toLocaleString()} missing records in Data Explorer"
                      >
                        <Eye class="w-3.5 h-3.5" />
                      </button>
                    {/if}
                    <button
                      onclick={(e) => {
                        e.stopPropagation();
                        feedState.fetchDeepDive(col.column_name);
                        feedState.activeTab = "deepdive";
                      }}
                      class="p-1.5 rounded-lg hover:bg-white/[0.08] text-slate-500 hover:text-white transition cursor-pointer"
                      title="Inspect in Deep Dive"
                    >
                      <BarChart2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>

              <!-- Expandable Inline Drawer (Clean Frosted Glass Inspection Panel) -->
              {#if isExpanded}
                <tr class="bg-white/[0.015]">
                  <td colspan="9" class="p-4 border-b border-white/[0.06] font-sans">
                    <div class="rounded-2xl p-5 backdrop-blur-2xl bg-slate-900/70 border border-white/[0.09] flex flex-col gap-4 shadow-2xl shadow-black/50">
                      <!-- Top Bar: Full Path + Copy Button + Quick Jump Actions -->
                      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-white/[0.07] pb-3">
                        <div class="flex items-center gap-2 flex-wrap">
                          <span class="text-xs font-medium text-slate-400 uppercase tracking-wider">Field Path:</span>
                          <code class="px-2.5 py-1 rounded-lg backdrop-blur-md bg-white/[0.05] border border-white/[0.08] text-xs font-mono text-slate-200 select-all">
                            {col.original_name}
                          </code>
                          <button
                            onclick={() => copyToClipboard(col.original_name)}
                            class="p-1 rounded-lg hover:bg-white/[0.08] text-slate-400 hover:text-white transition cursor-pointer"
                            title="Copy full attribute path"
                          >
                            {#if copiedText === col.original_name}
                              <Check class="w-3.5 h-3.5 text-slate-200" />
                            {:else}
                              <Copy class="w-3.5 h-3.5" />
                            {/if}
                          </button>
                        </div>

                        <!-- 1-Click Frosted Action Buttons (Aesthetic, neutral with subtle hover) -->
                        <div class="flex items-center gap-2 flex-wrap">
                          {#if missingCount > 0}
                            <button
                              onclick={() => feedState.filterByMissing(col.original_name)}
                              class="px-3 py-1.5 rounded-xl text-xs font-medium text-slate-300 hover:text-white backdrop-blur-md bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] flex items-center gap-1.5 transition cursor-pointer shadow-sm"
                              title="Show rows missing this attribute in Data Explorer"
                            >
                              <Eye class="w-3 h-3 text-slate-400" />
                              <span>View {missingCount.toLocaleString()} Missing</span>
                            </button>
                          {/if}

                          <button
                            onclick={() => feedState.filterByPopulated(col.original_name)}
                            class="px-3 py-1.5 rounded-xl text-xs font-medium text-slate-300 hover:text-white backdrop-blur-md bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] flex items-center gap-1.5 transition cursor-pointer shadow-sm"
                            title="Show rows populated with this attribute in Data Explorer"
                          >
                            <Eye class="w-3 h-3 text-slate-400" />
                            <span>View {col.valid_count.toLocaleString()} Populated</span>
                          </button>

                          <button
                            onclick={() => {
                              feedState.fetchDeepDive(col.column_name);
                              feedState.activeTab = "deepdive";
                            }}
                            class="px-3 py-1.5 rounded-xl text-xs font-medium text-slate-300 hover:text-white backdrop-blur-md bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] flex items-center gap-1.5 transition cursor-pointer shadow-sm"
                          >
                            <BarChart2 class="w-3 h-3 text-slate-400" />
                            <span>Deep Dive</span>
                          </button>
                        </div>
                      </div>

                      <!-- Middle Metrics Grid (Clean Translucent Cards) -->
                      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
                        <div class="p-3 rounded-xl backdrop-blur-md bg-white/[0.03] border border-white/[0.07] shadow-inner">
                          <span class="text-slate-400 block text-[11px]">Inferred Type:</span>
                          <span class="font-medium text-white font-mono mt-0.5 inline-block">
                            {col.inferred_type || "Text"}
                          </span>
                        </div>

                        <div class="p-3 rounded-xl backdrop-blur-md bg-white/[0.03] border border-white/[0.07] shadow-inner">
                          <span class="text-slate-400 block text-[11px]">Length Profile:</span>
                          <span class="font-medium text-white font-mono mt-0.5 inline-block">
                            Min {col.min_length || 0} · Max {col.max_length || 0} · Avg {col.avg_length ? col.avg_length.toFixed(1) : 0}
                          </span>
                        </div>

                        <div class="p-3 rounded-xl backdrop-blur-md bg-white/[0.03] border border-white/[0.07] shadow-inner">
                          <span class="text-slate-400 block text-[11px]">Uniqueness Ratio:</span>
                          <span class="font-medium text-white font-mono mt-0.5 inline-block">
                            {col.valid_count > 0 ? ((col.unique_count / col.valid_count) * 100).toFixed(1) : 0}% ({col.unique_count.toLocaleString()} uniques)
                          </span>
                        </div>

                        <div class="p-3 rounded-xl backdrop-blur-md bg-white/[0.03] border border-white/[0.07] shadow-inner">
                          <span class="text-slate-400 block text-[11px]">Key Candidate:</span>
                          <span class="font-medium text-slate-300 font-mono mt-0.5 inline-block">
                            {col.is_candidate_key ? "YES (Primary Identifier)" : "NO"}
                          </span>
                        </div>
                      </div>

                      <!-- Bottom Samples List (Clean Translucent Chips) -->
                      <div>
                        <span class="text-[11px] font-medium text-slate-400 uppercase tracking-wider block mb-2">
                          Sample Values ({col.sample_values ? col.sample_values.length : 0} inspected):
                        </span>
                        {#if col.sample_values && col.sample_values.length > 0}
                          <div class="flex flex-wrap gap-2 max-h-36 overflow-y-auto">
                            {#each col.sample_values as sample}
                              <button
                                onclick={() => copyToClipboard(sample)}
                                class="px-2.5 py-1 rounded-lg backdrop-blur-md bg-white/[0.03] hover:bg-white/[0.08] border border-white/[0.07] hover:border-white/[0.14] text-slate-300 text-xs font-mono truncate max-w-sm flex items-center gap-1.5 transition cursor-pointer"
                                title="Click to copy: {sample}"
                              >
                                <span class="truncate">{sample}</span>
                                {#if copiedText === sample}
                                  <Check class="w-3 h-3 text-slate-200 shrink-0" />
                                {:else}
                                  <Copy class="w-3 h-3 text-slate-500 opacity-60 shrink-0" />
                                {/if}
                              </button>
                            {/each}
                          </div>
                        {:else}
                          <p class="text-xs text-slate-500 italic font-sans">No populated values recorded for this attribute.</p>
                        {/if}
                      </div>
                    </div>
                  </td>
                </tr>
              {/if}
            {/each}
          {/if}
        </tbody>
      </table>
    </div>
  </div>
</div>
