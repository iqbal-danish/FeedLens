<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { Search, ArrowUpDown, BarChart2 } from "@lucide/svelte";

  let searchFilter = $state("");
  let sortBy = $state<"name" | "fill" | "uniques">("fill");
  let sortAsc = $state(false);

  let filteredColumns = $derived(() => {
    let list = feedState.completeness.filter((c) =>
      c.original_name.toLowerCase().includes(searchFilter.toLowerCase())
    );

    list.sort((a, b) => {
      let valA: any = a.fill_rate;
      let valB: any = b.fill_rate;
      if (sortBy === "name") {
        valA = a.original_name.toLowerCase();
        valB = b.original_name.toLowerCase();
      } else if (sortBy === "uniques") {
        valA = a.unique_count;
        valB = b.unique_count;
      }

      if (valA < valB) return sortAsc ? -1 : 1;
      if (valA > valB) return sortAsc ? 1 : -1;
      return 0;
    });

    return list;
  });

  function toggleSort(field: "name" | "fill" | "uniques") {
    if (sortBy === field) {
      sortAsc = !sortAsc;
    } else {
      sortBy = field;
      sortAsc = false;
    }
  }
</script>

<div class="p-6 h-full flex flex-col space-y-4">
  <!-- Controls Bar -->
  <div class="flex items-center justify-between gap-4 shrink-0">
    <div class="relative max-w-sm w-full">
      <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
      <input
        type="text"
        placeholder="Filter attributes..."
        bind:value={searchFilter}
        class="w-full bg-slate-900 border border-slate-800 rounded-lg pl-9 pr-3 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 transition"
      />
    </div>

    <div class="text-xs text-slate-400 font-mono">
      Showing <span class="text-white font-bold">{filteredColumns().length}</span> of {feedState.completeness.length} attributes
    </div>
  </div>

  <!-- Matrix Table Container -->
  <div class="flex-1 bg-slate-900/70 border border-slate-800 rounded-xl overflow-hidden flex flex-col">
    <div class="overflow-x-auto overflow-y-auto flex-1 isolate">
      <table class="w-full text-left text-xs border-collapse">
        <thead class="bg-slate-950 sticky top-0 z-10 border-b border-slate-800 text-slate-400 font-semibold uppercase tracking-wider text-[11px]">
          <tr class="bg-slate-950">
            <th class="py-3 px-4 bg-slate-950 cursor-pointer hover:text-white" onclick={() => toggleSort("name")}>
              <div class="flex items-center gap-1.5">
                <span>Attribute</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-4 text-right cursor-pointer hover:text-white" onclick={() => toggleSort("fill")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Fill Rate</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-4 text-right">Valid Rows</th>
            <th class="py-3 px-4 text-right">Missing / Null</th>
            <th class="py-3 px-4 text-right cursor-pointer hover:text-white" onclick={() => toggleSort("uniques")}>
              <div class="flex items-center justify-end gap-1.5">
                <span>Distinct Uniques</span>
                <ArrowUpDown class="w-3 h-3 text-slate-500" />
              </div>
            </th>
            <th class="py-3 px-4">Sample Values</th>
            <th class="py-3 px-4 text-center">Action</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/60 font-mono text-slate-300">
          {#each filteredColumns() as col}
            <tr class="hover:bg-slate-800/40 transition">
              <!-- Name -->
              <td class="py-3 px-4 font-semibold text-white">
                <span class="font-mono text-blue-300">{col.original_name}</span>
              </td>

              <!-- Fill Rate -->
              <td class="py-3 px-4 text-right">
                <div class="flex items-center justify-end gap-2">
                  <div class="w-16 bg-slate-800 rounded-full h-1.5 overflow-hidden hidden sm:block">
                    <div
                      class="h-full rounded-full {col.fill_rate >= 95 ? 'bg-blue-500' : col.fill_rate >= 80 ? 'bg-indigo-500' : 'bg-slate-600'}"
                      style="width: {col.fill_rate}%"
                    ></div>
                  </div>
                  <span class="font-bold {col.fill_rate >= 95 ? 'text-blue-400' : col.fill_rate >= 80 ? 'text-indigo-400' : 'text-slate-400'}">
                    {col.fill_rate.toFixed(1)}%
                  </span>
                </div>
              </td>

              <!-- Valid -->
              <td class="py-3 px-4 text-right text-slate-300">
                {col.valid_count.toLocaleString()}
              </td>

              <!-- Missing -->
              <td class="py-3 px-4 text-right">
                {#if col.empty_count > 0}
                  <span class="text-slate-400 font-medium">{col.empty_count.toLocaleString()}</span>
                {:else}
                  <span class="text-slate-600">0</span>
                {/if}
              </td>

              <!-- Unique count -->
              <td class="py-3 px-4 text-right text-blue-400 font-semibold">
                {col.unique_count.toLocaleString()}
              </td>

              <!-- Samples -->
              <td class="py-3 px-4 max-w-xs truncate font-sans text-xs">
                <div class="flex flex-wrap gap-1">
                  {#each col.sample_values as sample}
                    <span class="px-1.5 py-0.5 rounded bg-slate-800 border border-slate-700/60 text-slate-300 text-[11px] truncate max-w-[120px]" title={sample}>
                      {sample}
                    </span>
                  {/each}
                </div>
              </td>

              <!-- Action Deep Dive -->
              <td class="py-3 px-4 text-center font-sans">
                <button
                  onclick={() => {
                    feedState.fetchDeepDive(col.column_name);
                    feedState.activeTab = "deepdive";
                  }}
                  class="p-1.5 rounded-lg bg-slate-800 hover:bg-blue-950/60 text-slate-400 hover:text-blue-400 border border-slate-700 hover:border-blue-800 transition cursor-pointer"
                  title="Inspect Value Distribution"
                >
                  <BarChart2 class="w-3.5 h-3.5" />
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>
