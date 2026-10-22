<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import Header from "$lib/components/Header.svelte";
  import ProgressBanner from "$lib/components/ProgressBanner.svelte";
  import DropZone from "$lib/components/DropZone.svelte";
  import OverviewTab from "$lib/components/OverviewTab.svelte";
  import CompletenessMatrixTab from "$lib/components/CompletenessMatrixTab.svelte";
  import DataExplorerTab from "$lib/components/DataExplorerTab.svelte";
  import DeepDiveTab from "$lib/components/DeepDiveTab.svelte";
  import RecordInspectorModal from "$lib/components/RecordInspectorModal.svelte";

  import { LayoutDashboard, TableProperties, Database, BarChart3 } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { SelectedFileInfo } from "$lib/types";

  onMount(async () => {
    try {
      const cliFile = await invoke<SelectedFileInfo | null>("get_cli_file");
      if (cliFile && cliFile.path) {
        feedState.currentFile = cliFile;
      }
    } catch (e) {
      console.warn("Could not check CLI file:", e);
    }
  });
</script>

<div class="h-screen w-screen flex flex-col bg-slate-950 text-slate-100 overflow-hidden font-sans select-none relative">
  <!-- Ambient light gradients that softly glow through frosted glass and cards -->
  <div class="absolute -top-32 left-1/4 w-[500px] h-[350px] bg-blue-600/12 rounded-full blur-[100px] pointer-events-none z-0"></div>
  <div class="absolute top-1/3 -right-20 w-[450px] h-[400px] bg-indigo-600/10 rounded-full blur-[110px] pointer-events-none z-0"></div>
  <div class="absolute -bottom-20 left-1/3 w-[500px] h-[350px] bg-cyan-600/8 rounded-full blur-[100px] pointer-events-none z-0"></div>

  <!-- Top App Header -->
  <Header />

  <!-- Live Streaming Progress Bar -->
  <ProgressBanner />

  <!-- Main View Area -->
  <main class="flex-1 flex flex-col overflow-hidden relative">
    {#if !feedState.hasActiveDataset && !feedState.isIngesting}
      <!-- Empty State / Landing Dropzone -->
      <DropZone />
    {:else}
      <!-- FROSTED GLASS TAB BAR (Sticky Floating Chrome) -->
      <div class="glass-chrome h-12 px-6 flex items-center justify-between z-30 shrink-0 select-none">
        <div class="flex items-center gap-2 h-full">
          <!-- Overview Tab -->
          <button
            onclick={() => (feedState.activeTab = "overview")}
            class="h-9 px-4 rounded-xl flex items-center gap-2 text-xs font-bold transition cursor-pointer {feedState.activeTab === 'overview'
              ? 'tab-active'
              : 'text-slate-400 hover:text-white hover:bg-white/5'}"
          >
            <LayoutDashboard class="w-3.5 h-3.5 text-blue-400" />
            <span>Overview & Health</span>
          </button>

          <!-- Completeness Matrix Tab -->
          <button
            onclick={() => (feedState.activeTab = "completeness")}
            class="h-9 px-4 rounded-xl flex items-center gap-2 text-xs font-bold transition cursor-pointer {feedState.activeTab === 'completeness'
              ? 'tab-active'
              : 'text-slate-400 hover:text-white hover:bg-white/5'}"
          >
            <TableProperties class="w-3.5 h-3.5 text-blue-400" />
            <span>Completeness Matrix</span>
            <span class="tabular-tag px-1.5 py-0.2 rounded-full text-[10px] bg-slate-800 text-slate-300 border border-slate-700">
              {feedState.completeness.length}
            </span>
          </button>

          <!-- Data Explorer Tab -->
          <button
            onclick={() => (feedState.activeTab = "explorer")}
            class="h-9 px-4 rounded-xl flex items-center gap-2 text-xs font-bold transition cursor-pointer {feedState.activeTab === 'explorer'
              ? 'tab-active'
              : 'text-slate-400 hover:text-white hover:bg-white/5'}"
          >
            <Database class="w-3.5 h-3.5 text-blue-400" />
            <span>Data Explorer</span>
          </button>

          <!-- Deep Dive Tab -->
          <button
            onclick={() => (feedState.activeTab = "deepdive")}
            class="h-9 px-4 rounded-xl flex items-center gap-2 text-xs font-bold transition cursor-pointer {feedState.activeTab === 'deepdive'
              ? 'tab-active'
              : 'text-slate-400 hover:text-white hover:bg-white/5'}"
          >
            <BarChart3 class="w-3.5 h-3.5 text-blue-400" />
            <span>Deep Dive & Distribution</span>
          </button>
        </div>

        {#if feedState.stats}
          <div class="text-xs text-slate-400 tabular-tag flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-cyan-400 animate-pulse"></span>
            <span>{feedState.stats.total_records.toLocaleString()} rows loaded</span>
          </div>
        {/if}
      </div>

      <!-- Tab Content Area -->
      <div class="flex-1 overflow-hidden relative">
        {#if feedState.activeTab === "overview"}
          <OverviewTab />
        {:else if feedState.activeTab === "completeness"}
          <CompletenessMatrixTab />
        {:else if feedState.activeTab === "explorer"}
          <DataExplorerTab />
        {:else if feedState.activeTab === "deepdive"}
          <DeepDiveTab />
        {/if}
      </div>
    {/if}
  </main>

  <!-- Record Inspector Modal -->
  <RecordInspectorModal />

  <!-- Toast Notification Overlay -->
  {#if feedState.toastMessage}
    <div class="fixed bottom-5 right-5 z-50 px-4 py-2.5 rounded-xl bg-slate-900 border border-blue-500/40 text-xs font-medium text-slate-200 shadow-2xl flex items-center gap-2 animate-in fade-in slide-in-from-bottom-2">
      <span class="w-2 h-2 rounded-full bg-blue-400 animate-ping"></span>
      <span>{feedState.toastMessage}</span>
    </div>
  {/if}
</div>
