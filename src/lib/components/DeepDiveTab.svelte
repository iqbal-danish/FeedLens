<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { onMount } from "svelte";
  import * as echarts from "echarts";
  import {
    PieChart,
    BarChart2,
    AlertCircle,
    CheckCircle,
    RefreshCw,
    Download,
    Search,
    ChevronDown,
    X,
  } from "@lucide/svelte";

  let chartContainer: HTMLDivElement;
  let chartInstance: echarts.ECharts | null = null;
  let chartType = $state<"bar" | "pie">("bar");

  let isDropdownOpen = $state(false);
  let searchQuery = $state("");
  let filterCategory = $state<"all" | "complete" | "partial">("all");
  let tableSearchQuery = $state("");
  let dropdownRef = $state<HTMLDivElement>();
  let searchInputRef = $state<HTMLInputElement>();

  let completeCount = $derived(feedState.completeness.filter((c) => c.fill_rate >= 100).length);
  let partialCount = $derived(feedState.completeness.filter((c) => c.fill_rate < 100).length);

  let filteredColumns = $derived(() => {
    return feedState.completeness.filter((c) => {
      const q = searchQuery.toLowerCase();
      const matchesSearch = c.column_name.toLowerCase().includes(q) ||
        c.original_name.toLowerCase().includes(q);
      if (!matchesSearch) return false;
      if (filterCategory === "complete") return c.fill_rate >= 100;
      if (filterCategory === "partial") return c.fill_rate < 100;
      return true;
    });
  });

  let currentColumnInfo = $derived(() => {
    return feedState.completeness.find((c) => c.column_name === feedState.selectedColumn);
  });

  let filteredDistribution = $derived(() => {
    if (!tableSearchQuery.trim()) return feedState.columnDistribution;
    const q = tableSearchQuery.toLowerCase();
    return feedState.columnDistribution.filter((d) => d.value.toLowerCase().includes(q));
  });

  function selectColumn(name: string) {
    feedState.selectedColumn = name;
    feedState.fetchDeepDive(name);
    isDropdownOpen = false;
    searchQuery = "";
  }

  function toggleDropdown() {
    isDropdownOpen = !isDropdownOpen;
    if (isDropdownOpen) {
      setTimeout(() => searchInputRef?.focus(), 50);
    }
  }

  function handleDocumentClick(e: MouseEvent) {
    if (isDropdownOpen && dropdownRef && !dropdownRef.contains(e.target as Node)) {
      isDropdownOpen = false;
    }
  }

  function updateChart() {
    if (!chartContainer || feedState.columnDistribution.length === 0) return;

    if (!chartInstance) {
      chartInstance = echarts.init(chartContainer);
    }

    const top15 = feedState.columnDistribution.slice(0, 15);

    if (chartType === "bar") {
      const categories = top15.map((d) => (d.value.length > 25 ? d.value.slice(0, 25) + "..." : d.value)).reverse();
      const counts = top15.map((d) => d.count).reverse();

      const option: echarts.EChartsOption = {
        backgroundColor: "transparent",
        tooltip: {
          trigger: "axis",
          backgroundColor: "#0f172a",
          borderColor: "#334155",
          borderWidth: 1,
          padding: [8, 12],
          textStyle: { color: "#f8fafc" },
          formatter: (params: any) => {
            const item = params[0];
            return `<div class="font-sans text-xs">
              <span class="font-semibold text-slate-300">${item.name}</span>: 
              <span class="font-bold text-white font-mono">${item.value.toLocaleString()}</span> records
            </div>`;
          },
        },
        grid: { left: "3%", right: "8%", bottom: "3%", top: "4%", containLabel: true },
        xAxis: {
          type: "value",
          axisLabel: { color: "#94a3b8", fontSize: 11 },
          splitLine: { lineStyle: { color: "#1e293b", opacity: 0.8 } },
        },
        yAxis: {
          type: "category",
          data: categories,
          axisLabel: { color: "#cbd5e1", fontSize: 11 },
          axisLine: { lineStyle: { color: "#334155" } },
        },
        series: [
          {
            name: "Count",
            type: "bar",
            data: counts,
            itemStyle: {
              color: "#3b82f6",
              borderRadius: [0, 3, 3, 0],
            },
            label: {
              show: true,
              position: "right",
              color: "#94a3b8",
              fontSize: 10,
              formatter: "{c}",
            },
          },
        ],
      };
      chartInstance.setOption(option, true);
    } else {
      const pieData = top15.map((d) => ({
        name: d.value.length > 20 ? d.value.slice(0, 20) + "..." : d.value,
        value: d.count,
      }));

      const option: echarts.EChartsOption = {
        backgroundColor: "transparent",
        tooltip: {
          trigger: "item",
          backgroundColor: "#0f172a",
          borderColor: "#334155",
          borderWidth: 1,
          padding: [8, 12],
          textStyle: { color: "#f8fafc" },
          formatter: "{b}: {c} ({d}%)",
        },
        series: [
          {
            name: "Distribution",
            type: "pie",
            radius: ["42%", "72%"],
            center: ["50%", "50%"],
            avoidLabelOverlap: true,
            itemStyle: {
              borderRadius: 4,
              borderColor: "#0f172a",
              borderWidth: 2,
            },
            label: {
              show: true,
              color: "#cbd5e1",
              fontSize: 11,
              formatter: "{b}: {d}%",
            },
            data: pieData,
          },
        ],
      };
      chartInstance.setOption(option, true);
    }
  }

  $effect(() => {
    if (feedState.columnDistribution.length > 0 && chartType) {
      updateChart();
    }
  });

  onMount(() => {
    const handleResize = () => chartInstance?.resize();
    window.addEventListener("resize", handleResize);
    document.addEventListener("click", handleDocumentClick);
    return () => {
      window.removeEventListener("resize", handleResize);
      document.removeEventListener("click", handleDocumentClick);
      chartInstance?.dispose();
    };
  });
