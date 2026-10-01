<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { FileUp, Download, Database, Layers, BarChart3, TableProperties } from "@lucide/svelte";

  function formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return (bytes / Math.pow(k, i)).toFixed(2) + " " + sizes[i];
  }
</script>

<header class="h-14 bg-slate-900/90 border-b border-slate-800 px-4 flex items-center justify-between select-none backdrop-blur-md z-30 shrink-0">
  <!-- Brand & Logo -->
  <div class="flex items-center gap-3">
    <div class="flex items-center gap-2">
      <div class="w-8 h-8 rounded-lg bg-gradient-to-tr from-cyan-500 to-blue-600 flex items-center justify-center shadow-lg shadow-cyan-500/20 text-white font-bold text-lg">
        ⚡
      </div>
      <div>
        <div class="flex items-center gap-2">
          <span class="font-extrabold text-base tracking-tight text-white">FeedLens</span>
          <span class="text-[10px] uppercase tracking-wider font-semibold px-1.5 py-0.5 rounded bg-cyan-950/80 text-cyan-400 border border-cyan-800/50">
            DuckDB Core
          </span>
        </div>
        <p class="text-[11px] text-slate-400 -mt-0.5">High-Performance Feed Analytics</p>
      </div>
    </div>

    <!-- Active File Badge -->
    {#if feedState.currentFile}
      <div class="h-5 w-px bg-slate-800 mx-2 hidden sm:block"></div>
      <div class="hidden sm:flex items-center gap-2 bg-slate-800/60 border border-slate-700/50 rounded-md px-2.5 py-1 text-xs text-slate-300">
        <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
        <span class="font-medium max-w-[220px] truncate" title={feedState.currentFile.filename}>
          {feedState.currentFile.filename}
        </span>
        <span class="text-slate-500">|</span>
        <span class="text-slate-400 font-mono text-[11px]">
          {formatBytes(feedState.currentFile.size_bytes)}
        </span>
        <span class="px-1.5 py-0.2 rounded text-[10px] font-mono uppercase bg-slate-700/80 text-slate-300">
          {feedState.currentFile.format}
        </span>
      </div>
    {/if}
  </div>

  <!-- Actions -->
  <div class="flex items-center gap-2">
    <!-- Open Feed File Button -->
    <button
      onclick={() => feedState.openFile()}
      disabled={feedState.isIngesting}
      class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg text-xs font-semibold text-slate-950 bg-cyan-400 hover:bg-cyan-300 transition shadow-sm disabled:opacity-50 cursor-pointer"
    >
      <FileUp class="w-3.5 h-3.5 text-slate-950" />
      <span>Open Feed File</span>
    </button>

    <!-- Export Menu -->
    {#if feedState.stats && feedState.stats.total_records > 0}
      <div class="h-5 w-px bg-slate-800 mx-1"></div>
      <button
        onclick={() => feedState.exportData(false)}
        title="Export full filtered table to CSV"
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium text-slate-200 bg-slate-800/80 hover:bg-slate-700 border border-slate-700 transition cursor-pointer"
      >
        <Download class="w-3.5 h-3.5 text-emerald-400" />
        <span>Export CSV</span>
      </button>

      <button
        onclick={() => feedState.exportData(true)}
        title="Export full filtered table to JSON"
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium text-slate-200 bg-slate-800/80 hover:bg-slate-700 border border-slate-700 transition cursor-pointer"
      >
        <Download class="w-3.5 h-3.5 text-amber-400" />
        <span>Export JSON</span>
      </button>
    {/if}
  </div>
</header>
