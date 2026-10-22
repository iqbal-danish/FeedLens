<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { X, Copy, Check, Search } from "@lucide/svelte";

  let copiedKey = $state<string | null>(null);
  let filterQuery = $state("");

  function copyValue(key: string, val: string) {
    navigator.clipboard.writeText(val);
    copiedKey = key;
    setTimeout(() => {
      if (copiedKey === key) copiedKey = null;
    }, 2000);
  }

  let filteredEntries = $derived(() => {
    if (!feedState.inspectingRecord) return [];
    const entries = Object.entries(feedState.inspectingRecord);
    if (!filterQuery.trim()) return entries;
    const q = filterQuery.toLowerCase();
    return entries.filter(([k, v]) =>
      k.toLowerCase().includes(q) || (v && String(v).toLowerCase().includes(q))
    );
  });
</script>

{#if feedState.inspectingRecord}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-md animate-in fade-in duration-150">
    <div class="glass-modal-panel rounded-3xl w-full max-w-4xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden border border-white/15">
      <!-- Modal Header -->
      <div class="px-6 py-4 border-b border-white/10 flex flex-col sm:flex-row sm:items-center justify-between gap-3 bg-white/[0.02]">
        <div>
          <h3 class="text-sm font-bold text-white tracking-tight">Record Details Inspector</h3>
          <p class="text-xs text-slate-400">Inspecting all field values for this feed record</p>
        </div>
        
        <div class="flex items-center gap-3">
          <!-- Quick Field Search -->
          <div class="relative w-48 sm:w-64">
            <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              type="text"
              placeholder="Filter fields..."
              bind:value={filterQuery}
              class="w-full bg-slate-950 border border-slate-800 rounded-lg pl-8 pr-3 py-1 text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500 font-mono transition"
            />
          </div>

          <button
            onclick={() => (feedState.inspectingRecord = null)}
            class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
            title="Close"
          >
            <X class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Modal Body -->
      <div class="p-6 overflow-y-auto divide-y divide-slate-800/60 font-mono text-xs">
        {#if filteredEntries().length === 0}
          <div class="py-12 text-center text-xs text-slate-500 font-mono">
            No fields matching "{filterQuery}"
          </div>
        {:else}
          {#each filteredEntries() as [key, value]}
            <div class="py-2.5 px-2 rounded-lg hover:bg-slate-800/40 flex items-start gap-4 group transition">
              <!-- Key Column (Balanced width, wraps cleanly instead of crushing the value) -->
              <div class="w-5/12 min-w-[200px] max-w-[340px] shrink-0" title={key}>
                <span class="text-blue-400 font-semibold break-all leading-relaxed select-all">
                  {key}
                </span>
              </div>

              <!-- Value Column (Generous space, never crushed into vertical letters) -->
              <div class="flex-1 min-w-[200px] text-slate-200 break-all leading-relaxed select-all">
                {#if value === null || value === undefined || value.trim() === ""}
                  <span class="text-slate-600 italic font-sans">[EMPTY / NULL]</span>
                {:else}
                  <span>{value}</span>
                {/if}
              </div>

              <!-- Copy Value Button -->
              <div class="shrink-0 w-8 flex justify-end">
                {#if value}
                  <button
                    onclick={() => copyValue(key, value)}
                    class="p-1 rounded text-slate-500 hover:text-slate-200 hover:bg-slate-800 transition cursor-pointer"
                    title="Copy value"
                  >
                    {#if copiedKey === key}
                      <Check class="w-3.5 h-3.5 text-emerald-400" />
                    {:else}
                      <Copy class="w-3.5 h-3.5" />
                    {/if}
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <!-- Modal Footer -->
      <div class="px-6 py-3 border-t border-slate-800 bg-slate-950/50 flex items-center justify-between text-xs text-slate-400 font-mono">
        <span>{filteredEntries().length} fields displayed</span>
        <button
          onclick={() => (feedState.inspectingRecord = null)}
          class="px-4 py-1.5 rounded-lg text-xs font-semibold text-slate-300 bg-slate-800 hover:bg-slate-700 transition cursor-pointer"
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}
