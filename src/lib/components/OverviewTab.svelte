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

    const resizeObserver = new ResizeObserver(() => {
      chartInstance?.resize();
    });
    if (chartContainer) {
      resizeObserver.observe(chartContainer);
    }

    return () => {
      window.removeEventListener("resize", handleResize);
      resizeObserver.disconnect();
      chartInstance?.dispose();
    };
  });
</script>

<div class="p-6 space-y-6 overflow-y-auto h-full w-full">
  <!-- 1. FOUR TOP KPI CARDS (Matches Hybrid Mockup Exactly) -->
  <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
    <!-- KPI 1 -->
    <div class="data-card rounded-2xl p-5 relative overflow-hidden">
      <div class="flex items-center justify-between mb-2">
        <span class="micro-title">TOTAL RECORD VOLUME</span>
        <span class="tabular-tag text-[10px] font-bold px-2 py-0.5 rounded bg-blue-500/15 text-blue-400">#01</span>
      </div>
      <div class="kpi-number text-3xl text-white">
        {totalRecords.toLocaleString()}
      </div>
      <div class="mt-2.5 flex items-center gap-2 text-xs">
        <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
        <span class="text-emerald-400 font-medium">100% Valid XML Structure</span>
      </div>
    </div>

    <!-- KPI 2 -->
    <div class="data-card rounded-2xl p-5 relative overflow-hidden">
      <div class="flex items-center justify-between mb-2">
        <span class="micro-title">OVERALL HEALTH SCORE</span>
        <span class="tabular-tag text-[10px] font-bold px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-400">⚡</span>
      </div>
      <div class="kpi-number text-3xl text-emerald-400 flex items-baseline">
        <span>{averageFillRate().toFixed(0)}</span><span class="kpi-decimal">.{Math.floor((averageFillRate() % 1) * 10)}%</span>
      </div>
      <div class="mt-2.5 flex items-center gap-2 text-xs">
        <span class="tabular-tag text-[10px] font-bold px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/30">
          {averageFillRate() >= 80 ? 'GRADE A' : averageFillRate() >= 60 ? 'GRADE B' : 'GRADE C'}
        </span>
        <span class="text-slate-400">Production Standard</span>
      </div>
    </div>

    <!-- KPI 3 -->
    <div class="data-card rounded-2xl p-5 relative overflow-hidden">
      <div class="flex items-center justify-between mb-2">
        <span class="micro-title">ATTRIBUTES TRACKED</span>
        <span class="tabular-tag text-[10px] font-bold px-2 py-0.5 rounded bg-purple-500/15 text-purple-400">COLUMNS</span>
      </div>
      <div class="kpi-number text-3xl text-white">
        {totalAttributes}
      </div>
      <div class="mt-2.5 flex items-center gap-2 text-xs">
        <span class="text-cyan-400 tabular-tag font-bold">{completeCols.length} Complete</span>
        <span class="text-slate-600">·</span>
        <span class="text-amber-400 tabular-tag font-bold">{warningCols.length} Partial</span>
      </div>
    </div>

    <!-- KPI 4 -->
    <div class="data-card rounded-2xl p-5 relative overflow-hidden">
      <div class="flex items-center justify-between mb-2">
        <span class="micro-title">INGESTION QUALITY</span>
        <span class="tabular-tag text-[10px] font-bold px-2 py-0.5 rounded bg-cyan-500/15 text-cyan-400">DUCKDB</span>
      </div>
      <div class="kpi-number text-3xl text-cyan-400 flex items-baseline">
        <span>{completeCols.length}</span><span class="kpi-decimal"> / {totalAttributes}</span>
      </div>
      <div class="mt-2.5 flex items-center gap-2 text-xs text-slate-400">
        <span class="tabular-tag">{criticalCols.length} critical empty fields</span>
      </div>
    </div>
  </div>

  <!-- 2. MAIN SPLIT: LEFT CHART (5 COLS) + RIGHT ATTRIBUTES (7 COLS) -->
  <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
    
    <!-- LEFT SECTION (5 Cols): Donut Chart & Completeness Breakdown -->
    <div class="lg:col-span-5 space-y-4">
      <div class="data-card rounded-2xl p-6 flex flex-col items-center justify-between relative overflow-hidden">
        <div class="w-full flex items-center justify-between mb-2">
          <div>
            <h3 class="text-sm font-bold text-white tracking-tight">Completeness Breakdown</h3>
            <p class="text-xs text-slate-400 mt-0.5">Attribute fill rates across {feedState.completeness.length} columns</p>
          </div>
          <span class="tabular-tag text-xs font-bold px-2.5 py-1 rounded-lg data-card-subtle text-slate-200">
            {averageFillRate().toFixed(1)}% SCORE
          </span>
        </div>

        <!-- ECharts Donut Chart Container -->
        <div class="w-full h-64 flex items-center justify-center my-2">
          <div bind:this={chartContainer} class="w-full h-full"></div>
        </div>

        <!-- Interactive Legend Cards -->
        <div class="w-full grid grid-cols-3 gap-2.5 pt-4 border-t border-slate-800 text-center">
          <button
            onclick={() => setFilter("complete")}
            class="data-card-subtle p-2.5 rounded-xl hover:border-blue-500/60 transition cursor-pointer text-center group {currentFilter === 'complete' ? 'ring-1 ring-blue-500' : ''}"
          >
            <div class="micro-title">COMPLETE (100%)</div>
            <div class="kpi-number text-sm text-cyan-400 mt-1">{completeCols.length} fields</div>
          </button>
          <button
            onclick={() => setFilter("warning")}
            class="data-card-subtle p-2.5 rounded-xl hover:border-amber-500/60 transition cursor-pointer text-center group {currentFilter === 'warning' ? 'ring-1 ring-amber-500' : ''}"
          >
            <div class="micro-title">PARTIAL (&gt;0%)</div>
            <div class="kpi-number text-sm text-amber-400 mt-1">{warningCols.length} fields</div>
          </button>
          <button
            onclick={() => setFilter("critical")}
            class="data-card-subtle p-2.5 rounded-xl hover:border-rose-500/60 transition cursor-pointer text-center group {currentFilter === 'critical' ? 'ring-1 ring-rose-500' : ''}"
          >
            <div class="micro-title">EMPTY (0%)</div>
            <div class="kpi-number text-sm text-rose-400 mt-1">{criticalCols.length} fields</div>
          </button>
        </div>
      </div>

      <!-- Priority Attention Box -->
      {#if priorityLowestCols().length > 0}
        <div class="data-card rounded-2xl p-4 space-y-2.5">
          <div class="flex items-center justify-between">
            <h4 class="text-xs font-bold text-slate-200 uppercase tracking-wider flex items-center gap-1.5">
              <ShieldAlert class="w-4 h-4 text-rose-400" />
              Priority Attention Fields
            </h4>
            <span class="text-[11px] text-slate-400">Lowest coverage</span>
          </div>

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
        </div>
      {/if}
    </div>

    <!-- RIGHT SECTION (7 Cols): Core Recruitment Attributes & List -->
    <div class="lg:col-span-7 data-card rounded-2xl p-6 flex flex-col justify-between space-y-4">
      <div class="flex items-center justify-between pb-3 border-b border-slate-800">
        <div>
          <h3 class="text-sm font-bold text-white tracking-tight">Critical Recruitment Attributes</h3>
          <p class="text-xs text-slate-400 mt-0.5">Core fields required by major job boards (Indeed, Google for Jobs)</p>
        </div>
        <button
          onclick={() => (feedState.activeTab = "completeness")}
          class="text-xs text-cyan-400 font-bold hover:underline flex items-center gap-1 cursor-pointer"
        >
          <span>View Full Matrix</span>
          <span>&rarr;</span>
        </button>
      </div>

      <!-- Controls: Filter Pills + Sort -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <!-- Filter Pills -->
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

        <!-- Sort Button -->
        <button 
          onclick={toggleSort} 
          class="px-3 py-1.5 rounded-xl bg-slate-800/90 hover:bg-slate-800 border border-slate-700 text-xs font-semibold text-slate-200 hover:text-white flex items-center gap-2 transition cursor-pointer shadow-sm shrink-0"
        >
          <ArrowUpDown class="w-3.5 h-3.5 text-blue-400" />
          <span>{sortMode === "lowest_first" ? "Lowest First" : "Highest First"}</span>
        </button>
      </div>

      <!-- Search Bar -->
      <div class="relative">
        <Search class="w-3.5 h-3.5 text-slate-500 absolute left-3.5 top-3 pointer-events-none" />
        <input 
          type="text" 
          bind:value={searchQuery}
          placeholder="Filter fields by name (e.g. title, salary, location)..."
          class="w-full bg-slate-950/80 border border-slate-800 rounded-xl pl-9 pr-24 py-2 text-xs font-mono text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500/40 transition"
        />
        <div class="absolute right-3 top-2.5 text-[11px] text-slate-500 font-mono">
          {filteredCols().length} fields
        </div>
      </div>

      <!-- Scrollable Attribute Rows -->
      <div class="space-y-2.5 max-h-[500px] overflow-y-auto pr-1">
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
              class="w-full data-card-subtle p-3.5 rounded-xl flex items-center justify-between hover:border-slate-600 transition cursor-pointer group text-left"
              title="Click to inspect '{col.original_name}' in Deep Dive"
            >
              <div class="flex items-center gap-3 min-w-0 pr-2">
                <div class="w-2.5 h-2.5 rounded-full {badge.barColor} shrink-0"></div>
                <div class="min-w-0">
                  <div class="tabular-tag text-xs font-bold text-white group-hover:text-blue-300 truncate" title={col.original_name}>
                    {col.original_name}
                  </div>
                  <div class="text-[11px] text-slate-400">
                    {col.valid_count.toLocaleString()} rows populated
                  </div>
                </div>
              </div>
              <div class="flex items-center gap-4 shrink-0">
                <div class="w-28 bg-slate-800 h-2 rounded-full overflow-hidden hidden sm:block">
                  <div class="h-full {badge.barColor} rounded-full" style="width: {col.fill_rate}%"></div>
                </div>
                <span class="kpi-number text-xs font-bold {badge.textColor} w-14 text-right">
                  {col.fill_rate.toFixed(1)}%
                </span>
              </div>
            </button>
          {/each}
        {/if}
      </div>
    </div>

  </div>
</div>
