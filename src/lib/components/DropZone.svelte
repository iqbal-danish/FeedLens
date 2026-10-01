<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import type { RecentFeed } from "$lib/types";
  import {
    FileUp,
    Globe,
    FolderOpen,
    Play,
    Clock,
    Trash2,
    ArrowRight,
    Tag,
    Gauge,
    Database,
    HardDrive,
    Layers,
    FileText,
    AlertCircle,
    X,
  } from "@lucide/svelte";

  let sourceType = $state<"file" | "url">("file");
  let selectedFilePath = $state("");
  let selectedFileObj = $state<{ path: string; filename: string; size_bytes: number; format: string } | null>(null);
  let feedUrl = $state("");
  let jobElement = $state("Auto");
  let ingestionMode = $state<"extreme" | "fast" | "full">("extreme");

  async function handleBrowse() {
    const file = await feedState.pickFeedFile();
    if (file) {
      selectedFileObj = file;
      selectedFilePath = file.path;
    }
  }

  function formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return (bytes / Math.pow(k, i)).toFixed(2) + " " + sizes[i];
  }

  async function handleSubmit(e?: Event) {
    if (e) e.preventDefault();

    if (sourceType === "file") {
      if (!selectedFilePath) {
        feedState.showToast("Please select a feed file first.");
        return;
      }
      feedState.currentFile = selectedFileObj || {
        path: selectedFilePath,
        filename: selectedFilePath.split(/[\\/]/).pop() || "feed.xml",
        size_bytes: 0,
        format: selectedFilePath.endsWith(".json") ? "JSON" : "XML",
      };
      await feedState.startIngest("file", selectedFilePath, jobElement, ingestionMode);
    } else {
      if (!feedUrl.trim()) {
        feedState.showToast("Please enter a valid feed URL.");
        return;
      }
      const trimmedUrl = feedUrl.trim();
      const urlFilename = trimmedUrl.split("/").pop()?.split("?")[0] || "remote_feed.xml";
      feedState.currentFile = {
        path: trimmedUrl,
        filename: urlFilename,
        size_bytes: 0,
        format: trimmedUrl.includes(".json") ? "JSON" : "XML",
      };
      await feedState.startIngest("url", trimmedUrl, jobElement, ingestionMode);
    }
  }
</script>

