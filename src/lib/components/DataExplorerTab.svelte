<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { Search, ArrowUpDown, ChevronLeft, ChevronRight, Eye, RefreshCw } from "@lucide/svelte";

  let searchInput = $state(feedState.searchFilter);

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      feedState.applySearch(searchInput);
    }
  }

  function handleSearchClear() {
    searchInput = "";
    feedState.applySearch("");
  }
</script>

<div class="p-6 h-full flex flex-col space-y-4">
  <!-- Controls Bar -->
  <div class="flex flex-col sm:flex-row items-center justify-between gap-3 shrink-0">
    <!-- Search Box -->
    <div class="flex items-center gap-2 max-w-md w-full">
      <div class="relative flex-1">
        <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          placeholder="Search records across all attributes... (Press Enter)"
          bind:value={searchInput}
          onkeydown={handleSearchKeydown}
          class="w-full bg-slate-900 border border-slate-800 rounded-lg pl-9 pr-8 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 transition"
        />
        {#if searchInput}
          <button
            onclick={handleSearchClear}
            class="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-500 hover:text-white text-xs cursor-pointer"
          >
            ✕
          </button>
        {/if}
      </div>
      <button
        onclick={() => feedState.applySearch(searchInput)}
        class="px-3 py-1.5 rounded-lg text-xs font-semibold text-white bg-blue-600 hover:bg-blue-500 transition cursor-pointer"
      >
        Filter
      </button>
    </div>

    <!-- Pagination & Page Size -->
    <div class="flex items-center gap-3">
      <!-- Page Size Selector -->
      <div class="flex items-center gap-1 text-xs text-slate-400">
        <span>Rows:</span>
        <select
          bind:value={feedState.pageSize}
          onchange={() => {
            feedState.currentPage = 1;
            feedState.fetchTablePage();
          }}
          class="bg-slate-900 border border-slate-800 text-slate-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-blue-500"
        >
          <option value={25}>25</option>
          <option value={50}>50</option>
          <option value={100}>100</option>
          <option value={250}>250</option>
        </select>
      </div>

      <!-- Pagination Buttons -->
      {#if feedState.tableData}
        <div class="flex items-center gap-1.5 text-xs font-mono text-slate-300">
          <button
            onclick={() => feedState.setPage(feedState.currentPage - 1)}
            disabled={feedState.currentPage <= 1 || feedState.isLoadingTable}
            class="p-1 rounded bg-slate-800 hover:bg-slate-700 disabled:opacity-40 disabled:hover:bg-slate-800 transition cursor-pointer"
          >
            <ChevronLeft class="w-4 h-4" />
          </button>
          <span>
            {feedState.currentPage} / {feedState.tableData.total_pages}
          </span>
          <button
            onclick={() => feedState.setPage(feedState.currentPage + 1)}
            disabled={feedState.currentPage >= feedState.tableData.total_pages || feedState.isLoadingTable}
            class="p-1 rounded bg-slate-800 hover:bg-slate-700 disabled:opacity-40 disabled:hover:bg-slate-800 transition cursor-pointer"
          >
            <ChevronRight class="w-4 h-4" />
          </button>
        </div>
      {/if}
    </div>
  </div>

  <!-- Data Table Container -->
  <div class="flex-1 bg-slate-900/70 border border-slate-800 rounded-xl overflow-hidden flex flex-col relative">
    {#if feedState.isLoadingTable}
      <div class="absolute inset-0 bg-slate-950/40 backdrop-blur-[2px] z-20 flex items-center justify-center">
        <RefreshCw class="w-6 h-6 text-blue-400 animate-spin" />
      </div>
    {/if}

    <div class="overflow-x-auto overflow-y-auto flex-1 isolate">
      <table class="w-full text-left text-xs border-collapse">
        <thead class="bg-slate-950 sticky top-0 z-10 border-b border-slate-800 text-slate-400 font-semibold uppercase tracking-wider text-[11px]">
          <tr class="bg-slate-950">
            <th class="py-3 px-3 w-10 text-center text-slate-600 bg-slate-950">#</th>
            {#if feedState.tableData}
              {#each feedState.tableData.columns as col}
                <th
                  class="py-3 px-3 cursor-pointer hover:text-white whitespace-nowrap bg-slate-950"
                  onclick={() => feedState.setSort(col)}
                >
                  <div class="flex items-center gap-1.5">
                    <span>{col}</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-500" />
                  </div>
                </th>
              {/each}
            {/if}
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/60 font-mono text-slate-300">
          {#if feedState.tableData && feedState.tableData.rows.length > 0}
            {#each feedState.tableData.rows as row, idx}
              <tr
                onclick={() => (feedState.inspectingRecord = row)}
                class="hover:bg-slate-800/60 transition cursor-pointer group"
              >
                <!-- Row index -->
                <td class="py-2.5 px-3 text-center text-slate-600 font-mono text-[10px] group-hover:text-blue-400">
                  {(feedState.currentPage - 1) * feedState.pageSize + idx + 1}
                </td>
                <!-- Columns -->
                {#each feedState.tableData.columns as col}
                  <td class="py-2.5 px-3 max-w-[240px] truncate">
                    {#if row[col] === null || row[col] === undefined || row[col] === ""}
                      <span class="text-slate-600 italic font-sans text-[11px]">-</span>
                    {:else}
                      <span title={row[col]}>{row[col]}</span>
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
    <div class="px-4 py-2 bg-slate-950/80 border-t border-slate-800 flex items-center justify-between text-xs text-slate-400">
      <div class="flex items-center gap-2">
        <Eye class="w-3.5 h-3.5 text-blue-400" />
        <span>Click any row to inspect all fields</span>
      </div>
      {#if feedState.tableData}
        <div class="font-mono">
          Total Matching: <span class="text-white font-bold">{feedState.tableData.total_records.toLocaleString()}</span> records
        </div>
      {/if}
    </div>
  </div>
</div>
