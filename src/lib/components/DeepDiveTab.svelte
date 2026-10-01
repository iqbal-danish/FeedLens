<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { onMount } from "svelte";
  import * as echarts from "echarts";
  import { PieChart, BarChart2, AlertCircle, CheckCircle, RefreshCw } from "@lucide/svelte";

  let chartContainer: HTMLDivElement;
  let chartInstance: echarts.ECharts | null = null;
  let chartType = $state<"bar" | "pie">("bar");

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
              <span class="font-bold text-cyan-400">${item.name}</span>: ${item.value.toLocaleString()} records
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
                { offset: 0, color: "#06b6d4" },
                { offset: 1, color: "#3b82f6" },
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
    return () => {
      window.removeEventListener("resize", handleResize);
      chartInstance?.dispose();
    };
  });
</script>

<div class="p-6 h-full flex flex-col space-y-5 overflow-y-auto">
  <!-- Attribute Selector Header -->
  <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 p-4 rounded-xl bg-slate-900/70 border border-slate-800">
    <div class="flex items-center gap-3">
      <span class="text-xs font-semibold uppercase tracking-wider text-slate-400">Select Attribute:</span>
      <select
        bind:value={feedState.selectedColumn}
        onchange={() => feedState.fetchDeepDive(feedState.selectedColumn)}
        class="bg-slate-950 border border-slate-700 text-cyan-400 font-mono font-bold rounded-lg px-3 py-1.5 text-xs focus:outline-none focus:border-cyan-500 cursor-pointer"
      >
        {#each feedState.completeness as col}
          <option value={col.column_name}>
            {col.original_name} ({col.fill_rate.toFixed(0)}% fill)
          </option>
        {/each}
      </select>
    </div>

    <!-- Quick Stats for chosen col -->
    {#if currentColumnInfo()}
      <div class="flex items-center gap-4 text-xs font-mono">
        <div class="flex items-center gap-1.5 bg-slate-950/80 px-2.5 py-1 rounded border border-slate-800">
          <span class="text-slate-400">Fill:</span>
          <span class="font-bold {currentColumnInfo()!.fill_rate >= 95 ? 'text-emerald-400' : 'text-amber-400'}">
            {currentColumnInfo()!.fill_rate.toFixed(1)}%
          </span>
        </div>
        <div class="flex items-center gap-1.5 bg-slate-950/80 px-2.5 py-1 rounded border border-slate-800">
          <span class="text-slate-400">Distinct Uniques:</span>
          <span class="font-bold text-blue-400">
            {currentColumnInfo()!.unique_count.toLocaleString()}
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
            class="p-1 rounded {chartType === 'bar' ? 'bg-cyan-950 text-cyan-400' : 'text-slate-500 hover:text-slate-300'} transition cursor-pointer"
            title="Horizontal Bar Chart"
          >
            <BarChart2 class="w-3.5 h-3.5" />
          </button>
          <button
            onclick={() => (chartType = "pie")}
            class="p-1 rounded {chartType === 'pie' ? 'bg-cyan-950 text-cyan-400' : 'text-slate-500 hover:text-slate-300'} transition cursor-pointer"
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
              <span class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-950 text-amber-400 border border-amber-800/60">
                {feedState.duplicateEntries.length} Collisions
              </span>
            {:else}
              <span class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-950 text-emerald-400 border border-emerald-800/60">
                Unique
              </span>
            {/if}
          </h3>
          <p class="text-xs text-slate-400">Values that occur multiple times for this attribute</p>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto max-h-80 border border-slate-800/80 rounded-lg">
        {#if feedState.duplicateEntries.length > 0}
          <table class="w-full text-left text-xs">
            <thead class="bg-slate-950/80 sticky top-0 text-slate-400 font-semibold uppercase text-[10px] border-b border-slate-800">
              <tr>
                <th class="py-2.5 px-3">Duplicate Value</th>
                <th class="py-2.5 px-3 text-right">Occurrences</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-800 font-mono text-slate-300">
              {#each feedState.duplicateEntries as entry}
                <tr class="hover:bg-slate-800/40">
                  <td class="py-2 px-3 text-amber-300 truncate max-w-[200px]" title={entry.key}>
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
            <CheckCircle class="w-8 h-8 text-emerald-500 mb-2 opacity-80" />
            <p class="text-xs font-semibold text-slate-300">Zero Duplicates Detected</p>
            <p class="text-[11px] text-slate-500 mt-0.5">All populated values for this attribute are completely unique.</p>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Detailed Frequency Table -->
  <div class="p-5 rounded-xl bg-slate-900/70 border border-slate-800 shadow-sm flex flex-col">
    <h3 class="text-sm font-bold text-white mb-1">Top Frequency Breakdown</h3>
    <p class="text-xs text-slate-400 mb-3">Complete ranking of the most frequent values for <span class="text-cyan-400 font-mono">{feedState.selectedColumn}</span></p>

    <div class="overflow-x-auto border border-slate-800 rounded-lg max-h-72 overflow-y-auto">
      <table class="w-full text-left text-xs border-collapse">
        <thead class="bg-slate-950/80 sticky top-0 text-slate-400 font-semibold uppercase text-[10px] border-b border-slate-800">
          <tr>
            <th class="py-2.5 px-3 w-12 text-center text-slate-600">Rank</th>
            <th class="py-2.5 px-3">Value</th>
            <th class="py-2.5 px-3 text-right">Count</th>
            <th class="py-2.5 px-3 text-right">Distribution Share</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800 font-mono text-slate-300">
          {#each feedState.columnDistribution as dist, i}
            <tr class="hover:bg-slate-800/40">
              <td class="py-2 px-3 text-center text-slate-500 text-[10px]">{i + 1}</td>
              <td class="py-2 px-3 font-semibold text-slate-200 truncate max-w-sm" title={dist.value}>
                {dist.value}
              </td>
              <td class="py-2 px-3 text-right text-cyan-300">{dist.count.toLocaleString()}</td>
              <td class="py-2 px-3 text-right">
                <div class="flex items-center justify-end gap-2">
                  <div class="w-16 bg-slate-800 rounded-full h-1.5 overflow-hidden">
                    <div class="bg-cyan-500 h-full rounded-full" style="width: {dist.percentage}%"></div>
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
