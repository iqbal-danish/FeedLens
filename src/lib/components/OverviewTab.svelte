<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { onMount } from "svelte";
  import * as echarts from "echarts";
  import { Layers, CheckCircle2, AlertTriangle, Zap, Database, ArrowRight } from "@lucide/svelte";

  let chartContainer: HTMLDivElement;
  let chartInstance: echarts.ECharts | null = null;

  let averageFillRate = $derived(() => {
    if (feedState.completeness.length === 0) return 0;
    const total = feedState.completeness.reduce((acc, c) => acc + c.fill_rate, 0);
    return total / feedState.completeness.length;
  });

  let fullyPopulatedCols = $derived(() => {
    return feedState.completeness.filter((c) => c.fill_rate >= 99.9);
  });

  let lowFillCols = $derived(() => {
    return feedState.completeness.filter((c) => c.fill_rate < 90.0);
  });

  function updateChart() {
    if (!chartContainer || feedState.completeness.length === 0) return;

    if (!chartInstance) {
      chartInstance = echarts.init(chartContainer);
    }

    // Sort ascending for bottom-to-top horizontal bar chart
    const data = [...feedState.completeness].reverse();
    const categories = data.map((d) => d.original_name);
    const fillRates = data.map((d) => parseFloat(d.fill_rate.toFixed(1)));

    const option: echarts.EChartsOption = {
      backgroundColor: "transparent",
      tooltip: {
        trigger: "axis",
        axisPointer: { type: "shadow" },
        backgroundColor: "#1e293b",
        borderColor: "#334155",
        textStyle: { color: "#f8fafc" },
        formatter: (params: any) => {
          const item = params[0];
          return `<div class="font-sans text-xs">
            <span class="font-bold text-blue-400">${item.name}</span>: ${item.value}% fill rate
          </div>`;
        },
      },
      grid: {
        left: "3%",
        right: "6%",
        bottom: "3%",
        top: "4%",
        containLabel: true,
      },
      xAxis: {
        type: "value",
        max: 100,
        axisLabel: { color: "#94a3b8", formatter: "{value}%" },
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
          name: "Fill Rate",
          type: "bar",
          data: fillRates.map((val) => ({
            value: val,
            itemStyle: {
              color: val >= 95 ? "#3b82f6" : val >= 80 ? "#6366f1" : "#64748b",
              borderRadius: [0, 4, 4, 0],
            },
          })),
          label: {
            show: true,
            position: "right",
            color: "#94a3b8",
            fontSize: 10,
            formatter: "{c}%",
          },
        },
      ],
    };

    chartInstance.setOption(option);
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

