<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { Search, ArrowUpDown, BarChart2, Download } from "@lucide/svelte";

  let searchFilter = $state("");
  let currentTier = $state<"all" | "complete" | "warning" | "critical">("all");
  let sortBy = $state<"name" | "fill" | "uniques" | "valid" | "missing">("fill");
  let sortAsc = $state(false);

  let completeCount = $derived(feedState.completeness.filter((c) => c.fill_rate >= 99.9).length);
  let warningCount = $derived(feedState.completeness.filter((c) => c.fill_rate >= 10.0 && c.fill_rate < 99.9).length);
  let criticalCount = $derived(feedState.completeness.filter((c) => c.fill_rate < 10.0).length);

  let filteredColumns = $derived(() => {
    let list = feedState.completeness.filter((c) => {
      const matchesSearch = !searchFilter || c.original_name.toLowerCase().includes(searchFilter.toLowerCase());
      if (!matchesSearch) return false;

      if (currentTier === "complete") return c.fill_rate >= 99.9;
      if (currentTier === "warning") return c.fill_rate >= 10.0 && c.fill_rate < 99.9;
      if (currentTier === "critical") return c.fill_rate < 10.0;
      return true;
    });

    list.sort((a, b) => {
      let valA: any = a.fill_rate;
      let valB: any = b.fill_rate;
      if (sortBy === "name") {
        valA = a.original_name.toLowerCase();
        valB = b.original_name.toLowerCase();
      } else if (sortBy === "uniques") {
        valA = a.unique_count;
        valB = b.unique_count;
      } else if (sortBy === "valid") {
        valA = a.valid_count;
        valB = b.valid_count;
      } else if (sortBy === "missing") {
        valA = a.empty_count;
        valB = b.empty_count;
      }

      if (valA < valB) return sortAsc ? -1 : 1;
      if (valA > valB) return sortAsc ? 1 : -1;
      return 0;
    });

    return list;
  });

  function toggleSort(field: "name" | "fill" | "uniques" | "valid" | "missing") {
    if (sortBy === field) {
      sortAsc = !sortAsc;
    } else {
      sortBy = field;
      sortAsc = false;
    }
  }

  function setTier(tier: "all" | "complete" | "warning" | "critical") {
    currentTier = tier;
  }

  function getHealthColor(rate: number) {
    if (rate >= 99.9) {
      return {
        bar: "bg-emerald-500",
        text: "text-emerald-400",
        dot: "bg-emerald-400",
      };
    } else if (rate >= 10.0) {
      return {
        bar: "bg-amber-500",
        text: "text-amber-400",
        dot: "bg-amber-400",
      };
    } else {
      return {
        bar: "bg-rose-500",
        text: "text-rose-400",
        dot: "bg-rose-400",
      };
    }
  }

  function exportMatrixCsv() {
    if (feedState.completeness.length === 0) return;
    const headers = ["Attribute", "Fill Rate (%)", "Valid Rows", "Missing Rows", "Distinct Uniques"];
    const rows = feedState.completeness.map((c) => [
      `"${c.original_name.replace(/"/g, '""')}"`,
      c.fill_rate.toFixed(1),
      c.valid_count,
      c.empty_count,
      c.unique_count,
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
  <!-- Controls Bar -->
  <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-4 shrink-0">
    <div class="flex items-center gap-3 flex-wrap flex-1">
      <!-- Search Input -->
      <div class="relative w-72">
        <Search class="w-3.5 h-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          placeholder="Filter {feedState.completeness.length} attributes..."
          bind:value={searchFilter}
          class="w-full bg-slate-900/90 border border-slate-800 rounded-xl pl-9 pr-3 py-2 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 transition shadow-sm font-mono"
        />
      </div>

      <!-- 4 Quick Filter Pills -->
      <div class="flex items-center gap-1.5 flex-wrap">
        <!-- All -->
        <button
          onclick={() => setTier("all")}
          class="px-3 py-1.5 rounded-full text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer {currentTier === 'all'
            ? 'bg-blue-600 text-white shadow-sm'
            : 'bg-slate-900/80 border border-slate-800 text-slate-300 hover:bg-slate-800'}"
        >
          <span>All</span>
          <span class="text-[10px] px-1.5 py-0.2 rounded-full font-mono {currentTier === 'all' ? 'bg-blue-900/60 text-blue-200' : 'bg-slate-800 text-slate-400'}">
            {feedState.completeness.length}
          </span>
        </button>

        <!-- Complete -->
        <button
          onclick={() => setTier("complete")}
          class="px-3 py-1.5 rounded-full text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer {currentTier === 'complete'
            ? 'bg-emerald-600 text-white shadow-sm'
            : 'bg-slate-900/80 border border-slate-800 text-emerald-300 hover:bg-slate-800'}"
        >
          <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
          <span>100% Complete</span>
          <span class="text-[10px] px-1.5 py-0.2 rounded-full font-mono {currentTier === 'complete' ? 'bg-emerald-900/60 text-emerald-200' : 'bg-slate-800 text-emerald-400'}">
            {completeCount}
          </span>
        </button>

        <!-- Warning -->
        <button
          onclick={() => setTier("warning")}
          class="px-3 py-1.5 rounded-full text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer {currentTier === 'warning'
            ? 'bg-amber-600 text-white shadow-sm'
            : 'bg-slate-900/80 border border-slate-800 text-amber-300 hover:bg-slate-800'}"
        >
          <span class="w-2 h-2 rounded-full bg-amber-400"></span>
          <span>Partial</span>
          <span class="text-[10px] px-1.5 py-0.2 rounded-full font-mono {currentTier === 'warning' ? 'bg-amber-900/60 text-amber-200' : 'bg-slate-800 text-amber-400'}">
            {warningCount}
          </span>
        </button>

        <!-- Critical -->
        <button
          onclick={() => setTier("critical")}
          class="px-3 py-1.5 rounded-full text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer {currentTier === 'critical'
            ? 'bg-rose-600 text-white shadow-sm'
            : 'bg-slate-900/80 border border-slate-800 text-rose-300 hover:bg-slate-800'}"
        >
          <span class="w-2 h-2 rounded-full bg-rose-400"></span>
          <span>Critical (&lt;10%)</span>
          <span class="text-[10px] px-1.5 py-0.2 rounded-full font-mono {currentTier === 'critical' ? 'bg-rose-900/60 text-rose-200' : 'bg-slate-800 text-rose-400'}">
            {criticalCount}
          </span>
        </button>
      </div>
    </div>

    <!-- Right Controls: Count & Export -->
    <div class="flex items-center gap-3 shrink-0">
      <div class="text-xs text-slate-400 font-mono">
        Showing <span class="text-white font-bold">{filteredColumns().length}</span> of {feedState.completeness.length} attributes
      </div>
      <button
        onclick={exportMatrixCsv}
        class="px-3 py-1.5 rounded-xl bg-slate-900/90 hover:bg-slate-800 border border-slate-700/80 text-xs font-semibold text-slate-200 hover:text-white flex items-center gap-2 transition cursor-pointer shadow-sm"
        title="Download matrix as CSV"
      >
        <Download class="w-3.5 h-3.5 text-blue-400" />
        <span>Export Matrix</span>
      </button>
    </div>
  </div>

  <!-- Matrix Table Container -->
  <div class="flex-1 data-card rounded-2xl overflow-hidden flex flex-col">
    <div class="overflow-x-auto overflow-y-auto flex-1 isolate">
      <table class="w-full text-left text-xs border-collapse">
        <thead class="bg-slate-950/95 sticky top-0 z-10 border-b border-slate-800 text-slate-400 font-bold uppercase tracking-wider text-[10px] backdrop-blur-md">
          <tr>
            <th class="py-3.5 px-4 cursor-pointer hover:text-white" onclick={() => toggleSort("name")}>
              <div class="flex items-center gap-1.5">
                <span>Attribute</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3.5 px-4 text-right cursor-pointer hover:text-white" onclick={() => toggleSort("fill")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Fill Rate</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3.5 px-4 text-right cursor-pointer hover:text-white" onclick={() => toggleSort("valid")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Valid Rows</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3.5 px-4 text-right cursor-pointer hover:text-white" onclick={() => toggleSort("missing")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Missing / Null</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3.5 px-4 text-right cursor-pointer hover:text-white" onclick={() => toggleSort("uniques")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Distinct Uniques</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3.5 px-4">Sample Values</th>
            <th class="py-3.5 px-4 text-center">Action</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/60 font-mono text-slate-300">
          {#if filteredColumns().length === 0}
            <tr>
              <td colspan="7" class="py-12 text-center text-slate-500 italic">
                No attributes match the selected filter or search query.
              </td>
            </tr>
          {:else}
            {#each filteredColumns() as col}
              {@const health = getHealthColor(col.fill_rate)}
              <tr class="hover:bg-white/[0.02] transition">
                <!-- Attribute Name -->
                <td class="py-3 px-4 font-semibold text-white">
                  <div class="flex items-center gap-2">
                    <span class="w-2 h-2 rounded-full {health.dot} shrink-0"></span>
                    <span class="font-mono text-slate-200 hover:text-blue-400 transition cursor-pointer truncate max-w-md" title={col.original_name}>
                      {col.original_name}
                    </span>
                  </div>
                </td>

                <!-- Fill Rate (Progress Bar + Percentage) -->
                <td class="py-3 px-4 text-right">
                  <div class="flex items-center justify-end gap-2.5">
                    <div class="w-20 bg-slate-800 rounded-full h-1.5 overflow-hidden hidden sm:block">
                      <div
                        class="h-full rounded-full {health.bar} animate-fill-bar transition-all duration-300"
                        style="--fill-width: {Math.max(2, col.fill_rate)}%; width: {Math.max(2, col.fill_rate)}%;"
                      ></div>
                    </div>
                    <span class="font-bold {health.text} font-mono w-14 text-right">
                      {col.fill_rate.toFixed(1)}%
                    </span>
                  </div>
                </td>

                <!-- Valid Rows -->
                <td class="py-3 px-4 text-right text-slate-200 font-mono">
                  {col.valid_count.toLocaleString()}
                </td>

                <!-- Missing / Null -->
                <td class="py-3 px-4 text-right font-mono">
                  {#if col.empty_count > 0}
                    <span class="font-bold {col.fill_rate < 10 ? 'text-rose-400' : 'text-amber-400'}">
                      {col.empty_count.toLocaleString()}
                    </span>
                  {:else}
                    <span class="text-slate-600">—</span>
                  {/if}
                </td>

                <!-- Distinct Uniques Badge -->
                <td class="py-3 px-4 text-right">
                  <span class="px-2 py-0.5 rounded-md bg-blue-500/10 text-cyan-300 border border-blue-500/25 font-bold font-mono text-[11px]">
                    {col.unique_count.toLocaleString()}
                  </span>
                </td>

                <!-- Sample Values Tags -->
                <td class="py-3 px-4 max-w-sm truncate font-sans text-xs">
                  <div class="flex flex-wrap gap-1">
                    {#if col.sample_values && col.sample_values.length > 0}
                      {#each col.sample_values.slice(0, 3) as sample}
                        <span class="px-1.5 py-0.5 rounded-md bg-slate-900 border border-slate-800 text-slate-300 text-[11px] font-mono truncate max-w-[140px]" title={sample}>
                          {sample}
                        </span>
                      {/each}
                    {:else}
                      <span class="text-slate-600 italic text-[11px]">No sample data</span>
                    {/if}
                  </div>
                </td>

                <!-- Action Button -->
                <td class="py-3 px-4 text-center font-sans">
                  <button
                    onclick={() => {
                      feedState.fetchDeepDive(col.column_name);
                      feedState.activeTab = "deepdive";
                    }}
                    class="p-1.5 rounded-lg bg-slate-900 hover:bg-blue-950/70 text-slate-400 hover:text-blue-300 border border-slate-800 hover:border-blue-700/60 transition cursor-pointer shadow-sm"
                    title="Inspect '{col.original_name}' in Deep Dive"
                  >
                    <BarChart2 class="w-3.5 h-3.5" />
                  </button>
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>
  </div>
</div>
