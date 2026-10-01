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
</script>

<div class="h-screen w-screen flex flex-col bg-slate-950 text-slate-100 overflow-hidden font-sans select-none">
  <!-- Top App Header -->
  <Header />

  <!-- Live Streaming Progress Bar -->
  <ProgressBanner />

  <!-- Main View Area -->
  <main class="flex-1 flex flex-col overflow-hidden relative">
    {#if !feedState.stats && !feedState.isIngesting && feedState.completeness.length === 0}
      <!-- Empty State / Landing Dropzone -->
      <DropZone />
    {:else}
      <!-- Tab Navigation Bar -->
      <div class="h-11 bg-slate-900 border-b border-slate-800 px-6 flex items-center justify-between shrink-0">
        <div class="flex items-center gap-1 h-full">
          <!-- Overview Tab -->
          <button
            onclick={() => (feedState.activeTab = "overview")}
            class="h-full px-4 flex items-center gap-2 text-xs font-semibold border-b-2 transition cursor-pointer {feedState.activeTab === 'overview'
              ? 'border-cyan-400 text-cyan-400 bg-slate-800/40'
              : 'border-transparent text-slate-400 hover:text-slate-200 hover:bg-slate-800/20'}"
          >
            <LayoutDashboard class="w-3.5 h-3.5" />
            <span>Overview & Health</span>
          </button>

          <!-- Completeness Matrix Tab -->
          <button
            onclick={() => (feedState.activeTab = "completeness")}
            class="h-full px-4 flex items-center gap-2 text-xs font-semibold border-b-2 transition cursor-pointer {feedState.activeTab === 'completeness'
              ? 'border-cyan-400 text-cyan-400 bg-slate-800/40'
              : 'border-transparent text-slate-400 hover:text-slate-200 hover:bg-slate-800/20'}"
          >
            <TableProperties class="w-3.5 h-3.5" />
            <span>Completeness Matrix</span>
            <span class="px-1.5 py-0.2 rounded-full text-[10px] font-mono bg-slate-800 text-slate-400 border border-slate-700">
              {feedState.completeness.length}
            </span>
          </button>

          <!-- Data Explorer Tab -->
          <button
            onclick={() => (feedState.activeTab = "explorer")}
            class="h-full px-4 flex items-center gap-2 text-xs font-semibold border-b-2 transition cursor-pointer {feedState.activeTab === 'explorer'
              ? 'border-cyan-400 text-cyan-400 bg-slate-800/40'
              : 'border-transparent text-slate-400 hover:text-slate-200 hover:bg-slate-800/20'}"
          >
            <Database class="w-3.5 h-3.5" />
            <span>Data Explorer</span>
          </button>

          <!-- Deep Dive Tab -->
          <button
            onclick={() => (feedState.activeTab = "deepdive")}
            class="h-full px-4 flex items-center gap-2 text-xs font-semibold border-b-2 transition cursor-pointer {feedState.activeTab === 'deepdive'
              ? 'border-cyan-400 text-cyan-400 bg-slate-800/40'
              : 'border-transparent text-slate-400 hover:text-slate-200 hover:bg-slate-800/20'}"
          >
            <BarChart3 class="w-3.5 h-3.5" />
            <span>Deep Dive & Distribution</span>
          </button>
        </div>
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
    <div class="fixed bottom-5 right-5 z-50 px-4 py-2.5 rounded-xl bg-slate-900 border border-cyan-500/40 text-xs font-medium text-slate-200 shadow-2xl flex items-center gap-2 animate-in fade-in slide-in-from-bottom-2">
      <span class="w-2 h-2 rounded-full bg-cyan-400 animate-ping"></span>
      <span>{feedState.toastMessage}</span>
    </div>
  {/if}
</div>
