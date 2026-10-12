<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { onMount } from "svelte";
  import * as echarts from "echarts";
  import { 
    Layers, 
    CheckCircle2, 
    AlertTriangle, 
    Database, 
    ArrowUpDown, 
    Search, 
    ArrowRight, 
    ShieldAlert 
  } from "@lucide/svelte";

  let chartContainer: HTMLDivElement;
  let chartInstance: echarts.ECharts | null = null;

  let currentFilter = $state<"all" | "complete" | "warning" | "critical">("all");
  let sortMode = $state<"lowest_first" | "highest_first">("lowest_first");
  let searchQuery = $state("");

  let totalRecords = $derived(feedState.stats?.total_records ?? 0);
  let totalAttributes = $derived(feedState.completeness.length);

  let averageFillRate = $derived(() => {
    if (feedState.completeness.length === 0) return 0;
    const total = feedState.completeness.reduce((acc, c) => acc + c.fill_rate, 0);
    return total / feedState.completeness.length;
  });

  let completeCols = $derived(feedState.completeness.filter((c) => c.fill_rate >= 99.9));
  let warningCols = $derived(feedState.completeness.filter((c) => c.fill_rate >= 10.0 && c.fill_rate < 99.9));
  let criticalCols = $derived(feedState.completeness.filter((c) => c.fill_rate < 10.0));
  let incompleteCols = $derived(feedState.completeness.filter((c) => c.fill_rate < 99.9));

  // Priority attention list: lowest 3 fields with missing rows
  let priorityLowestCols = $derived(() => {
    const sorted = [...feedState.completeness].sort((a, b) => a.fill_rate - b.fill_rate);
    return sorted.filter((c) => c.fill_rate < 99.9).slice(0, 3);
  });

  // Filtered and sorted columns for the left cards
  let filteredCols = $derived(() => {
    const query = searchQuery.trim().toLowerCase();
    let list = feedState.completeness.filter((c) => {
      const matchesSearch = 
        !query || 
        c.original_name.toLowerCase().includes(query) || 
        c.column_name.toLowerCase().includes(query);
      
      if (!matchesSearch) return false;
      if (currentFilter === "complete") return c.fill_rate >= 99.9;
      if (currentFilter === "warning") return c.fill_rate >= 10.0 && c.fill_rate < 99.9;
      if (currentFilter === "critical") return c.fill_rate < 10.0;
      return true;
    });

    if (sortMode === "lowest_first") {
      list.sort((a, b) => a.fill_rate - b.fill_rate);
    } else {
      list.sort((a, b) => b.fill_rate - a.fill_rate);
    }
    return list;
  });

  function getHealthBadge(fillRate: number) {
    if (fillRate < 10) {
      return {
        textColor: "text-rose-400",
        barColor: "bg-rose-500",
      };
    } else if (fillRate < 50) {
      return {
        textColor: "text-amber-400",
        barColor: "bg-amber-500",
      };
    } else if (fillRate < 90) {
      return {
        textColor: "text-indigo-400",
        barColor: "bg-indigo-500",
      };
    } else {
      return {
        textColor: "text-emerald-400",
        barColor: "bg-emerald-500",
      };
    }
  }

  function toggleSort() {
    sortMode = sortMode === "lowest_first" ? "highest_first" : "lowest_first";
  }

  function setFilter(tier: "all" | "complete" | "warning" | "critical") {
    currentFilter = tier;
  }

  function updateChart() {
    if (!chartContainer || feedState.completeness.length === 0) return;

    if (!chartInstance) {
      chartInstance = echarts.init(chartContainer);
      chartInstance.on("click", (params: any) => {
        if (params.name.includes("Complete")) {
          setFilter("complete");
        } else if (params.name.includes("Warning")) {
          setFilter("warning");
        } else if (params.name.includes("Critical")) {
          setFilter("critical");
        }
      });
    }

    const completeCount = completeCols.length;
    const warningCount = warningCols.length;
    const criticalCount = criticalCols.length;
    const avg = averageFillRate();

    const option: echarts.EChartsOption = {
      backgroundColor: "transparent",
      tooltip: {
        trigger: "item",
        backgroundColor: "#0f172a",
        borderColor: "#334155",
        borderWidth: 1,
        textStyle: { color: "#f8fafc", fontSize: 12 },
        formatter: (params: any) => {
          return `<div class="font-sans px-1 py-0.5">
            <div class="font-bold text-white flex items-center gap-1.5">
              <span style="display:inline-block;width:8px;height:8px;border-radius:50%;background-color:${params.color}"></span>
              ${params.name}
            </div>
            <div class="text-xs text-slate-300 mt-1">${params.value} attributes (${params.percent}%)</div>
            <div class="text-[10px] text-blue-400 mt-1 font-mono">Click to filter left list</div>
          </div>`;
        },
      },
      title: {
        text: `${avg.toFixed(1)}%`,
        subtext: "Average Health",
        left: "center",
        top: "37%",
        textStyle: {
          color: "#ffffff",
          fontSize: 24,
          fontWeight: "bold",
          fontFamily: "monospace",
        },
        subtextStyle: {
          color: "#94a3b8",
          fontSize: 11,
          fontWeight: 500,
        },
      },
      series: [
        {
          name: "Feed Health",
          type: "pie",
          radius: ["60%", "84%"],
          center: ["50%", "50%"],
          avoidLabelOverlap: false,
          itemStyle: {
            borderRadius: 6,
            borderColor: "#090d16",
            borderWidth: 3,
          },
          label: { show: false },
          emphasis: {
            scale: true,
            scaleSize: 6,
            itemStyle: {
              shadowBlur: 14,
              shadowOffsetX: 0,
              shadowColor: "rgba(0, 0, 0, 0.6)",
            },
          },
          data: [
            {
              value: completeCount,
              name: "100% Complete",
              itemStyle: { color: "#3b82f6" },
            },
            {
              value: warningCount,
              name: "Warning (10-99%)",
              itemStyle: { color: "#f59e0b" },
            },
            {
              value: criticalCount,
              name: "Critical (<10%)",
              itemStyle: { color: "#ef4444" },
            },
          ].filter((d) => d.value > 0),
        },
      ],
    };

    chartInstance.setOption(option, true);
  }

  $effect(() => {
    if (feedState.completeness.length > 0) {
      updateChart();
    }
  });

  onMount(() => {
    const handleResize = () => chartInstance?.resize();
    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
      chartInstance?.dispose();
    };
  });