</script>

<div class="p-6 h-full flex flex-col space-y-5 overflow-y-auto w-full">
  <!-- Attribute Selector Header Panel -->
  <div class="flex flex-col lg:flex-row items-start lg:items-center justify-between gap-4 p-4 rounded-2xl data-card">
    <div class="relative flex items-center gap-3" bind:this={dropdownRef}>
      <span class="text-xs font-semibold uppercase tracking-wider text-slate-400 shrink-0">Select Attribute:</span>
      
      <!-- Custom Dropdown Trigger Button -->
      <div class="relative">
        <button
          type="button"
          onclick={toggleDropdown}
          class="flex items-center justify-between gap-3 bg-slate-950 hover:bg-slate-900 border border-slate-700/80 hover:border-slate-500 text-white rounded-xl px-3.5 py-2 text-xs font-mono transition-all duration-150 group min-w-[320px] max-w-lg focus:outline-none focus:ring-1 focus:ring-slate-500 cursor-pointer"
          title={feedState.selectedColumn || "Select attribute..."}
        >
          <div class="flex items-center gap-2 truncate flex-1 min-w-0">
            <span class="w-2 h-2 rounded-full bg-blue-500 shrink-0"></span>
            <span class="font-bold text-slate-100 truncate" title={feedState.selectedColumn}>
              {feedState.selectedColumn || "Select attribute..."}
            </span>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            {#if currentColumnInfo()}
              <span class="px-2 py-0.5 rounded-md text-[11px] font-mono font-medium bg-slate-900 border border-slate-800 text-slate-300">
                {currentColumnInfo()!.fill_rate.toFixed(0)}% fill
              </span>
            {/if}
            <ChevronDown class="w-4 h-4 text-slate-400 group-hover:text-slate-200 transition-transform duration-200 {isDropdownOpen ? 'rotate-180' : ''}" />
          </div>
        </button>

        <!-- Floating Dropdown Menu -->
        {#if isDropdownOpen}
          <div
            class="absolute top-full left-0 mt-2 w-[540px] max-w-[92vw] max-h-[480px] bg-slate-900/98 border border-slate-700/80 rounded-2xl shadow-2xl backdrop-blur-2xl z-50 flex flex-col overflow-hidden animate-in fade-in slide-in-from-top-2 duration-150"
            style="box-shadow: 0 20px 50px -10px rgba(0,0,0,0.8), 0 0 0 1px rgba(255,255,255,0.06);"
          >
            <!-- Search Bar -->
            <div class="p-3 border-b border-slate-800 bg-slate-950/80 shrink-0">
              <div class="relative">
                <Search class="w-4 h-4 text-slate-400 absolute left-3 top-2.5 pointer-events-none" />
                <input
                  bind:this={searchInputRef}
                  bind:value={searchQuery}
                  type="text"
                  placeholder="Search attribute name..."
                  class="w-full bg-slate-900 border border-slate-800 rounded-xl pl-9 pr-8 py-2 text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-slate-600 font-mono transition"
                />
                {#if searchQuery}
                  <button
                    type="button"
                    onclick={() => (searchQuery = "")}
                    class="absolute right-2.5 top-2 text-slate-500 hover:text-slate-300 p-0.5 rounded transition cursor-pointer"
                  >
                    <X class="w-3.5 h-3.5" />
                  </button>
                {/if}
              </div>

              <!-- Quick Filter Tabs (Clean Matte Buttons) -->
              <div class="flex items-center gap-1.5 mt-2.5 pt-1">
                <button
                  type="button"
                  onclick={() => (filterCategory = "all")}
                  class="px-2.5 py-1 rounded-lg text-[10px] font-semibold border transition cursor-pointer {filterCategory === 'all' ? 'bg-slate-800 text-white border-slate-600' : 'bg-slate-900/80 text-slate-400 hover:text-slate-200 border-slate-800'}"
                >
                  All ({feedState.completeness.length})
                </button>
                <button
                  type="button"
                  onclick={() => (filterCategory = "complete")}
                  class="px-2.5 py-1 rounded-lg text-[10px] font-semibold border transition cursor-pointer {filterCategory === 'complete' ? 'bg-slate-800 text-white border-slate-600' : 'bg-slate-900/80 text-slate-400 hover:text-slate-200 border-slate-800'}"
                >
                  100% Fill ({completeCount})
                </button>
                <button
                  type="button"
                  onclick={() => (filterCategory = "partial")}
                  class="px-2.5 py-1 rounded-lg text-[10px] font-semibold border transition cursor-pointer {filterCategory === 'partial' ? 'bg-slate-800 text-white border-slate-600' : 'bg-slate-900/80 text-slate-400 hover:text-slate-200 border-slate-800'}"
                >
                  Partial &lt;100% ({partialCount})
                </button>
              </div>
            </div>

            <!-- Options Scroll List (Matte text, no neon pills) -->
            <div class="flex-1 overflow-y-auto p-2 space-y-1 max-h-[300px]">
              {#if filteredColumns().length === 0}
                <div class="py-8 text-center text-xs text-slate-500 font-mono">
                  No attributes matching "{searchQuery}"
                </div>
              {:else}
                {#each filteredColumns() as col}
                  {@const isSelected = col.column_name === feedState.selectedColumn}
                  <button
                    type="button"
                    onclick={() => selectColumn(col.column_name)}
                    class="w-full text-left flex items-center justify-between gap-3 px-3 py-2 rounded-xl text-xs font-mono transition-colors group cursor-pointer {isSelected ? 'bg-slate-800 text-white font-bold' : 'hover:bg-slate-800/60 text-slate-300'}"
                    title={col.original_name}
                  >
                    <div class="flex items-center gap-2 truncate flex-1 min-w-0" title={col.original_name}>
                      <span class="w-1.5 h-1.5 rounded-full {isSelected ? 'bg-blue-400' : 'bg-slate-600'} shrink-0"></span>
                      <span class="truncate {isSelected ? 'text-white' : 'text-slate-200'}">{col.original_name}</span>
                    </div>
                    <div class="flex items-center gap-3 shrink-0">
                      <span class="text-[11px] text-slate-500 font-normal">{col.unique_count.toLocaleString()} unique</span>
                      <span class="text-[11px] font-mono font-medium {col.fill_rate >= 100 ? 'text-emerald-400' : col.fill_rate >= 60 ? 'text-amber-400' : 'text-rose-400'}">
                        {col.fill_rate.toFixed(0)}%
                      </span>
                    </div>
                  </button>
                {/each}
              {/if}
            </div>

            <!-- Dropdown Footer -->
            <div class="px-3 py-2 border-t border-slate-800/80 bg-slate-950/80 flex items-center justify-between text-[11px] text-slate-500 font-mono shrink-0">
              <span>{filteredColumns().length} attributes shown</span>
              <span>Click to select</span>
            </div>
          </div>
        {/if}
      </div>
    </div>

    <!-- Quick Stats for chosen col (Matte Neutral Badges) -->
    {#if currentColumnInfo()}
      <div class="flex flex-wrap items-center gap-2 text-xs font-mono">
        <!-- Total Count Metric -->
        <div class="flex items-center gap-2 bg-slate-950/80 px-3 py-1.5 rounded-xl border border-slate-800/80">
          <span class="text-slate-400">Total Count:</span>
          <span class="font-bold text-slate-100">
            {currentColumnInfo()!.valid_count.toLocaleString()}
            {#if currentColumnInfo()!.total_records > currentColumnInfo()!.valid_count}
              <span class="text-slate-500 font-normal"> / {currentColumnInfo()!.total_records.toLocaleString()}</span>
            {/if}
          </span>
        </div>

        <!-- Distinct Uniques Metric -->
        <div class="flex items-center gap-2 bg-slate-950/80 px-3 py-1.5 rounded-xl border border-slate-800/80">
          <span class="text-slate-400">Distinct Uniques:</span>
          <span class="font-bold text-slate-100">
            {currentColumnInfo()!.unique_count.toLocaleString()}
          </span>
        </div>

        <!-- Fill Rate Metric -->
        <div class="flex items-center gap-2 bg-slate-950/80 px-3 py-1.5 rounded-xl border border-slate-800/80">
          <span class="text-slate-400">Fill:</span>
          <span class="font-bold text-slate-100">
            {currentColumnInfo()!.fill_rate.toFixed(1)}%
          </span>
        </div>

        <!-- Uniqueness Ratio -->
        <div class="flex items-center gap-2 bg-slate-950/80 px-3 py-1.5 rounded-xl border border-slate-800/80">
          <span class="text-slate-400">Uniqueness:</span>
          <span class="font-bold text-slate-100">
            {currentColumnInfo()!.valid_count > 0 ? ((currentColumnInfo()!.unique_count / currentColumnInfo()!.valid_count) * 100).toFixed(1) : 0}%
          </span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Chart and Duplicates Section (Top) -->
  <div class="grid grid-cols-1 lg:grid-cols-2 gap-5">
    <!-- Value Distribution Chart -->
    <div class="p-5 rounded-2xl data-card flex flex-col">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h3 class="text-sm font-bold text-white tracking-tight">Top Frequency Distribution</h3>
          <p class="text-xs text-slate-400">Most frequent values and percentage share</p>
        </div>
        <!-- Chart type toggles (Matte styling) -->
        <div class="flex items-center gap-1 bg-slate-950 p-1 rounded-xl border border-slate-800/80">
          <button
            onclick={() => (chartType = "bar")}
            class="px-2.5 py-1 rounded-lg text-xs font-semibold flex items-center gap-1.5 {chartType === 'bar' ? 'bg-slate-800 text-white border border-slate-700' : 'text-slate-400 hover:text-slate-200'} transition cursor-pointer"
            title="Horizontal Bar Chart"
          >
            <BarChart2 class="w-3.5 h-3.5" />
            <span>Bars</span>
          </button>
          <button
            onclick={() => (chartType = "pie")}
            class="px-2.5 py-1 rounded-lg text-xs font-semibold flex items-center gap-1.5 {chartType === 'pie' ? 'bg-slate-800 text-white border border-slate-700' : 'text-slate-400 hover:text-slate-200'} transition cursor-pointer"
            title="Donut Chart"
          >
            <PieChart class="w-3.5 h-3.5" />
            <span>Donut</span>
          </button>
        </div>
      </div>

      <div bind:this={chartContainer} class="w-full h-80 min-h-[300px]"></div>
    </div>

    <!-- Duplicate / Collision Finder -->
    <div class="p-5 rounded-2xl data-card flex flex-col">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h3 class="text-sm font-bold text-white flex items-center gap-2 tracking-tight">
            <span>Collision & Duplicate Tracker</span>
            <span class="px-2 py-0.5 rounded-md text-[10px] font-mono bg-slate-900 border border-slate-800 text-slate-400">
              {#if feedState.duplicateEntries.length > 0}
                {feedState.duplicateEntries.length} Collisions
              {:else}
                Unique
              {/if}
            </span>
          </h3>
          <p class="text-xs text-slate-400">Values that occur multiple times for this attribute</p>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto max-h-80 border border-slate-800/80 rounded-xl isolate bg-slate-950/50">
        {#if feedState.duplicateEntries.length > 0}
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-950 sticky top-0 z-10 text-slate-400 font-semibold uppercase text-[10px] border-b border-slate-800">
              <tr class="bg-slate-950">
                <th class="py-2.5 px-3.5 bg-slate-950">Duplicate Value</th>
                <th class="py-2.5 px-3.5 text-right bg-slate-950">Occurrences</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-800/80 font-mono text-slate-300">
              {#each feedState.duplicateEntries as entry}
                <tr class="hover:bg-slate-800/40 transition-colors">
                  <td class="py-2.5 px-3.5 text-slate-200 truncate max-w-[240px]" title={entry.key}>
                    {entry.key}
                  </td>
                  <!-- Matte, clean occurrence count badge (no glossy pink/red box) -->
                  <td class="py-2.5 px-3.5 text-right font-medium text-slate-300">
                    <span class="px-2 py-0.5 rounded bg-slate-900 border border-slate-800 text-slate-300 font-mono text-xs">
                      {entry.count.toLocaleString()}x
                    </span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <div class="h-full flex flex-col items-center justify-center p-8 text-center text-slate-500">
            <CheckCircle class="w-8 h-8 text-slate-400 mb-2.5 opacity-80" />
            <p class="text-xs font-semibold text-slate-200">Zero Duplicates Detected</p>
            <p class="text-[11px] text-slate-500 mt-1">All populated values for this attribute are completely unique.</p>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Detailed Frequency Table (Full Width, Expanded Below) -->
  <div class="p-5 rounded-2xl data-card flex flex-col">
    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 mb-4">
      <div>
        <div class="flex items-center gap-2">
          <h3 class="text-sm font-bold text-white tracking-tight">Top Frequency Breakdown</h3>
          <span class="px-2 py-0.5 rounded-md text-[10px] font-mono text-slate-400 bg-slate-900 border border-slate-800">
            {filteredDistribution().length} values
          </span>
        </div>
        <p class="text-xs text-slate-400 mt-0.5">Complete ranking of the most frequent values for <span class="text-slate-300 font-mono">{feedState.selectedColumn}</span></p>
      </div>

      <div class="flex items-center gap-2.5">
        <!-- Live Table Filter -->
        <div class="relative">
          <Search class="w-3.5 h-3.5 text-slate-500 absolute left-2.5 top-2.5 pointer-events-none" />
          <input
            bind:value={tableSearchQuery}
            type="text"
            placeholder="Filter values..."
            class="bg-slate-950/80 border border-slate-800 rounded-xl pl-8 pr-7 py-1.5 text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-slate-600 font-mono transition w-44 focus:w-56"
          />
          {#if tableSearchQuery}
            <button
              type="button"
              onclick={() => (tableSearchQuery = "")}
              class="absolute right-2 top-2 text-slate-500 hover:text-slate-300 p-0.5 rounded transition cursor-pointer"
            >
              <X class="w-3 h-3" />
            </button>
          {/if}
        </div>

        <!-- Export CSV Button (Clean matte slate/blue button) -->
        <button
          onclick={() => feedState.exportColumnFrequency(feedState.selectedColumn)}
          class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-semibold bg-blue-600 hover:bg-blue-500 text-white transition cursor-pointer"
          title="Export this column's frequency distribution to CSV"
        >
          <Download class="w-3.5 h-3.5 text-white" />
          <span>Export to CSV</span>
        </button>
      </div>
    </div>

    <div class="overflow-x-auto border border-slate-800/80 rounded-xl overflow-y-auto isolate bg-slate-950/40" style="max-height: calc(50 * 2.5rem + 2.5rem);">
      <table class="w-full text-left text-xs border-collapse">
        <thead class="bg-slate-950 sticky top-0 z-10 text-slate-400 font-semibold uppercase text-[10px] border-b border-slate-800">
          <tr class="bg-slate-950">
            <th class="py-2.5 px-3.5 w-16 text-center text-slate-500 bg-slate-950">Rank</th>
            <th class="py-2.5 px-3.5 bg-slate-950">Value</th>
            <th class="py-2.5 px-3.5 text-right bg-slate-950 w-28">Count</th>
            <th class="py-2.5 px-3.5 text-right bg-slate-950 w-48">Distribution Share</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/80 font-mono text-slate-300">
          {#if filteredDistribution().length === 0}
            <tr>
              <td colspan="4" class="py-8 text-center text-xs text-slate-500 font-mono">
                No values match "{tableSearchQuery}"
              </td>
            </tr>
          {:else}
            {#each filteredDistribution() as dist, i}
              <tr class="hover:bg-slate-800/40 transition-colors">
                <!-- Clean, matte rank number (no glowing colorful circles) -->
                <td class="py-2.5 px-3.5 text-center text-slate-500 text-[11px]">
                  {i + 1}
                </td>
                <td class="py-2.5 px-3.5 font-medium text-slate-200 truncate max-w-md" title={dist.value}>
                  {dist.value}
                </td>
                <td class="py-2.5 px-3.5 text-right text-slate-200 font-medium">{dist.count.toLocaleString()}</td>
                <td class="py-2.5 px-3.5 text-right">
                  <div class="flex items-center justify-end gap-2.5">
                    <div class="w-24 bg-slate-800/80 rounded-full h-1.5 overflow-hidden">
                      <div class="bg-blue-500 h-full rounded-full" style="width: {dist.percentage}%"></div>
                    </div>
                    <span class="text-slate-400 text-xs w-12 text-right">{dist.percentage.toFixed(1)}%</span>
                  </div>
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>
  </div>
</div>