<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { FileUp, Sparkles, Database, Cpu, Zap, Archive, HardDrive } from "@lucide/svelte";
</script>

<div class="h-full flex flex-col items-center justify-center p-8 text-center select-none overflow-y-auto">
  <!-- Glowing Center Card -->
  <div class="max-w-2xl w-full bg-slate-900/60 border border-slate-800 rounded-2xl p-8 shadow-2xl relative overflow-hidden backdrop-blur-xl">
    <!-- Subtle glow background -->
    <div class="absolute -top-24 -left-24 w-72 h-72 bg-cyan-500/10 rounded-full blur-3xl pointer-events-none"></div>
    <div class="absolute -bottom-24 -right-24 w-72 h-72 bg-blue-500/10 rounded-full blur-3xl pointer-events-none"></div>

    <!-- Center Icon -->
    <div class="w-16 h-16 rounded-2xl bg-gradient-to-tr from-cyan-500 to-blue-600 flex items-center justify-center mx-auto mb-6 shadow-xl shadow-cyan-500/25 text-3xl">
      ⚡
    </div>

    <h1 class="text-2xl font-extrabold text-white tracking-tight mb-2">
      FeedLens Analytics Engine
    </h1>
    <p class="text-sm text-slate-400 max-w-lg mx-auto mb-8 leading-relaxed">
      Analyze 100 GB+ XML and JSON feeds with instant start, zero disk pre-extraction, and sub-150ms analytical queries powered by Rust and DuckDB OLAP.
    </p>

    <!-- Main Action Buttons -->
    <div class="flex flex-col sm:flex-row items-center justify-center gap-3 mb-8">
      <button
        onclick={() => feedState.openFile()}
        class="w-full sm:w-auto flex items-center justify-center gap-2 px-6 py-3 rounded-xl bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-bold text-sm shadow-lg shadow-cyan-500/30 transition transform hover:-translate-y-0.5 cursor-pointer"
      >
        <FileUp class="w-4 h-4" />
        <span>Select Feed File</span>
      </button>

      <button
        onclick={() => feedState.loadDemo(25000)}
        class="w-full sm:w-auto flex items-center justify-center gap-2 px-5 py-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-cyan-300 border border-slate-700 hover:border-slate-600 font-semibold text-sm transition cursor-pointer"
      >
        <Sparkles class="w-4 h-4 text-cyan-400" />
        <span>Try 25k Demo Feed</span>
      </button>

      <button
        onclick={() => feedState.loadDemo(100000)}
        class="w-full sm:w-auto flex items-center justify-center gap-2 px-5 py-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 hover:border-slate-600 font-medium text-sm transition cursor-pointer"
      >
        <Sparkles class="w-4 h-4 text-amber-400" />
        <span>Try 100k Benchmark</span>
      </button>
    </div>

    <!-- Supported Archives and Formats Pills -->
    <div class="border-t border-slate-800/80 pt-6">
      <div class="text-[11px] font-semibold text-slate-500 uppercase tracking-wider mb-3">
        Native Zero-Extraction Streaming Codecs
      </div>
      <div class="flex flex-wrap items-center justify-center gap-2">
        <span class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-mono bg-slate-850 border border-slate-750 text-slate-300">
          <Archive class="w-3 h-3 text-cyan-400" /> .xml / .xml.gz
        </span>
        <span class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-mono bg-slate-850 border border-slate-750 text-slate-300">
          <Archive class="w-3 h-3 text-blue-400" /> .json / .json.gz
        </span>
        <span class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-mono bg-slate-850 border border-slate-750 text-slate-300">
          <Archive class="w-3 h-3 text-amber-400" /> .zip / .bz2
        </span>
        <span class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-mono bg-slate-850 border border-slate-750 text-slate-300">
          <Archive class="w-3 h-3 text-emerald-400" /> .tar.gz / .ndjson
        </span>
      </div>
    </div>
  </div>

  <!-- Tech Highlights Grid -->
  <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 max-w-2xl w-full mt-6 text-left">
    <div class="p-3.5 rounded-xl bg-slate-900/40 border border-slate-800/60 flex items-start gap-3">
      <div class="p-2 rounded-lg bg-cyan-950/60 border border-cyan-800/40 text-cyan-400">
        <Database class="w-4 h-4" />
      </div>
      <div>
        <h4 class="text-xs font-bold text-slate-200">DuckDB Columnar OLAP</h4>
        <p class="text-[11px] text-slate-400 mt-0.5">Vectorized execution across CPU cores with out-of-core memory safety.</p>
      </div>
    </div>

    <div class="p-3.5 rounded-xl bg-slate-900/40 border border-slate-800/60 flex items-start gap-3">
      <div class="p-2 rounded-lg bg-blue-950/60 border border-blue-800/40 text-blue-400">
        <Cpu class="w-4 h-4" />
      </div>
      <div>
        <h4 class="text-xs font-bold text-slate-200">Zero-Copy Parser</h4>
        <p class="text-[11px] text-slate-400 mt-0.5">SIMD-accelerated streaming XML and JSON tokenizers in Rust.</p>
      </div>
    </div>

    <div class="p-3.5 rounded-xl bg-slate-900/40 border border-slate-800/60 flex items-start gap-3">
      <div class="p-2 rounded-lg bg-amber-950/60 border border-amber-800/40 text-amber-400">
        <HardDrive class="w-4 h-4" />
      </div>
      <div>
        <h4 class="text-xs font-bold text-slate-200">Tiny Footprint</h4>
        <p class="text-[11px] text-slate-400 mt-0.5">Tauri 2.0 shell running on ~35MB RAM with sub-200ms cold startup.</p>
      </div>
    </div>
  </div>
</div>