</script>

<div class="p-6 space-y-5 overflow-y-auto h-full">
  <!-- 1. SLEEK SINGLE-LINE HEADER BAR (Throughput removed as requested) -->
  <div class="px-5 py-3 rounded-xl bg-slate-900/80 border border-slate-800 shadow-sm flex flex-wrap items-center justify-between gap-4">
    <!-- Metric 1: Total Records -->
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-blue-950/60 border border-blue-800/50 flex items-center justify-center text-blue-400">
        <Database class="w-4 h-4" />
      </div>
      <div>
        <div class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Total Records</div>
        <div class="text-lg font-black text-white font-mono leading-tight">
          {totalRecords.toLocaleString()}
        </div>
      </div>
    </div>

    <div class="hidden sm:block h-6 w-px bg-slate-800"></div>

    <!-- Metric 2: Attributes Count -->
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-cyan-950/60 border border-cyan-800/50 flex items-center justify-center text-cyan-400">
        <Layers class="w-4 h-4" />
      </div>
      <div>
        <div class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Attributes</div>
        <div class="text-lg font-black text-white font-mono leading-tight">
          {totalAttributes}
        </div>
      </div>
    </div>

    <div class="hidden sm:block h-6 w-px bg-slate-800"></div>

    <!-- Metric 3: Average Fill Rate -->
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-blue-950/60 border border-blue-800/50 flex items-center justify-center text-blue-400">
        <CheckCircle2 class="w-4 h-4" />
      </div>
      <div>
        <div class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Avg Fill Rate</div>
        <div class="text-lg font-black text-blue-400 font-mono leading-tight">
          {averageFillRate().toFixed(1)}%
        </div>
      </div>
    </div>

    <div class="hidden sm:block h-6 w-px bg-slate-800"></div>

    <!-- Metric 4: Incomplete Fields -->
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-amber-950/60 border border-amber-800/50 flex items-center justify-center text-amber-400">
        <AlertTriangle class="w-4 h-4" />
      </div>
      <div>
        <div class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Incomplete Fields</div>
        <div class="text-lg font-black text-amber-400 font-mono leading-tight">
          {incompleteCols.length}
        </div>
      </div>
    </div>
  </div>

  <!-- 2. MAIN SPLIT: LEFT FIELD DETAILS + RIGHT PIE CHART -->
  <div class="grid grid-cols-1 lg:grid-cols-12 gap-5 items-start">
    
    <!-- LEFT SECTION (7 Cols / ~58% width): Field Details & Cards -->
    <div class="lg:col-span-7 bg-slate-900/70 border border-slate-800 rounded-2xl p-5 shadow-lg flex flex-col space-y-3.5">
      
      <!-- Controls Header: Category Sleek Bar + Sort Button -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-slate-800">
        
        <!-- Category Sleek Filter Pills -->
        <div class="flex items-center gap-1.5 flex-wrap">
          <button 
            onclick={() => setFilter("all")} 
            class="px-3 py-1.5 rounded-full text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer {currentFilter === 'all' ? 'bg-blue-600 text-white shadow-sm' : 'bg-slate-800/80 text-slate-300 hover:bg-slate-800 hover:text-white'}"
          >
            <span>All</span>
            <span class="text-[11px] px-1.5 py-0.2 rounded-full {currentFilter === 'all' ? 'bg-blue-900/60 text-blue-200' : 'bg-slate-700 text-slate-300'}">
              {feedState.completeness.length}
            </span>
          </button>

          <button 
            onclick={() => setFilter("complete")} 
            class="px-3 py-1.5 rounded-full text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer {currentFilter === 'complete' ? 'bg-blue-600 text-white shadow-sm' : 'bg-slate-800/80 text-slate-300 hover:bg-slate-800 hover:text-white'}"
          >
            <span>100% Complete</span>
            <span class="text-[11px] px-1.5 py-0.2 rounded-full {currentFilter === 'complete' ? 'bg-blue-900/60 text-blue-200' : 'bg-slate-700 text-slate-300'}">
              {completeCols.length}
            </span>
          </button>

          <button 
            onclick={() => setFilter("warning")} 
            class="px-3 py-1.5 rounded-full text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer {currentFilter === 'warning' ? 'bg-amber-600 text-white shadow-sm' : 'bg-slate-800/80 text-amber-300 hover:bg-amber-950/40 hover:border-amber-800/50'}"
          >
            <span>Warning</span>
            <span class="text-[11px] px-1.5 py-0.2 rounded-full {currentFilter === 'warning' ? 'bg-amber-900/60 text-amber-200' : 'bg-amber-500/20 text-amber-300'}">
              {warningCols.length}
            </span>
          </button>

          <button 
            onclick={() => setFilter("critical")} 
            class="px-3 py-1.5 rounded-full text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer {currentFilter === 'critical' ? 'bg-rose-600 text-white shadow-sm' : 'bg-slate-800/80 text-rose-300 hover:bg-rose-950/40 hover:border-rose-800/50'}"
          >
            <span>Critical</span>
            <span class="text-[11px] px-1.5 py-0.2 rounded-full {currentFilter === 'critical' ? 'bg-rose-900/60 text-rose-200' : 'bg-rose-500/20 text-rose-300'}">
              {criticalCols.length}
            </span>
          </button>
        </div>

        <!-- Sort Button (Lowest Fill First / Highest Fill First) -->
        <div class="flex items-center gap-2 shrink-0">
          <button 
            onclick={toggleSort} 
            class="px-3 py-1.5 rounded-lg bg-slate-800/90 hover:bg-slate-800 border border-slate-700 text-xs font-semibold text-slate-200 hover:text-white flex items-center gap-2 transition cursor-pointer shadow-sm"
          >
            <ArrowUpDown class="w-3.5 h-3.5 text-blue-400" />
            <span>{sortMode === "lowest_first" ? "Lowest Fill First" : "Highest Fill First"}</span>
          </button>
        </div>
      </div>

      <!-- Search Bar with Live Field Counter -->
      <div class="relative">
        <Search class="w-3.5 h-3.5 text-slate-500 absolute left-3.5 top-3 pointer-events-none" />
        <input 
          type="text" 
          bind:value={searchQuery}
          placeholder="Filter fields by name (e.g. payout, date, id, title)..."
          class="w-full bg-slate-950/80 border border-slate-800 rounded-xl pl-9 pr-24 py-2 text-xs font-mono text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500/40 transition"
        />
        <div class="absolute right-3 top-2.5 text-[11px] text-slate-500 font-mono">
          {filteredCols().length} fields
        </div>
      </div>

      <!-- Scrollable Cards Deck (Matching screenshot style) -->
      <div class="space-y-2.5 max-h-[560px] overflow-y-auto pr-1">
        {#if filteredCols().length === 0}
          <div class="p-8 text-center text-xs text-slate-500 italic">
            No attributes match the selected filter or search query.
          </div>
        {:else}
          {#each filteredCols() as col}
            {@const badge = getHealthBadge(col.fill_rate)}
            <button
              onclick={() => {
                feedState.fetchDeepDive(col.column_name);
                feedState.activeTab = "deepdive";
              }}
              class="w-full p-3.5 rounded-xl bg-slate-900/80 border border-slate-800 hover:border-slate-700 hover:bg-slate-850 transition cursor-pointer group shadow-sm block text-left"
              title="Click to inspect '{col.original_name}' in Deep Dive"
            >
              <div class="flex items-center justify-between gap-3 mb-2">
                <span class="text-xs font-mono text-slate-300 group-hover:text-blue-300 truncate" title={col.original_name}>
                  {col.original_name}
                </span>
                <span class="text-xs font-mono font-bold {badge.textColor} shrink-0">
                  {col.fill_rate.toFixed(1)}%
                </span>
              </div>
              <!-- Slim Progress Bar -->
              <div class="w-full bg-slate-800/80 h-1.5 rounded-full overflow-hidden">
                <div 
                  class="h-full {badge.barColor} rounded-full transition-all duration-300" 
                  style="width: {Math.max(2, col.fill_rate)}%"
                ></div>
              </div>
            </button>
          {/each}
        {/if}
      </div>

    </div>

    <!-- RIGHT SECTION (5 Cols / ~42% width): Aesthetic Interactive Pie Chart -->
    <div class="lg:col-span-5 space-y-4">
      
      <!-- Main Donut / Pie Chart Card -->
      <div class="bg-slate-900/70 border border-slate-800 rounded-2xl p-5 shadow-lg flex flex-col items-center relative overflow-hidden">
        <div class="w-full flex items-center justify-between mb-1">
          <div>
            <h3 class="text-sm font-bold text-white flex items-center gap-2">
              <span class="w-2 h-2 rounded-full bg-blue-500"></span>
              Feed Health Distribution
            </h3>
            <p class="text-xs text-slate-400">Quality breakdown across {feedState.completeness.length} discovered attributes</p>
          </div>
          <span class="text-[11px] font-mono text-slate-400 bg-slate-800/80 px-2 py-0.5 rounded border border-slate-700/60">DuckDB Core</span>
        </div>

        <!-- ECharts Donut Chart Container -->
        <div class="w-full h-64 flex items-center justify-center my-1">
          <div bind:this={chartContainer} class="w-full h-full"></div>
        </div>

        <!-- Interactive Legend Buttons -->
        <div class="grid grid-cols-3 gap-2 w-full mt-1">
          <!-- Complete -->
          <button 
            onclick={() => setFilter("complete")} 
            class="p-2.5 rounded-xl bg-slate-950/60 border border-slate-800 hover:border-blue-500/60 flex flex-col items-center transition cursor-pointer group"
          >
            <div class="flex items-center gap-1.5 mb-1">
              <span class="w-2.5 h-2.5 rounded-full bg-blue-500 shadow-sm shadow-blue-500/50"></span>
              <span class="text-xs font-bold text-slate-200 group-hover:text-white">Complete</span>
            </div>
            <span class="text-sm font-black text-blue-400 font-mono">
              {completeCols.length} 
              <span class="text-[10px] font-normal text-slate-500">
                ({feedState.completeness.length > 0 ? ((completeCols.length / feedState.completeness.length) * 100).toFixed(0) : 0}%)
              </span>
            </span>
          </button>

          <!-- Warning -->
          <button 
            onclick={() => setFilter("warning")} 
            class="p-2.5 rounded-xl bg-slate-950/60 border border-slate-800 hover:border-amber-500/60 flex flex-col items-center transition cursor-pointer group"
          >
            <div class="flex items-center gap-1.5 mb-1">
              <span class="w-2.5 h-2.5 rounded-full bg-amber-500 shadow-sm shadow-amber-500/50"></span>
              <span class="text-xs font-bold text-slate-200 group-hover:text-white">Warning</span>
            </div>
            <span class="text-sm font-black text-amber-400 font-mono">
              {warningCols.length} 
              <span class="text-[10px] font-normal text-slate-500">
                ({feedState.completeness.length > 0 ? ((warningCols.length / feedState.completeness.length) * 100).toFixed(0) : 0}%)
              </span>
            </span>
          </button>

          <!-- Critical -->
          <button 
            onclick={() => setFilter("critical")} 
            class="p-2.5 rounded-xl bg-slate-950/60 border border-slate-800 hover:border-rose-500/60 flex flex-col items-center transition cursor-pointer group"
          >
            <div class="flex items-center gap-1.5 mb-1">
              <span class="w-2.5 h-2.5 rounded-full bg-rose-500 shadow-sm shadow-rose-500/50"></span>
              <span class="text-xs font-bold text-slate-200 group-hover:text-white">Critical</span>
            </div>
            <span class="text-sm font-black text-rose-400 font-mono">
              {criticalCols.length} 
              <span class="text-[10px] font-normal text-slate-500">
                ({feedState.completeness.length > 0 ? ((criticalCols.length / feedState.completeness.length) * 100).toFixed(0) : 0}%)
              </span>
            </span>
          </button>
        </div>
      </div>

      <!-- Priority Attention Box (Lowest coverage fields) -->
      <div class="bg-slate-900/70 border border-slate-800 rounded-2xl p-4 shadow-lg space-y-2.5">
        <div class="flex items-center justify-between">
          <h4 class="text-xs font-bold text-slate-200 uppercase tracking-wider flex items-center gap-1.5">
            <ShieldAlert class="w-4 h-4 text-rose-400" />
            Priority Attention Fields
          </h4>
          <span class="text-[11px] text-slate-400">Lowest coverage</span>
        </div>

        {#if priorityLowestCols().length === 0}
          <div class="p-3 text-xs text-slate-500 italic text-center">
            All fields have 100% data coverage!
          </div>
        {:else}
          <div class="space-y-2">
            {#each priorityLowestCols() as col}
              {@const missingCount = (col.total_records || totalRecords) - col.valid_count}
              <button
                onclick={() => {
                  feedState.fetchDeepDive(col.column_name);
                  feedState.activeTab = "deepdive";
                }}
                class="w-full p-2.5 rounded-xl {col.fill_rate < 10 ? 'bg-rose-950/20 border-rose-900/30' : 'bg-amber-950/20 border-amber-900/30'} border flex items-center justify-between hover:bg-slate-800 transition cursor-pointer text-left"
                title="Inspect '{col.original_name}' in Deep Dive"
              >
                <div class="min-w-0 pr-2">
                  <div class="text-xs font-mono {col.fill_rate < 10 ? 'text-rose-200' : 'text-amber-200'} truncate" title={col.original_name}>
                    {col.original_name}
                  </div>
                  <div class="text-[11px] {col.fill_rate < 10 ? 'text-rose-400/80' : 'text-amber-400/80'}">
                    {missingCount > 0 ? missingCount.toLocaleString() : 'Some'} missing values ({((100 - col.fill_rate)).toFixed(1)}% empty)
                  </div>
                </div>
                <div class="flex items-center gap-1.5 shrink-0">
                  <span class="text-xs font-bold font-mono {col.fill_rate < 10 ? 'text-rose-400' : 'text-amber-400'}">
                    {col.fill_rate.toFixed(1)}%
                  </span>
                  <ArrowRight class="w-3.5 h-3.5 text-slate-400" />
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </div>

    </div>

  </div>
</div>
