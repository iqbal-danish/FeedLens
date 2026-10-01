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

  let currentColumnInfo = $derived(() => {
    return feedState.completeness.find((c) => c.column_name === feedState.selectedColumn);
  });

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
          backgroundColor: "#1e293b",
          borderColor: "#334155",
          textStyle: { color: "#f8fafc" },
          formatter: (params: any) => {
            const item = params[0];
            return `<div class="font-sans text-xs">
              <span class="font-bold text-blue-400">${item.name}</span>: ${item.value.toLocaleString()} records
            </div>`;
          },
        },
        grid: { left: "3%", right: "8%", bottom: "3%", top: "4%", containLabel: true },
        xAxis: {
          type: "value",
          axisLabel: { color: "#94a3b8" },
          splitLine: { lineStyle: { color: "#334155", opacity: 0.4 } },
        },
        yAxis: {
          type: "category",
          data: categories,
          axisLabel: { color: "#cbd5e1", fontSize: 11 },
          axisLine: { lineStyle: { color: "#475569" } },
        },
        series: [
          {
            name: "Count",
            type: "bar",
            data: counts,
            itemStyle: {
              color: new echarts.graphic.LinearGradient(0, 0, 1, 0, [
                { offset: 0, color: "#2563eb" },
                { offset: 1, color: "#6366f1" },
              ]),
              borderRadius: [0, 4, 4, 0],
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
          backgroundColor: "#1e293b",
          borderColor: "#334155",
          textStyle: { color: "#f8fafc" },
        },
        series: [
          {
            name: "Distribution",
            type: "pie",
            radius: ["40%", "70%"],
            center: ["50%", "50%"],
            avoidLabelOverlap: true,
            itemStyle: {
              borderRadius: 6,
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

<div class="p-6 h-full flex flex-col space-y-5 overflow-y-auto">
  <!-- Attribute Selector Header -->
  <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 p-4 rounded-xl bg-slate-900/70 border border-slate-800">
    <div class="relative flex items-center gap-3" bind:this={dropdownRef}>
      <span class="text-xs font-semibold uppercase tracking-wider text-slate-400 shrink-0">Select Attribute:</span>
      
      <!-- Custom Dropdown Trigger Button -->
      <div class="relative">
        <button
          type="button"
          onclick={toggleDropdown}
          class="flex items-center justify-between gap-3 bg-slate-950 hover:bg-slate-900 border border-slate-700/80 hover:border-blue-500/80 text-white rounded-xl px-3.5 py-2 text-xs font-mono transition-all duration-150 shadow-inner group min-w-[280px] focus:outline-none focus:ring-2 focus:ring-blue-500/30 cursor-pointer"
        >
          <div class="flex items-center gap-2 truncate">
            <span class="w-2 h-2 rounded-full bg-blue-400 group-hover:scale-125 transition-transform shrink-0"></span>
            <span class="font-bold text-slate-100 truncate">
              {feedState.selectedColumn || "Select attribute..."}
            </span>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            {#if currentColumnInfo()}
              <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-semibold border {currentColumnInfo()!.fill_rate >= 100 ? 'bg-emerald-950/80 text-emerald-400 border-emerald-800/60' : currentColumnInfo()!.fill_rate >= 60 ? 'bg-amber-950/80 text-amber-400 border-amber-800/60' : 'bg-rose-950/80 text-rose-400 border-rose-800/60'}">
                {currentColumnInfo()!.fill_rate.toFixed(0)}% fill
              </span>
            {/if}
            <ChevronDown class="w-4 h-4 text-slate-400 group-hover:text-blue-400 transition-transform duration-200 {isDropdownOpen ? 'rotate-180' : ''}" />
          </div>
        </button>

        <!-- Floating Dropdown Menu -->
        {#if isDropdownOpen}
          <div
            class="absolute top-full left-0 mt-2 w-[380px] max-h-[460px] bg-slate-900/98 border border-slate-750 rounded-2xl shadow-2xl backdrop-blur-2xl z-50 flex flex-col overflow-hidden animate-in fade-in slide-in-from-top-2 duration-150"
            style="box-shadow: 0 20px 50px -10px rgba(0,0,0,0.8), 0 0 0 1px rgba(255,255,255,0.06);"
          >
            <!-- Search Bar -->
            <div class="p-3 border-b border-slate-800/90 bg-slate-950/60 shrink-0">
              <div class="relative">
                <Search class="w-4 h-4 text-slate-400 absolute left-3 top-2.5 pointer-events-none" />
                <input
                  bind:this={searchInputRef}
                  bind:value={searchQuery}
                  type="text"
                  placeholder="Search attribute name..."
                  class="w-full bg-slate-900 border border-slate-800 rounded-xl pl-9 pr-8 py-2 text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-blue-500 font-mono transition"
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

              <!-- Quick Filter Tabs -->
              <div class="flex items-center gap-1.5 mt-2.5 pt-1">
                <button
                  type="button"
                  onclick={() => (filterCategory = "all")}
                  class="px-2.5 py-1 rounded-lg text-[10px] font-semibold border transition cursor-pointer {filterCategory === 'all' ? 'bg-blue-600/30 text-blue-300 border-blue-500/40' : 'bg-slate-800/60 text-slate-400 hover:text-slate-200 border-slate-750'}"
                >
                  All ({feedState.completeness.length})
                </button>
                <button
                  type="button"
                  onclick={() => (filterCategory = "complete")}
                  class="px-2.5 py-1 rounded-lg text-[10px] font-semibold border transition cursor-pointer {filterCategory === 'complete' ? 'bg-blue-600/30 text-blue-300 border-blue-500/40' : 'bg-slate-800/60 text-slate-400 hover:text-slate-200 border-slate-750'}"
                >
                  100% Fill ({completeCount})
                </button>
                <button
                  type="button"
                  onclick={() => (filterCategory = "partial")}
                  class="px-2.5 py-1 rounded-lg text-[10px] font-semibold border transition cursor-pointer {filterCategory === 'partial' ? 'bg-blue-600/30 text-blue-300 border-blue-500/40' : 'bg-slate-800/60 text-slate-400 hover:text-slate-200 border-slate-750'}"
                >
                  Partial &lt;100% ({partialCount})
                </button>
              </div>
            </div>

            <!-- Options Scroll List -->
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
                    class="w-full text-left flex items-center justify-between gap-3 px-3 py-2 rounded-xl text-xs font-mono transition-colors group cursor-pointer {isSelected ? 'bg-blue-600/20 text-blue-300 border border-blue-500/40 font-bold' : 'hover:bg-slate-800/70 text-slate-300 border border-transparent'}"
                  >
                    <div class="flex items-center gap-2 truncate">
                      <span class="w-1.5 h-1.5 rounded-full {isSelected ? 'bg-blue-400 scale-125' : 'bg-slate-600 group-hover:bg-slate-400'} shrink-0"></span>
                      <span class="truncate {isSelected ? 'text-white' : 'text-slate-200'}">{col.original_name}</span>
                    </div>
                    <div class="flex items-center gap-2 shrink-0">
                      <span class="text-[10px] text-slate-500 font-normal">{col.unique_count.toLocaleString()} unique</span>
                      <span class="px-1.5 py-0.5 rounded text-[10px] font-semibold border {col.fill_rate >= 100 ? 'bg-emerald-950/70 text-emerald-400 border-emerald-800/60' : col.fill_rate >= 60 ? 'bg-amber-950/70 text-amber-400 border-amber-800/60' : 'bg-rose-950/70 text-rose-400 border-rose-800/60'}">
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

    <!-- Quick Stats for chosen col -->
    {#if currentColumnInfo()}
      <div class="flex flex-wrap items-center gap-3 text-xs font-mono">
        <!-- Total Count Metric -->
        <div class="flex items-center gap-1.5 bg-slate-950/80 px-2.5 py-1 rounded border border-slate-800">
          <span class="text-slate-400">Total Count:</span>
          <span class="font-bold text-slate-200">
            {currentColumnInfo()!.valid_count.toLocaleString()}
            {#if currentColumnInfo()!.null_count > 0}
              <span class="text-slate-500 font-normal"> / {currentColumnInfo()!.total_rows.toLocaleString()}</span>
            {/if}
          </span>
        </div>

        <!-- Distinct Uniques Metric -->
        <div class="flex items-center gap-1.5 bg-slate-950/80 px-2.5 py-1 rounded border border-slate-800">
          <span class="text-slate-400">Distinct Uniques:</span>
          <span class="font-bold text-blue-400">
            {currentColumnInfo()!.unique_count.toLocaleString()}
          </span>
        </div>

        <!-- Fill Rate Metric -->
        <div class="flex items-center gap-1.5 bg-slate-950/80 px-2.5 py-1 rounded border border-slate-800">
          <span class="text-slate-400">Fill:</span>
          <span class="font-bold text-slate-200">
            {currentColumnInfo()!.fill_rate.toFixed(1)}%
          </span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Chart and Table Section -->
  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <!-- Value Distribution Chart -->
    <div class="p-5 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm flex flex-col">
      <div class="flex items-center justify-between mb-3">
        <div>
          <h3 class="text-sm font-bold text-white">Top Frequency Distribution</h3>
          <p class="text-xs text-slate-400">Most frequent values and percentage share</p>
        </div>
        <!-- Chart type toggles -->
        <div class="flex items-center gap-1 bg-slate-950 p-1 rounded-lg border border-slate-800">
          <button
            onclick={() => (chartType = "bar")}
            class="p-1 rounded {chartType === 'bar' ? 'bg-blue-950 text-blue-400 border border-blue-800/40' : 'text-slate-500 hover:text-slate-300'} transition cursor-pointer"
            title="Horizontal Bar Chart"
          >
            <BarChart2 class="w-3.5 h-3.5" />
          </button>
          <button
            onclick={() => (chartType = "pie")}
            class="p-1 rounded {chartType === 'pie' ? 'bg-blue-950 text-blue-400 border border-blue-800/40' : 'text-slate-500 hover:text-slate-300'} transition cursor-pointer"
            title="Donut Chart"
          >
            <PieChart class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      <div bind:this={chartContainer} class="w-full h-80 min-h-[300px]"></div>
    </div>

    <!-- Duplicate / Collision Finder -->
    <div class="p-5 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm flex flex-col">
      <div class="flex items-center justify-between mb-3">
        <div>
          <h3 class="text-sm font-bold text-white flex items-center gap-2">
            <span>Collision & Duplicate Tracker</span>
            {#if feedState.duplicateEntries.length > 0}
              <span class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-slate-800 text-slate-300 border border-slate-700">
                {feedState.duplicateEntries.length} Collisions
              </span>
            {:else}
              <span class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-950 text-blue-400 border border-blue-800/60">
                Unique
              </span>
            {/if}
          </h3>
          <p class="text-xs text-slate-400">Values that occur multiple times for this attribute</p>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto max-h-80 border border-slate-800/80 rounded-lg isolate">
        {#if feedState.duplicateEntries.length > 0}
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-950 sticky top-0 z-10 text-slate-400 font-semibold uppercase text-[10px] border-b border-slate-800">
              <tr class="bg-slate-950">
                <th class="py-2.5 px-3 bg-slate-950">Duplicate Value</th>
                <th class="py-2.5 px-3 text-right bg-slate-950">Occurrences</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-800 font-mono text-slate-300">
              {#each feedState.duplicateEntries as entry}
                <tr class="hover:bg-slate-800/40">
                  <td class="py-2 px-3 text-slate-200 truncate max-w-[200px]" title={entry.key}>
                    {entry.key}
                  </td>
                  <td class="py-2 px-3 text-right font-bold text-white">
                    {entry.count.toLocaleString()}x
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <div class="h-full flex flex-col items-center justify-center p-6 text-center text-slate-500">
            <CheckCircle class="w-8 h-8 text-blue-500 mb-2 opacity-80" />
            <p class="text-xs font-semibold text-slate-300">Zero Duplicates Detected</p>
            <p class="text-[11px] text-slate-500 mt-0.5">All populated values for this attribute are completely unique.</p>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Detailed Frequency Table -->
  <div class="p-5 rounded-xl bg-slate-900/70 border border-slate-800 flex flex-col">
    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 mb-4">
      <div>
        <h3 class="text-sm font-bold text-white mb-0.5">Top Frequency Breakdown</h3>
        <p class="text-xs text-slate-400">Complete ranking of the most frequent values for <span class="text-blue-400 font-mono">{feedState.selectedColumn}</span></p>
      </div>
      <button
        onclick={() => feedState.exportColumnFrequency(feedState.selectedColumn)}
        class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold bg-blue-600 hover:bg-blue-500 text-white transition shadow-sm cursor-pointer self-start sm:self-auto"
        title="Export this column's frequency distribution to CSV"
      >
        <Download class="w-3.5 h-3.5 text-white" />
        <span>Export Frequency to CSV</span>
      </button>
    </div>

    <div class="overflow-x-auto border border-slate-800 rounded-lg max-h-72 overflow-y-auto isolate">
      <table class="w-full text-left text-xs border-collapse">
        <thead class="bg-slate-950 sticky top-0 z-10 text-slate-400 font-semibold uppercase text-[10px] border-b border-slate-800">
          <tr class="bg-slate-950">
            <th class="py-2.5 px-3 w-12 text-center text-slate-600 bg-slate-950">Rank</th>
            <th class="py-2.5 px-3 bg-slate-950">Value</th>
            <th class="py-2.5 px-3 text-right bg-slate-950">Count</th>
            <th class="py-2.5 px-3 text-right bg-slate-950">Distribution Share</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800 font-mono text-slate-300">
          {#each feedState.columnDistribution as dist, i}
            <tr class="hover:bg-slate-800/40">
              <td class="py-2 px-3 text-center text-slate-500 text-[10px]">{i + 1}</td>
              <td class="py-2 px-3 font-semibold text-slate-200 truncate max-w-sm" title={dist.value}>
                {dist.value}
              </td>
              <td class="py-2 px-3 text-right text-blue-300">{dist.count.toLocaleString()}</td>
              <td class="py-2 px-3 text-right">
                <div class="flex items-center justify-end gap-2">
                  <div class="w-16 bg-slate-800 rounded-full h-1.5 overflow-hidden">
                    <div class="bg-blue-500 h-full rounded-full" style="width: {dist.percentage}%"></div>
                  </div>
                  <span class="text-slate-400 text-xs w-10 text-right">{dist.percentage.toFixed(1)}%</span>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>
