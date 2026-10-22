<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { FileUp, Download, Database, Layers, BarChart3, TableProperties, ArrowLeft } from "@lucide/svelte";

  function formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return (bytes / Math.pow(k, i)).toFixed(2) + " " + sizes[i];
  }
</script>

<header class="h-14 glass-chrome px-6 flex items-center justify-between select-none z-30 shrink-0">
  <!-- Brand & Logo -->
  <div class="flex items-center gap-4">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-xl bg-gradient-to-tr from-blue-600 to-cyan-500 flex items-center justify-center shadow-lg shadow-blue-500/20 text-white font-black text-xs border border-white/20 shrink-0">
        FL
      </div>
      <div>
        <div class="flex items-center gap-2">
          <span class="font-extrabold text-base tracking-tight text-white">FeedLens</span>
          <span class="text-[10px] font-mono uppercase tracking-wider font-bold px-2 py-0.5 rounded bg-blue-500/20 text-blue-300 border border-blue-400/30">
            DUCKDB CORE
          </span>
        </div>
        <p class="text-[11px] text-slate-400 -mt-0.5 font-medium">High-Performance Feed Analytics</p>
      </div>
    </div>

    <!-- Active File Badge -->
    {#if feedState.currentFile}
      <div class="h-5 w-px bg-slate-800 mx-1 hidden sm:block"></div>
      <div class="hidden sm:flex items-center gap-2.5 bg-slate-800/80 border border-slate-700/60 rounded-xl px-3 py-1 text-xs">
        <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
        <span class="tabular-tag text-slate-200 font-semibold max-w-[220px] truncate" title={feedState.currentFile.filename}>
          {feedState.currentFile.filename}
        </span>
        {#if feedState.currentFile.size_bytes > 0}
          <span class="text-slate-600">|</span>
          <span class="tabular-tag text-cyan-300 font-bold">
            {formatBytes(feedState.currentFile.size_bytes)}
          </span>
        {/if}
        <span class="tabular-tag px-1.5 py-0.2 rounded text-[10px] font-bold bg-slate-700 text-slate-200 uppercase">
          {feedState.stats?.total_records ? `${feedState.stats.total_records.toLocaleString()} JOBS` : feedState.currentFile.format}
        </span>
      </div>
    {/if}
  </div>

  <!-- Actions -->
  <div class="flex items-center gap-2">
    {#if feedState.stats || feedState.currentFile}
      <!-- Back to Feeds Landing -->
      <button
        onclick={() => feedState.resetToLanding()}
        class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-semibold text-slate-300 bg-slate-800/80 hover:bg-slate-700 hover:text-white border border-slate-700 transition cursor-pointer"
        title="Return to Feeds Home & Ingestion"
      >
        <ArrowLeft class="w-3.5 h-3.5 text-blue-400" />
        <span>Return Home</span>
      </button>
    {/if}

    <!-- Open Feed File Button -->
    <button
      onclick={() => feedState.openFile()}
      disabled={feedState.isIngesting}
      class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-bold text-white bg-blue-600 hover:bg-blue-500 shadow-md shadow-blue-600/20 border border-blue-400/30 transition disabled:opacity-50 cursor-pointer"
    >
      <FileUp class="w-3.5 h-3.5 text-white" />
      <span>New Feed</span>
    </button>

    <!-- Export Menu -->
    {#if feedState.stats && feedState.stats.total_records > 0}
      <div class="h-5 w-px bg-slate-800 mx-1"></div>
      <button
        onclick={() => feedState.exportData(false)}
        title="Export full filtered table to CSV"
        class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-semibold text-slate-200 bg-slate-800/80 hover:bg-slate-700 hover:text-white border border-slate-700 transition cursor-pointer"
      >
        <Download class="w-3.5 h-3.5 text-slate-300" />
        <span>Export CSV</span>
      </button>

      <button
        onclick={() => feedState.exportData(true)}
        title="Export full filtered table to JSON"
        class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-semibold text-slate-200 bg-slate-800/80 hover:bg-slate-700 hover:text-white border border-slate-700 transition cursor-pointer"
      >
        <Download class="w-3.5 h-3.5 text-slate-300" />
        <span>Export JSON</span>
      </button>
    {/if}
  </div>
</header>
