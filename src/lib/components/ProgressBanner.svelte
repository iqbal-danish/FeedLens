<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { Activity, XCircle, Clock, Zap, AlertCircle, X } from "@lucide/svelte";

  function formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return (bytes / Math.pow(k, i)).toFixed(2) + " " + sizes[i];
  }

  function formatTime(secs: number): string {
    if (!secs || isNaN(secs)) return "0s";
    if (secs < 60) return `${secs.toFixed(1)}s`;
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}m ${s}s`;
  }
</script>

{#if feedState.isIngesting && feedState.progress && feedState.progress.status === 'indexing'}
  <div class="bg-gradient-to-r from-slate-900 via-slate-850 to-slate-900 border-b border-blue-500/30 px-5 py-3 shadow-xl backdrop-blur-md relative overflow-hidden shrink-0">
    <!-- Top Progress Row -->
    <div class="flex items-center justify-between gap-4 mb-2">
      <div class="flex items-center gap-3">
        <div class="w-7 h-7 rounded-md bg-blue-500/20 text-blue-400 flex items-center justify-center animate-pulse">
          <Activity class="w-4 h-4" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <span class="text-xs font-bold uppercase tracking-wider text-blue-400">
              Streaming Ingestion & DuckDB Vector Indexing
            </span>
            <span class="text-xs font-mono font-bold text-white bg-slate-800 px-1.5 py-0.5 rounded border border-slate-700">
              {feedState.progress.percentage.toFixed(1)}%
            </span>
          </div>
          <p class="text-xs text-slate-400">
            {feedState.progress.current_phase} • <span class="font-mono text-white font-semibold">{feedState.progress.records_ingested.toLocaleString()}</span> records parsed
          </p>
        </div>
      </div>

      <!-- Live Throughput Metrics & Actions -->
      <div class="flex items-center gap-4 text-xs font-mono">
        <div class="flex items-center gap-1.5 bg-slate-800/80 px-2.5 py-1 rounded border border-slate-700/60 text-slate-300">
          <Zap class="w-3.5 h-3.5 text-blue-400" />
          <span>{feedState.progress.mb_per_sec.toFixed(1)} MB/s</span>
          <span class="text-slate-500">|</span>
          <span class="text-blue-300">{Math.round(feedState.progress.records_per_sec).toLocaleString()} rec/s</span>
        </div>

        <div class="flex items-center gap-1.5 bg-slate-800/80 px-2.5 py-1 rounded border border-slate-700/60 text-slate-300">
          <Clock class="w-3.5 h-3.5 text-blue-400" />
          <span>{formatTime(feedState.progress.elapsed_secs)}</span>
          {#if feedState.progress.estimated_remaining_secs}
            <span class="text-slate-500">|</span>
            <span class="text-slate-400">ETA {formatTime(feedState.progress.estimated_remaining_secs)}</span>
          {/if}
        </div>

        <!-- Cancel Ingestion -->
        <button
          onclick={() => feedState.cancelIngest()}
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-red-950/60 hover:bg-red-900/60 text-red-300 border border-red-800/50 hover:border-red-700 transition cursor-pointer text-xs"
        >
          <XCircle class="w-3.5 h-3.5 text-red-400" />
          <span>Cancel</span>
        </button>
      </div>
    </div>

    <!-- Animated Smooth Progress Bar -->
    <div class="w-full bg-slate-800 rounded-full h-2 overflow-hidden border border-slate-700/60">
      <div
        class="bg-gradient-to-r from-blue-600 via-blue-500 to-indigo-600 h-full rounded-full transition-all duration-150 ease-out"
        style="width: {feedState.progress.percentage}%"
      ></div>
    </div>
  </div>
{:else}
  {#if feedState.lastError && feedState.hasActiveDataset}
    <div class="bg-red-950/90 border-b border-red-800/60 px-5 py-2.5 shadow-xl text-red-200 text-xs flex items-center justify-between gap-4 shrink-0">
      <div class="flex items-center gap-2.5">
        <AlertCircle class="w-4 h-4 text-red-400 shrink-0" />
        <span class="font-bold text-red-300">Ingestion Error:</span>
        <span class="font-mono text-[11px] text-red-200">{feedState.lastError}</span>
      </div>
      <button
        onclick={() => feedState.clearError()}
        class="text-red-400 hover:text-white p-1 rounded transition cursor-pointer shrink-0"
      >
        <X class="w-4 h-4" />
      </button>
    </div>
  {/if}
{/if}