<div class="h-full flex-1 overflow-y-auto px-6 py-8 select-none">
  <div class="max-w-6xl mx-auto space-y-8">
    <!-- Header Banner -->
    <div class="text-center space-y-2">
      <div class="inline-flex items-center gap-2.5 px-3 py-1 rounded-full bg-blue-950/60 border border-blue-800/40 text-blue-400 text-xs font-semibold">
        <span class="w-2 h-2 rounded-full bg-blue-400 animate-pulse"></span>
        <span>Ultra-High-Speed Feed Processing</span>
      </div>
      <h1 class="text-3xl font-extrabold text-white tracking-tight sm:text-4xl">
        XML & JSON Feed Analyzer
      </h1>
      <p class="text-sm text-slate-400 max-w-2xl mx-auto">
        Analyze extremely large XML & JSON feeds (100GB+) with constant memory footprints, streaming decompressors, and sub-150ms analytical queries.
      </p>
    </div>

    <!-- Main Two-Column Layout -->
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-stretch">
      <!-- Left Column: Choose Source -->
      <div class="lg:col-span-7 bg-slate-900/60 border border-slate-800 rounded-2xl p-6 shadow-xl backdrop-blur-xl flex flex-col justify-between">
        <div>
          <div class="flex items-center gap-2 mb-5 pb-3 border-b border-slate-800">
            <Layers class="w-5 h-5 text-blue-400" />
            <h2 class="text-base font-bold text-white">Choose Source</h2>
          </div>

          {#if feedState.lastError}
            <div class="mb-5 p-4 rounded-xl bg-red-950/50 border border-red-800/60 text-red-200 text-xs flex items-start justify-between gap-3 animate-in fade-in slide-in-from-top-2">
              <div class="flex items-start gap-2.5">
                <AlertCircle class="w-4 h-4 text-red-400 shrink-0 mt-0.5" />
                <div class="space-y-1">
                  <span class="font-bold text-red-300 block">Feed Ingestion Error</span>
                  <p class="text-red-200 leading-relaxed font-mono text-[11px] select-text">{feedState.lastError}</p>
                </div>
              </div>
              <button
                type="button"
                onclick={() => feedState.clearError()}
                class="text-red-400 hover:text-white p-1 rounded transition shrink-0 cursor-pointer"
                title="Dismiss"
              >
                <X class="w-4 h-4" />
              </button>
            </div>
          {/if}

          <form onsubmit={handleSubmit} class="space-y-5">
            <!-- Source Type Radio Toggle -->
            <div>
              <label class="block text-xs font-semibold text-slate-400 mb-2">Source Type</label>
              <div class="flex gap-4">
                <label class="flex items-center gap-2 cursor-pointer text-xs font-medium {sourceType === 'file' ? 'text-blue-400' : 'text-slate-400'}">
                  <input
                    type="radio"
                    name="source_type"
                    value="file"
                    checked={sourceType === "file"}
                    onchange={() => (sourceType = "file")}
                    class="accent-blue-500 cursor-pointer"
                  />
                  <span>Upload XML / JSON File</span>
                </label>

                <label class="flex items-center gap-2 cursor-pointer text-xs font-medium {sourceType === 'url' ? 'text-blue-400' : 'text-slate-400'}">
                  <input
                    type="radio"
                    name="source_type"
                    value="url"
                    checked={sourceType === "url"}
                    onchange={() => (sourceType = "url")}
                    class="accent-blue-500 cursor-pointer"
                  />
                  <span>Remote XML / JSON URL</span>
                </label>
              </div>
            </div>

            <!-- File Input Option -->
            {#if sourceType === "file"}
              <div class="space-y-1.5 animate-in fade-in duration-200">
                <label class="block text-xs font-semibold text-slate-400">Browse File</label>
                <div class="flex gap-2">
                  <input
                    type="text"
                    readonly
                    value={selectedFilePath}
                    placeholder="Click Browse to select XML, JSON, GZ, or ZIP feed..."
                    onclick={handleBrowse}
                    class="flex-1 bg-slate-950/80 border border-slate-800 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 placeholder:text-slate-600 focus:outline-none focus:border-blue-500 cursor-pointer truncate font-mono"
                  />
                  <button
                    type="button"
                    onclick={handleBrowse}
                    class="inline-flex items-center gap-1.5 px-4 py-2.5 rounded-xl bg-slate-800 hover:bg-slate-700 border border-slate-700 text-xs font-semibold text-slate-200 transition cursor-pointer shrink-0"
                  >
                    <FolderOpen class="w-4 h-4 text-blue-400" />
                    <span>Browse</span>
                  </button>
                </div>
                <p class="text-[11px] text-slate-500">Direct instant parsing for feeds of any size (100MB to 50GB+).</p>
              </div>
            {:else}
              <!-- URL Input Option -->
              <div class="space-y-1.5 animate-in fade-in duration-200">
                <label class="block text-xs font-semibold text-slate-400">Feed URL</label>
                <div class="relative">
                  <Globe class="w-4 h-4 text-slate-500 absolute left-3.5 top-3 pointer-events-none" />
                  <input
                    type="url"
                    bind:value={feedUrl}
                    placeholder="https://example.com/jobs.xml or https://example.com/feed.xml.gz"
                    class="w-full bg-slate-950/80 border border-slate-800 rounded-xl pl-10 pr-3.5 py-2.5 text-xs text-slate-200 placeholder:text-slate-600 focus:outline-none focus:border-blue-500 font-mono"
                  />
                </div>
                <p class="text-[11px] text-slate-500">Directly streams and inspects HTTP/HTTPS XML/JSON endpoints.</p>
              </div>
            {/if}

            <!-- Job Element Tag / Path -->
            <div class="space-y-1.5">
              <label class="block text-xs font-semibold text-slate-400">Job Element Tag / Path</label>
              <div class="relative">
                <Tag class="w-4 h-4 text-slate-500 absolute left-3.5 top-3 pointer-events-none" />
                <input
                  type="text"
                  bind:value={jobElement}
                  placeholder="e.g. job, vacancy, jobs.item"
                  class="w-full bg-slate-950/80 border border-slate-800 rounded-xl pl-10 pr-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-blue-500 font-mono"
                />
              </div>
              <p class="text-[11px] text-slate-500">Keep "Auto" to automatically discover repeating records.</p>
            </div>

            <!-- Ingestion Mode Preset Selector -->
            <div class="p-3.5 rounded-xl bg-slate-950/60 border border-slate-800 space-y-2">
              <div class="flex items-center gap-1.5 text-xs font-bold text-white">
                <Gauge class="w-4 h-4 text-blue-400" />
                <span>Ingestion Mode Preset</span>
              </div>
              <select
                bind:value={ingestionMode}
                class="w-full bg-slate-900 border border-slate-750 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-blue-500 cursor-pointer"
              >
                <option value="extreme">🚀 Extreme Fast Mode (~25,000+ rec/s - Skip Descriptions & HTML)</option>
                <option value="fast">⚡ Fast Mode (~12,000 rec/s - Skip Descriptions, Retain Raw Tags)</option>
                <option value="full">🔍 Full Inspection Mode (~5,000 rec/s - Retain All HTML Descriptions)</option>
              </select>
              <p class="text-[11px] text-slate-400 leading-normal">
                Extreme Fast Mode bypasses heavy HTML description CDATA blocks, speeding up ingestion by ~5x and reducing RAM usage.
              </p>
            </div>

            <!-- Submit Button -->
            <button
              type="submit"
              disabled={feedState.isIngesting || (sourceType === "file" && !selectedFilePath) || (sourceType === "url" && !feedUrl.trim())}
              class="w-full py-3.5 rounded-xl bg-blue-600 hover:bg-blue-500 disabled:opacity-40 disabled:hover:bg-blue-600 text-white font-bold text-sm shadow-lg shadow-blue-600/20 transition flex items-center justify-center gap-2 cursor-pointer"
            >
              <Play class="w-4 h-4 fill-current" />
              <span>Analyze Feed</span>
            </button>
          </form>
        </div>
      </div>

      <!-- Right Column: Recent Analyzed Feeds -->
      <div class="lg:col-span-5 bg-slate-900/60 border border-slate-800 rounded-2xl p-6 shadow-xl backdrop-blur-xl flex flex-col">
        <div class="flex items-center justify-between mb-5 pb-3 border-b border-slate-800">
          <div class="flex items-center gap-2">
            <Clock class="w-5 h-5 text-slate-400" />
            <h2 class="text-base font-bold text-white">Recent Analyzed Feeds</h2>
          </div>
          <span class="text-[11px] font-mono text-slate-400 px-2 py-0.5 rounded-full bg-slate-800">
            {feedState.recentFeeds.length}
          </span>
        </div>

        <div class="flex-1 overflow-y-auto space-y-3 pr-1 max-h-[460px]">
          {#if feedState.isLoadingRecent}
            <div class="h-48 flex items-center justify-center text-xs text-slate-500">
              Loading recent feeds...
            </div>
          {:else if feedState.recentFeeds.length > 0}
            {#each feedState.recentFeeds as feed}
              <div
                class="p-3.5 rounded-xl bg-slate-950/70 border border-slate-800 hover:border-slate-700 transition flex items-center justify-between gap-3 group"
              >
                <div class="min-w-0 flex-1">
                  <h3 class="text-xs font-bold text-slate-200 truncate group-hover:text-blue-400 transition" title={feed.filename}>
                    {feed.filename}
                  </h3>
                  <div class="flex flex-wrap items-center gap-1.5 mt-1.5 text-[10px] text-slate-400">
                    <span class="px-1.5 py-0.2 rounded bg-slate-800 font-mono text-slate-300 uppercase">
                      {feed.source_type}
                    </span>
                    <span class="text-slate-500">•</span>
                    <span class="font-mono text-blue-400 font-semibold">
                      {feed.total_records.toLocaleString()} records
                    </span>
                    <span class="text-slate-500">•</span>
                    <span>{feed.file_size}</span>
                  </div>
                  <div class="text-[9px] text-slate-500 mt-1">
                    {feed.analyzed_at}
                  </div>
                </div>

                <div class="flex items-center gap-1.5 shrink-0">
                  <button
                    onclick={() => feedState.loadRecentFeed(feed)}
                    class="p-2 rounded-lg bg-blue-950/60 hover:bg-blue-900 border border-blue-800/40 text-blue-400 transition cursor-pointer"
                    title="Load this feed into memory instantly"
                  >
                    <ArrowRight class="w-4 h-4" />
                  </button>
                  <button
                    onclick={(e) => {
                      e.stopPropagation();
                      feedState.deleteRecentFeed(feed.id);
                    }}
                    class="p-2 rounded-lg bg-rose-950/40 hover:bg-rose-900/60 border border-rose-800/40 text-rose-400 transition cursor-pointer"
                    title="Delete feed and cached database"
                  >
                    <Trash2 class="w-4 h-4" />
                  </button>
                </div>
              </div>
            {/each}
          {:else}
            <!-- Empty State -->
            <div class="h-64 flex flex-col items-center justify-center text-center p-6 text-slate-500">
              <Database class="w-10 h-10 mb-3 opacity-30 text-slate-400" />
              <p class="text-xs font-semibold text-slate-300">No feeds analyzed yet</p>
              <p class="text-[11px] text-slate-500 mt-1 max-w-xs">
                Select a local file or enter a remote URL on the left to start analyzing.
              </p>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>
