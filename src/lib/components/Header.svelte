<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { FileUp, Download, ArrowLeft } from "@lucide/svelte";

  function formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return (bytes / Math.pow(k, i)).toFixed(2) + " " + sizes[i];
  }

  async function handleMinimize() {
    try {
      await invoke("window_minimize");
    } catch (e) {
      console.warn("Window minimize not supported in browser dev", e);
    }
  }

  async function handleToggleMaximize() {
    try {
      await invoke("window_toggle_maximize");
    } catch (e) {
      console.warn("Window toggle maximize not supported in browser dev", e);
    }
  }

  async function handleClose() {
    try {
      await invoke("window_close");
    } catch (e) {
      console.warn("Window close not supported in browser dev", e);
    }
  }
</script>

<header
  data-tauri-drag-region
  class="h-13 glass-chrome pl-5 pr-0 flex items-center justify-between select-none z-30 shrink-0"
>
  <!-- Brand & Logo (Left) -->
  <div class="flex items-center gap-4">
    <div class="flex items-center gap-3">
      <!-- Aesthetic FeedLens Mark -->
      <div class="w-8 h-8 rounded-xl bg-slate-900/90 border border-slate-700/80 flex items-center justify-center shadow-inner shrink-0 group">
        <svg class="w-4.5 h-4.5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <!-- Focal Lens Outer Ring -->
          <circle cx="12" cy="12" r="8.5" stroke="url(#feedlens-brand-grad)" stroke-width="1.75" />
          <!-- Aperture Optics / Crosshairs -->
          <path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3" stroke="#64748b" stroke-width="1.5" stroke-linecap="round" />
          <!-- Inner Iris Core -->
          <circle cx="12" cy="12" r="3.75" fill="url(#feedlens-brand-grad)" fill-opacity="0.2" stroke="url(#feedlens-brand-grad)" stroke-width="1.5" />
          <circle cx="12" cy="12" r="1.5" fill="#60a5fa" />
          <defs>
            <linearGradient id="feedlens-brand-grad" x1="3" y1="3" x2="21" y2="21" gradientUnits="userSpaceOnUse">
              <stop stop-color="#38bdf8" />
              <stop offset="1" stop-color="#3b82f6" />
            </linearGradient>
          </defs>
        </svg>
      </div>

      <!-- Aesthetic Brand Typography -->
      <div class="flex items-center">
        <span class="text-[17px] font-medium tracking-tight text-slate-100 font-sans select-none">
          Feed<span class="font-bold text-blue-400">Lens</span>
        </span>
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

  <!-- Draggable Center Space -->
  <div class="flex-1 h-full cursor-default" data-tauri-drag-region></div>

  <!-- Actions & In-App Window Controls (Right) -->
  <div class="flex items-center h-full">
    <div class="flex items-center gap-2 pr-3">
      {#if feedState.stats || feedState.currentFile}
        <!-- Back to Feeds Landing -->
        <button
          onclick={() => feedState.resetToLanding()}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold text-slate-300 bg-slate-800/80 hover:bg-slate-700 hover:text-white border border-slate-700 transition cursor-pointer"
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
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-bold text-white bg-blue-600 hover:bg-blue-500 shadow-md shadow-blue-600/20 border border-blue-400/30 transition disabled:opacity-50 cursor-pointer"
      >
        <FileUp class="w-3.5 h-3.5 text-white" />
        <span>New Feed</span>
      </button>

      <!-- Export Menu -->
      {#if feedState.stats && feedState.stats.total_records > 0}
        <div class="h-4 w-px bg-slate-800 mx-1"></div>
        <button
          onclick={() => feedState.exportData(false)}
          title="Export full filtered table to CSV"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold text-slate-200 bg-slate-800/80 hover:bg-slate-700 hover:text-white border border-slate-700 transition cursor-pointer"
        >
          <Download class="w-3.5 h-3.5 text-slate-300" />
          <span>Export CSV</span>
        </button>

        <button
          onclick={() => feedState.exportData(true)}
          title="Export full filtered table to JSON"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold text-slate-200 bg-slate-800/80 hover:bg-slate-700 hover:text-white border border-slate-700 transition cursor-pointer"
        >
          <Download class="w-3.5 h-3.5 text-slate-300" />
          <span>Export JSON</span>
        </button>
      {/if}
    </div>

    <!-- Divider before window controls -->
    <div class="h-5 w-px bg-slate-800/90"></div>

    <!-- In-App Window Controls (Linear Style) -->
    <div class="flex items-center h-full">
      <button
        onclick={handleMinimize}
        class="w-12 h-full flex items-center justify-center text-slate-400 hover:text-white hover:bg-white/[0.06] transition-colors cursor-pointer"
        title="Minimize"
        aria-label="Minimize Window"
      >
        <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none">
          <rect y="5.5" width="12" height="1" fill="currentColor" />
        </svg>
      </button>

      <button
        onclick={handleToggleMaximize}
        class="w-12 h-full flex items-center justify-center text-slate-400 hover:text-white hover:bg-white/[0.06] transition-colors cursor-pointer"
        title="Maximize / Restore"
        aria-label="Maximize Window"
      >
        <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.2">
          <rect x="1" y="1" width="10" height="10" rx="1" />
        </svg>
      </button>

      <button
        onclick={handleClose}
        class="w-12 h-full flex items-center justify-center text-slate-400 hover:text-white hover:bg-rose-600 transition-colors cursor-pointer"
        title="Close"
        aria-label="Close Window"
      >
        <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.3">
          <path d="M1.5 1.5L10.5 10.5M10.5 1.5L1.5 10.5" />
        </svg>
      </button>
    </div>
  </div>
</header>
