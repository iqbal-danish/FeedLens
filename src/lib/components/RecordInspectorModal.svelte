<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { X, Copy, Check } from "@lucide/svelte";

  let copiedKey = $state<string | null>(null);

  function copyValue(key: string, val: string) {
    navigator.clipboard.writeText(val);
    copiedKey = key;
    setTimeout(() => {
      if (copiedKey === key) copiedKey = null;
    }, 2000);
  }
</script>

{#if feedState.inspectingRecord}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm animate-in fade-in duration-150">
    <div class="bg-slate-900 border border-slate-700/80 rounded-2xl w-full max-w-2xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden">
      <!-- Modal Header -->
      <div class="px-6 py-4 border-b border-slate-800 flex items-center justify-between bg-slate-950/50">
        <div>
          <h3 class="text-sm font-bold text-white">Record Details Inspector</h3>
          <p class="text-xs text-slate-400">Inspecting all field values for this feed record</p>
        </div>
        <button
          onclick={() => (feedState.inspectingRecord = null)}
          class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-6 overflow-y-auto divide-y divide-slate-800/60 font-mono text-xs">
        {#each Object.entries(feedState.inspectingRecord) as [key, value]}
          <div class="py-2.5 flex items-start justify-between gap-4 group">
            <span class="text-cyan-400 font-semibold shrink-0 min-w-[140px]">{key}</span>
            <div class="flex-1 text-slate-200 break-all">
              {#if value === null || value === undefined || value.trim() === ""}
                <span class="text-slate-600 italic">[EMPTY / NULL]</span>
              {:else}
                <span>{value}</span>
              {/if}
            </div>
            {#if value}
              <button
                onclick={() => copyValue(key, value)}
                class="p-1 rounded text-slate-500 hover:text-slate-300 opacity-0 group-hover:opacity-100 transition cursor-pointer"
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
        {/each}
      </div>

      <!-- Modal Footer -->
      <div class="px-6 py-3 border-t border-slate-800 bg-slate-950/50 flex justify-end">
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