<div class="p-6 space-y-6 overflow-y-auto h-full">
  <!-- Top 4 KPI Cards -->
  <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
    <!-- Card 1: Total Records -->
    <div class="p-4 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm relative overflow-hidden">
      <div class="flex items-center justify-between text-slate-400 text-xs font-semibold uppercase tracking-wider mb-2">
        <span>Total Records</span>
        <Database class="w-4 h-4 text-blue-400" />
      </div>
      <div class="text-2xl font-black text-white font-mono">
        {feedState.stats?.total_records.toLocaleString() || "0"}
      </div>
      <p class="text-[11px] text-slate-400 mt-1">Indexed in DuckDB Columnar Store</p>
    </div>

    <!-- Card 2: Attributes Count -->
    <div class="p-4 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm relative overflow-hidden">
      <div class="flex items-center justify-between text-slate-400 text-xs font-semibold uppercase tracking-wider mb-2">
        <span>Attributes / Columns</span>
        <Layers class="w-4 h-4 text-blue-400" />
      </div>
      <div class="text-2xl font-black text-white font-mono">
        {feedState.stats?.column_count || feedState.completeness.length}
      </div>
      <p class="text-[11px] text-slate-400 mt-1">Auto-detected & flattened schema</p>
    </div>

    <!-- Card 3: Average Fill Rate -->
    <div class="p-4 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm relative overflow-hidden">
      <div class="flex items-center justify-between text-slate-400 text-xs font-semibold uppercase tracking-wider mb-2">
        <span>Average Fill Rate</span>
        <CheckCircle2 class="w-4 h-4 text-blue-400" />
      </div>
      <div class="text-2xl font-black text-blue-400 font-mono">
        {averageFillRate().toFixed(1)}%
      </div>
      <p class="text-[11px] text-slate-400 mt-1">Completeness across all fields</p>
    </div>

    <!-- Card 4: Indexing Speed -->
    <div class="p-4 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm relative overflow-hidden">
      <div class="flex items-center justify-between text-slate-400 text-xs font-semibold uppercase tracking-wider mb-2">
        <span>Throughput</span>
        <Zap class="w-4 h-4 text-slate-400" />
      </div>
      <div class="text-2xl font-black text-slate-200 font-mono">
        {Math.round(feedState.stats?.records_per_sec || 0).toLocaleString()} <span class="text-xs font-normal text-slate-400">rec/s</span>
      </div>
      <p class="text-[11px] text-slate-400 mt-1">
        Elapsed: {(feedState.stats?.elapsed_secs || 0).toFixed(2)}s
      </p>
    </div>
  </div>

  <!-- Main Chart & Insights Split -->
  <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <!-- ECharts Fill Rate Chart (2 Cols) -->
    <div class="lg:col-span-2 p-5 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm flex flex-col">
      <div class="flex items-center justify-between mb-3">
        <div>
          <h3 class="text-sm font-bold text-white">Attribute Completeness Matrix</h3>
          <p class="text-xs text-slate-400">Fill rate distribution (%) for all discovered feed fields</p>
        </div>
        <button
          onclick={() => (feedState.activeTab = "completeness")}
          class="flex items-center gap-1 text-xs text-blue-400 hover:text-blue-300 font-medium cursor-pointer"
        >
          <span>View Matrix Table</span>
          <ArrowRight class="w-3.5 h-3.5" />
        </button>
      </div>

      <div bind:this={chartContainer} class="w-full h-80 min-h-[300px]"></div>
    </div>

    <!-- Right Insights Panel (1 Col) -->
    <div class="space-y-4">
      <!-- 100% Populated Attributes -->
      <div class="p-4 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm">
        <div class="flex items-center gap-2 mb-3">
          <CheckCircle2 class="w-4 h-4 text-blue-400" />
          <h4 class="text-xs font-bold text-white uppercase tracking-wider">
            100% Complete ({fullyPopulatedCols().length})
          </h4>
        </div>
        <div class="flex flex-wrap gap-1.5 max-h-36 overflow-y-auto">
          {#each fullyPopulatedCols() as col}
            <button
              onclick={() => {
                feedState.fetchDeepDive(col.column_name);
                feedState.activeTab = "deepdive";
              }}
              class="px-2 py-1 rounded bg-blue-950/40 border border-blue-800/40 text-blue-300 text-xs font-mono hover:bg-blue-900/50 transition cursor-pointer"
            >
              {col.original_name}
            </button>
          {/each}
        </div>
      </div>

      <!-- Attributes with Missing Values -->
      <div class="p-4 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm">
        <div class="flex items-center gap-2 mb-3">
          <AlertTriangle class="w-4 h-4 text-slate-400" />
          <h4 class="text-xs font-bold text-white uppercase tracking-wider">
            Incomplete Fields ({lowFillCols().length})
          </h4>
        </div>
        <div class="space-y-2 max-h-48 overflow-y-auto">
          {#if lowFillCols().length === 0}
            <p class="text-xs text-slate-500 italic">All fields have >90% fill rate</p>
          {:else}
            {#each lowFillCols() as col}
              <button
                onclick={() => {
                  feedState.fetchDeepDive(col.column_name);
                  feedState.activeTab = "deepdive";
                }}
                class="w-full flex items-center justify-between p-2 rounded bg-slate-800/60 hover:bg-slate-800 border border-slate-700/50 text-left transition cursor-pointer"
              >
                <span class="text-xs font-mono text-slate-200 truncate">{col.original_name}</span>
                <span class="text-xs font-bold font-mono text-slate-300 ml-2">{col.fill_rate.toFixed(1)}%</span>
              </button>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>
