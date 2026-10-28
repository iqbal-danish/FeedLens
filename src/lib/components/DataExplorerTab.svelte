<script lang="ts">
  import { feedState } from "$lib/state.svelte";
  import { onMount } from "svelte";
  import {
    Search,
    ArrowUpDown,
    ChevronLeft,
    ChevronRight,
    Eye,
    RefreshCw,
    X,
    Copy,
    Check,
    RotateCcw,
    Sparkles,
    Building2,
    ChevronDown,
    Download,
    Bookmark,
    HelpCircle,
    Plus,
    Trash2,
    SlidersHorizontal,
  } from "@lucide/svelte";

  let searchInput = $state(feedState.searchFilter);
  $effect(() => {
    searchInput = feedState.searchFilter;
  });
  let fieldFilterQuery = $state("");
  let copiedKey = $state<string | null>(null);

  // Column prioritization: Populated first is always turned on by default
  let orderColumnsByFill = $state(true);

  // Custom Column Extraction Modal State
  let showExportColumnsModal = $state(false);
  let selectedExportCols = $state<string[]>([]);
  let deduplicateExport = $state(true);
  let includeCountsExport = $state(false);
  let exportColSearchQuery = $state("");
  let applyFilterToExport = $state(true);

  // Auto-initialize selectedExportCols with smart company candidate if empty
  $effect(() => {
    if (selectedExportCols.length === 0 && feedState.tableData?.columns) {
      const cols = feedState.tableData.columns;
      const match = cols.find((c) => {
        const low = c.toLowerCase();
        return (
          low === "source.name" ||
          low === "source_name" ||
          low === "company" ||
          low === "name" ||
          low === "brand_name"
        );
      });
      selectedExportCols = match ? [match] : (cols[0] ? [cols[0]] : []);
    }
  });

  // Suggested company-related columns in this feed
  let suggestedCompanyCols = $derived(() => {
    if (!feedState.tableData?.columns) return [];
    return feedState.tableData.columns.filter((c) => {
      const low = c.toLowerCase();
      return (
        low.includes("name") ||
        low.includes("company") ||
        low.includes("brand") ||
        low.includes("employer") ||
        low.includes("source")
      );
    }).slice(0, 6);
  });

  // Filtered column list inside the selection modal
  let modalFilteredCols = $derived(() => {
    if (!feedState.tableData?.columns) return [];
    const q = exportColSearchQuery.trim().toLowerCase();
    if (!q) return feedState.tableData.columns;
    return feedState.tableData.columns.filter((c) => c.toLowerCase().includes(q));
  });

  function toggleExportCol(col: string) {
    if (selectedExportCols.includes(col)) {
      selectedExportCols = selectedExportCols.filter((c) => c !== col);
    } else {
      selectedExportCols = [...selectedExportCols, col];
    }
  }

  function selectAllExportCols() {
    if (feedState.tableData?.columns) {
      selectedExportCols = [...feedState.tableData.columns];
    }
  }

  function clearAllExportCols() {
    selectedExportCols = [];
  }

  function triggerExportColumns() {
    if (selectedExportCols.length === 0) return;
    feedState.exportCustomColumns(
      selectedExportCols,
      deduplicateExport,
      includeCountsExport,
      applyFilterToExport
    );
    showExportColumnsModal = false;
  }

  // Autocomplete Suggestions State
  let searchInputRef = $state<HTMLInputElement>();
  let isSearchFocused = $state(false);
  let searchContainerRef = $state<HTMLDivElement>();
  let selectedSuggestionIndex = $state(0);

  interface SearchSuggestion {
    type: "column" | "value" | "operator";
    insertText: string;
    label: string;
    badge: string;
  }

  // Derive the active word currently being typed
  let activeTokenInfo = $derived(() => {
    const val = searchInput || "";
    const lastSpaceIdx = Math.max(val.lastIndexOf(" "), val.lastIndexOf("("));
    const prefix = lastSpaceIdx >= 0 ? val.slice(0, lastSpaceIdx + 1) : "";
    const word = lastSpaceIdx >= 0 ? val.slice(lastSpaceIdx + 1) : val;

    if (word.includes(":")) {
      const colonIdx = word.indexOf(":");
      const rawCol = word.slice(0, colonIdx);
      const rawVal = word.slice(colonIdx + 1);
      return { prefix, word, col: rawCol.trim().toLowerCase(), valPrefix: rawVal || "" };
    }

    return { prefix, word, col: null, valPrefix: "" };
  });

  // Generate live autocomplete suggestions based on what the user is typing
  let searchSuggestions = $derived((): SearchSuggestion[] => {
    const { word, col, valPrefix } = activeTokenInfo();
    const suggestions: SearchSuggestion[] = [];

    // Case 1: User typed a column prefix with colon, e.g. "remote:" or "city:New"
    if (col !== null) {
      const colLow = col.toLowerCase();
      // Find matching column info in completeness
      const matchedCol = feedState.completeness.find(
        (c) => c.column_name.toLowerCase() === colLow || c.original_name.toLowerCase() === colLow
      );

      // Boolean shortcut
      if (colLow === "remote" || colLow.includes("remote") || colLow.includes("is_")) {
        ["true", "false"].forEach((b) => {
          if (!valPrefix || b.startsWith(valPrefix.toLowerCase())) {
            suggestions.push({
              type: "value",
              insertText: `${col}:${b}`,
              label: `${col}:${b}`,
              badge: "Value",
            });
          }
        });
      }

      // Salary / numeric shortcut
      if (colLow.includes("salary") || colLow.includes("pay") || colLow.includes("wage") || colLow.includes("rate")) {
        [">50k", ">80k", ">100k", "50k..100k", "80k..150k"].forEach((s) => {
          if (!valPrefix || s.startsWith(valPrefix.toLowerCase())) {
            suggestions.push({
              type: "value",
              insertText: `${col}:${s}`,
              label: `${col}:${s}`,
              badge: "Range",
            });
          }
        });
      }

      // Sample values from the column
      if (matchedCol && matchedCol.sample_values) {
        for (const sample of matchedCol.sample_values) {
          const s = sample.trim();
          if (!s) continue;
          if (!valPrefix || s.toLowerCase().includes(valPrefix.toLowerCase())) {
            const formattedVal = s.includes(" ") ? `"${s}"` : s;
            suggestions.push({
              type: "value",
              insertText: `${col}:${formattedVal}`,
              label: `${col}:${formattedVal}`,
              badge: "Top Value",
            });
            if (suggestions.length >= 8) break;
          }
        }
      }

      // Missing / Populated shortcuts for this col
      if (!valPrefix) {
        suggestions.push({
          type: "operator",
          insertText: `missing:${col}`,
          label: `missing:${col}`,
          badge: "Empty",
        });
        suggestions.push({
          type: "operator",
          insertText: `populated:${col}`,
          label: `populated:${col}`,
          badge: "Valid",
        });
      }

      return suggestions.slice(0, 8);
    }

    // Case 2: User is typing a column name or just started typing (e.g. "ci", "rem", "sal", or empty)
    const q = word.toLowerCase().trim();

    // High-priority common columns
    const priorityCols = ["source_name", "locations_location_city", "remote", "work_model", "salary", "title"];

    // Suggest matching columns
    const matchedCols = feedState.completeness.filter((c) => {
      if (!q) return priorityCols.some((p) => c.column_name.toLowerCase().includes(p));
      return (
        c.column_name.toLowerCase().includes(q) ||
        c.original_name.toLowerCase().includes(q)
      );
    });

    for (const c of matchedCols.slice(0, 6)) {
      suggestions.push({
        type: "column",
        insertText: `${c.original_name || c.column_name}:`,
        label: `${c.original_name || c.column_name}:`,
        badge: `${c.fill_rate.toFixed(0)}% fill`,
      });
    }

    // Suggest operators if matching
    if (q) {
      if ("and".startsWith(q)) {
        suggestions.push({ type: "operator", insertText: "AND ", label: "AND", badge: "Operator" });
      }
      if ("or".startsWith(q)) {
        suggestions.push({ type: "operator", insertText: "OR ", label: "OR", badge: "Operator" });
      }
      if ("not".startsWith(q)) {
        suggestions.push({ type: "operator", insertText: "NOT ", label: "NOT", badge: "Operator" });
      }
      if ("missing:".startsWith(q)) {
        suggestions.push({ type: "operator", insertText: "missing:", label: "missing:", badge: "Filter" });
      }
      if ("populated:".startsWith(q)) {
        suggestions.push({ type: "operator", insertText: "populated:", label: "populated:", badge: "Filter" });
      }
    }

    return suggestions.slice(0, 8);
  });

  function selectSuggestion(s: SearchSuggestion) {
    const { prefix } = activeTokenInfo();
    searchInput = prefix + s.insertText + (s.type === "value" ? " " : "");
    setTimeout(() => {
      searchInputRef?.focus();
    }, 10);
  }

  function handleSearchKeydown(e: KeyboardEvent) {
    const suggestions = searchSuggestions();
    if (isSearchFocused && suggestions.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        selectedSuggestionIndex = (selectedSuggestionIndex + 1) % suggestions.length;
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        selectedSuggestionIndex = (selectedSuggestionIndex - 1 + suggestions.length) % suggestions.length;
        return;
      }
      if (e.key === "Tab") {
        e.preventDefault();
        selectSuggestion(suggestions[selectedSuggestionIndex] || suggestions[0]);
        return;
      }
    }

    if (e.key === "Enter") {
      isSearchFocused = false;
      feedState.applySearch(searchInput);
    } else if (e.key === "Escape") {
      isSearchFocused = false;
    }
  }

  function handleSearchClear() {
    searchInput = "";
    feedState.applySearch("");
  }

  // Advanced Query Syntax Guide State
  let showSyntaxGuide = $state(false);
  let syntaxGuideRef = $state<HTMLDivElement>();

  onMount(() => {
    const handleGlobalClick = (e: MouseEvent) => {
      if (showSyntaxGuide && syntaxGuideRef && !syntaxGuideRef.contains(e.target as Node)) {
        showSyntaxGuide = false;
      }
      if (searchContainerRef && !searchContainerRef.contains(e.target as Node)) {
        isSearchFocused = false;
      }
    };
    window.addEventListener("click", handleGlobalClick);
    return () => {
      window.removeEventListener("click", handleGlobalClick);
    };
  });

  function applySyntaxExample(query: string) {
    searchInput = query;
    feedState.applySearch(query);
    showSyntaxGuide = false;
  }

  // Lookup map of column fill rates from DuckDB completeness analysis
  let columnFillRates = $derived(() => {
    const map = new Map<string, number>();
    for (const c of feedState.completeness) {
      map.set(c.column_name.toLowerCase(), c.fill_rate);
      map.set(c.original_name.toLowerCase(), c.fill_rate);
    }
    return map;
  });

  // Count of columns that are 100% empty (fill_rate < 0.1%)
  let emptyColumnsCount = $derived(() => {
    if (!feedState.tableData) return 0;
    const rates = columnFillRates();
    return feedState.tableData.columns.filter((col) => {
      const rate = rates.get(col.toLowerCase());
      return rate !== undefined && rate < 0.1;
    }).length;
  });

  // Derived target column if missing: or populated: or col:val filter is active
  let filterTargetCol = $derived(() => {
    if (!feedState.searchFilter) return null;
    const trimmed = feedState.searchFilter.trim();
    if (trimmed.startsWith("missing:")) {
      return trimmed.slice("missing:".length).trim().toLowerCase();
    }
    if (trimmed.startsWith("populated:")) {
      return trimmed.slice("populated:".length).trim().toLowerCase();
    }
    const colonIdx = trimmed.indexOf(":");
    if (colonIdx > 0) {
      const target = trimmed.slice(0, colonIdx).trim().toLowerCase().replace(/^[-!]/, "");
      if ((target === "company" || target === "employer") && !feedState.tableData?.columns.some(c => c.toLowerCase() === target)) {
        const match = feedState.tableData?.columns.find(c => c.toLowerCase() === "source.name" || c.toLowerCase() === "source_name" || c.toLowerCase() === "name");
        if (match) return match.toLowerCase();
      }
      return target;
    }
    return null;
  });

  // Filtered & ordered visible columns list (populated first, with pinned filter target)
  let visibleColumns = $derived(() => {
    if (!feedState.tableData) return [];
    let cols = [...feedState.tableData.columns];
    const rates = columnFillRates();

    // Sort by fill rate descending so 100% full columns appear on the left
    if (orderColumnsByFill) {
      cols.sort((a, b) => {
        const rateA = rates.get(a.toLowerCase()) ?? 50;
        const rateB = rates.get(b.toLowerCase()) ?? 50;
        return rateB - rateA;
      });
    }

    // Pin target filtered column to the front so the user sees it immediately
    const target = filterTargetCol();
    if (target) {
      const idx = cols.findIndex(
        (c) => c.toLowerCase() === target || c.toLowerCase().replace(/[^a-z0-9_]/g, "_") === target
      );
      if (idx > -1) {
        const [pinned] = cols.splice(idx, 1);
        cols.unshift(pinned);
      }
    }

    return cols;
  });

  // Row-level health calculation for the currently selected record
  let selectedRowHealth = $derived(() => {
    if (!feedState.inspectingRecord) return { rate: 0, populated: 0, total: 0 };
    const keys = Object.keys(feedState.inspectingRecord);
    const populated = keys.filter((k) => {
      const val = feedState.inspectingRecord![k];
      return val !== null && val !== undefined && String(val).trim() !== "";
    }).length;
    const rate = keys.length > 0 ? (populated / keys.length) * 100 : 0;
    return { rate, populated, total: keys.length };
  });

  // Filtered fields inside the sidecar inspector drawer
  let filteredInspectorFields = $derived(() => {
    if (!feedState.inspectingRecord) return [];
    const entries = Object.entries(feedState.inspectingRecord);
    if (!fieldFilterQuery.trim()) return entries;
    const q = fieldFilterQuery.toLowerCase();
    return entries.filter(([k, v]) =>
      k.toLowerCase().includes(q) || (v && String(v).toLowerCase().includes(q))
    );
  });

  // Currently selected row index
  let currentRowIndex = $derived(() => {
    if (!feedState.tableData || !feedState.inspectingRecord) return -1;
    return feedState.tableData.rows.indexOf(feedState.inspectingRecord);
  });

  function selectPreviousRow() {
    if (!feedState.tableData || !feedState.inspectingRecord) return;
    const idx = currentRowIndex();
    if (idx > 0) {
      feedState.inspectingRecord = feedState.tableData.rows[idx - 1];
    }
  }

  function selectNextRow() {
    if (!feedState.tableData || !feedState.inspectingRecord) return;
    const idx = currentRowIndex();
    if (idx >= 0 && idx < feedState.tableData.rows.length - 1) {
      feedState.inspectingRecord = feedState.tableData.rows[idx + 1];
    }
  }

  function copyValue(key: string, val: string) {
    navigator.clipboard.writeText(val);
    copiedKey = key;
    setTimeout(() => {
      if (copiedKey === key) copiedKey = null;
    }, 2000);
  }

  function copyRecordJson() {
    if (!feedState.inspectingRecord) return;
    navigator.clipboard.writeText(JSON.stringify(feedState.inspectingRecord, null, 2));
    feedState.showToast("Copied full record JSON to clipboard!");
  }

  function closeInspector() {
    feedState.inspectingRecord = null;
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && feedState.inspectingRecord) {
      closeInspector();
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="p-6 h-full flex flex-col space-y-4 w-full font-sans select-none">
  <!-- Controls Bar -->
  <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-3 shrink-0">
    
    <!-- Left Controls: Search Box + Clear Filter Button + Space Saver Toggles -->
    <div class="flex items-center gap-2.5 flex-wrap flex-1">
      <!-- Search Input -->
      <div class="relative w-72 sm:w-88" bind:this={searchContainerRef}>
        <Search class="w-3.5 h-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none z-10" />
        <input
          bind:this={searchInputRef}
          type="text"
          placeholder="Search records... (Press Enter)"
          bind:value={searchInput}
          onfocus={() => {
            isSearchFocused = true;
            selectedSuggestionIndex = 0;
          }}
          onkeydown={handleSearchKeydown}
          class="w-full bg-slate-900/90 border border-slate-800 rounded-xl pl-9 pr-14 py-2 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 transition shadow-sm font-sans"
        />
        <span class="absolute right-2 top-1/2 -translate-y-1/2 text-[9px] font-mono text-slate-500 bg-slate-800 px-1.5 py-0.5 rounded border border-slate-700 pointer-events-none">
          ↵
        </span>

        <!-- Autocomplete Suggestions Dropdown -->
        {#if isSearchFocused && searchSuggestions().length > 0}
          <div
            class="absolute top-full left-0 mt-1.5 w-full bg-slate-900/98 border border-slate-700/80 rounded-xl shadow-2xl backdrop-blur-2xl z-50 p-1.5 space-y-0.5 animate-in fade-in duration-100"
            style="box-shadow: 0 15px 35px -5px rgba(0,0,0,0.8), 0 0 0 1px rgba(255,255,255,0.06);"
          >
            <div class="px-2 py-1 text-[10px] font-bold uppercase tracking-wider text-slate-500 flex items-center justify-between">
              <span>Suggestions</span>
              <span class="font-mono text-[9px] text-slate-500">Tab to insert</span>
            </div>
            {#each searchSuggestions() as sug, idx}
              <button
                type="button"
                onmousedown={(e) => {
                  e.preventDefault();
                  selectSuggestion(sug);
                }}
                class="w-full text-left flex items-center justify-between px-2.5 py-1.5 rounded-lg text-xs font-mono transition cursor-pointer {idx === selectedSuggestionIndex ? 'bg-blue-600/25 border border-blue-500/50 text-white' : 'text-slate-300 hover:bg-slate-800/80 hover:text-white'}"
              >
                <div class="flex items-center gap-2 truncate">
                  {#if sug.type === 'column'}
                    <span class="w-1.5 h-1.5 rounded-full bg-blue-400 shrink-0"></span>
                  {:else if sug.type === 'value'}
                    <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 shrink-0"></span>
                  {:else}
                    <span class="w-1.5 h-1.5 rounded-full bg-amber-400 shrink-0"></span>
                  {/if}
                  <span class="truncate font-medium">{sug.label}</span>
                </div>
                <span class="text-[10px] font-sans px-1.5 py-0.5 rounded bg-slate-950/60 text-slate-400 border border-slate-800 shrink-0">
                  {sug.badge}
                </span>
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Filter Button -->
      <button
        onclick={() => feedState.applySearch(searchInput)}
        class="px-3.5 py-2 rounded-xl text-xs font-bold text-white bg-blue-600 hover:bg-blue-500 transition cursor-pointer shadow-md shadow-blue-600/20 shrink-0"
      >
        Filter
      </button>

      <!-- Advanced Syntax Guide Button & Popover -->
      <div class="relative" bind:this={syntaxGuideRef}>
        <button
          type="button"
          onclick={() => (showSyntaxGuide = !showSyntaxGuide)}
          class="px-2.5 py-2 rounded-xl text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer border {showSyntaxGuide ? 'bg-cyan-950/70 border-cyan-500/70 text-cyan-200' : 'bg-slate-900 border-slate-800 text-slate-400 hover:text-slate-200 hover:bg-slate-850'}"
          title="Advanced query syntax guide (AND, OR, NOT, ranges, dates)"
        >
          <Sparkles class="w-3.5 h-3.5 text-cyan-400" />
          <span>Syntax</span>
        </button>

        {#if showSyntaxGuide}
          <div
            class="absolute left-0 top-full mt-2 w-[440px] max-w-[95vw] bg-slate-900/98 border border-slate-700/80 rounded-2xl shadow-2xl backdrop-blur-2xl z-50 p-4 text-xs font-sans animate-in fade-in duration-150 text-slate-200"
            style="box-shadow: 0 20px 50px -10px rgba(0,0,0,0.8), 0 0 0 1px rgba(255,255,255,0.06);"
          >
            <div class="flex items-center justify-between pb-2.5 mb-3 border-b border-slate-800">
              <div class="flex items-center gap-2">
                <Sparkles class="w-4 h-4 text-cyan-400" />
                <h4 class="font-bold text-white text-sm">Advanced Search Syntax</h4>
              </div>
              <button onclick={() => (showSyntaxGuide = false)} class="p-1 hover:text-white text-slate-400 cursor-pointer">
                <X class="w-4 h-4" />
              </button>
            </div>

            <div class="space-y-3 max-h-[380px] overflow-y-auto pr-1">
              <!-- Boolean Logic -->
              <div class="p-2.5 rounded-xl bg-slate-950/70 border border-slate-800/80">
                <span class="block text-[11px] font-bold uppercase tracking-wider text-cyan-400 mb-1">
                  Boolean AND / OR / NOT &amp; Grouping
                </span>
                <div class="space-y-1.5 font-mono text-[11px]">
                  <div class="flex items-center justify-between group">
                    <code class="text-slate-300">(city:Austin OR city:Dallas) remote:true</code>
                    <button onclick={() => applySyntaxExample('(city:Austin OR city:Dallas) remote:true')} class="text-[10px] text-cyan-400 hover:underline cursor-pointer">Try</button>
                  </div>
                  <div class="flex items-center justify-between group">
                    <code class="text-slate-300">company:CVS -city:Chicago</code>
                    <button onclick={() => applySyntaxExample('company:CVS -city:Chicago')} class="text-[10px] text-cyan-400 hover:underline cursor-pointer">Try</button>
                  </div>
                  <div class="flex items-center justify-between group">
                    <code class="text-slate-300">NOT title:Intern</code>
                    <button onclick={() => applySyntaxExample('NOT title:Intern')} class="text-[10px] text-cyan-400 hover:underline cursor-pointer">Try</button>
                  </div>
                </div>
              </div>

              <!-- Numeric & Ranges -->
              <div class="p-2.5 rounded-xl bg-slate-950/70 border border-slate-800/80">
                <span class="block text-[11px] font-bold uppercase tracking-wider text-emerald-400 mb-1">
                  Numeric &amp; Range Comparisons (&gt;, &lt;, ..)
                </span>
                <div class="space-y-1.5 font-mono text-[11px]">
                  <div class="flex items-center justify-between group">
                    <code class="text-slate-300">salary:>80k</code>
                    <button onclick={() => applySyntaxExample('salary:>80k')} class="text-[10px] text-emerald-400 hover:underline cursor-pointer">Try</button>
                  </div>
                  <div class="flex items-center justify-between group">
                    <code class="text-slate-300">salary:50k..100k</code>
                    <button onclick={() => applySyntaxExample('salary:50k..100k')} class="text-[10px] text-emerald-400 hover:underline cursor-pointer">Try</button>
                  </div>
                  <div class="flex items-center justify-between group">
                    <code class="text-slate-300">date:>2026-01-01</code>
                    <button onclick={() => applySyntaxExample('date:>2026-01-01')} class="text-[10px] text-emerald-400 hover:underline cursor-pointer">Try</button>
                  </div>
                </div>
              </div>

              <!-- Presence & Exact Match -->
              <div class="p-2.5 rounded-xl bg-slate-950/70 border border-slate-800/80">
                <span class="block text-[11px] font-bold uppercase tracking-wider text-indigo-400 mb-1">
                  Presence &amp; Exact Match
                </span>
                <div class="space-y-1.5 font-mono text-[11px]">
                  <div class="flex items-center justify-between group">
                    <code class="text-slate-300">missing:city</code>
                    <button onclick={() => applySyntaxExample('missing:city')} class="text-[10px] text-indigo-400 hover:underline cursor-pointer">Try</button>
                  </div>
                  <div class="flex items-center justify-between group">
                    <code class="text-slate-300">populated:salary</code>
                    <button onclick={() => applySyntaxExample('populated:salary')} class="text-[10px] text-indigo-400 hover:underline cursor-pointer">Try</button>
                  </div>
                  <div class="flex items-center justify-between group">
                    <code class="text-slate-300">company:"CVS Health"</code>
                    <button onclick={() => applySyntaxExample('company:"CVS Health"')} class="text-[10px] text-indigo-400 hover:underline cursor-pointer">Try</button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Active Filter Banner / Clear Filter Button -->
      {#if feedState.searchFilter}
        {@const isMissing = feedState.searchFilter.startsWith("missing:")}
        {@const isPopulated = feedState.searchFilter.startsWith("populated:")}
        {#if isMissing}
          <div class="px-3 py-1.5 rounded-xl text-xs font-semibold backdrop-blur-md bg-rose-500/15 border border-rose-500/30 text-rose-300 flex items-center gap-2 shadow-sm shrink-0">
            <span>Missing: <strong class="text-white font-mono">{feedState.searchFilter.slice(8)}</strong></span>
            <button onclick={handleSearchClear} class="p-0.5 hover:text-white transition cursor-pointer" title="Clear filter">
              <X class="w-3.5 h-3.5" />
            </button>
          </div>
        {:else if isPopulated}
          <div class="px-3 py-1.5 rounded-xl text-xs font-semibold backdrop-blur-md bg-sky-500/15 border border-sky-500/30 text-sky-300 flex items-center gap-2 shadow-sm shrink-0">
            <span>Populated: <strong class="text-white font-mono">{feedState.searchFilter.slice(10)}</strong></span>
            <button onclick={handleSearchClear} class="p-0.5 hover:text-white transition cursor-pointer" title="Clear filter">
              <X class="w-3.5 h-3.5" />
            </button>
          </div>
        {:else}
          <button
            onclick={handleSearchClear}
            class="px-3 py-2 rounded-xl text-xs font-bold text-rose-400 bg-rose-500/15 hover:bg-rose-500/25 border border-rose-500/30 transition cursor-pointer flex items-center gap-1.5 shrink-0"
            title="Clear search filter and show all records"
          >
            <X class="w-3.5 h-3.5" />
            <span>Clear Filter</span>
          </button>
        {/if}
      {:else if searchInput}
        <button
          onclick={handleSearchClear}
          class="px-3 py-2 rounded-xl text-xs font-bold text-slate-400 bg-slate-800/80 hover:bg-slate-700 border border-slate-700 transition cursor-pointer flex items-center gap-1.5 shrink-0"
        >
          <X class="w-3.5 h-3.5" />
          <span>Clear</span>
        </button>
      {/if}

      <!-- Column Order: Populated First Toggle (Always ON on landing) -->
      <button
        onclick={() => (orderColumnsByFill = !orderColumnsByFill)}
        class="px-3 py-2 rounded-xl text-xs font-bold transition cursor-pointer flex items-center gap-1.5 border shrink-0 {orderColumnsByFill
          ? 'bg-blue-500/15 border-blue-500/35 text-blue-300'
          : 'bg-slate-900/80 border-slate-800 text-slate-400 hover:text-white hover:bg-slate-800'}"
        title="Sort columns by fill rate so 100% populated fields appear on the left"
      >
        <span>Populated First</span>
      </button>

      <!-- Select & Export Custom Columns Button -->
      <button
        onclick={() => (showExportColumnsModal = true)}
        class="px-3 py-2 rounded-xl text-xs font-semibold text-cyan-300 bg-cyan-950/50 hover:bg-cyan-900/60 border border-cyan-800/60 transition cursor-pointer flex items-center gap-1.5 shrink-0 shadow-sm"
        title="Select one or more columns to export, deduplicate, and filter"
      >
        <Building2 class="w-3.5 h-3.5 text-cyan-400" />
        <span>Export Columns ({selectedExportCols.length})</span>
        <ChevronDown class="w-3 h-3 text-cyan-400/80" />
      </button>
    </div>

    <!-- Right Controls: Rows Per Page & Segmented Pagination -->
    <div class="flex items-center gap-3 shrink-0">
      <!-- Page Size Selector -->
      <div class="flex items-center gap-1.5 text-xs text-slate-400">
        <span>Rows:</span>
        <select
          bind:value={feedState.pageSize}
          onchange={() => {
            feedState.currentPage = 1;
            feedState.fetchTablePage();
          }}
          class="bg-slate-900 border border-slate-800 text-slate-200 rounded-lg px-2.5 py-1 text-xs focus:outline-none focus:border-blue-500 font-mono"
        >
          <option value={25}>25</option>
          <option value={50}>50</option>
          <option value={100}>100</option>
          <option value={250}>250</option>
        </select>
      </div>

      <!-- Pagination Segmented Controls -->
      {#if feedState.tableData}
        <div class="flex items-center gap-1 bg-slate-900/90 border border-slate-800 rounded-xl p-1 text-xs font-mono text-slate-300">
          <button
            onclick={() => feedState.setPage(feedState.currentPage - 1)}
            disabled={feedState.currentPage <= 1 || feedState.isLoadingTable}
            class="px-2 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-slate-800 transition cursor-pointer"
            title="Previous page"
          >
            <ChevronLeft class="w-3.5 h-3.5" />
          </button>
          <span class="px-2 text-white font-bold">
            {feedState.currentPage} / {feedState.tableData.total_pages}
          </span>
          <button
            onclick={() => feedState.setPage(feedState.currentPage + 1)}
            disabled={feedState.currentPage >= feedState.tableData.total_pages || feedState.isLoadingTable}
            class="px-2 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-slate-800 transition cursor-pointer"
            title="Next page"
          >
            <ChevronRight class="w-3.5 h-3.5" />
          </button>
        </div>
      {/if}
    </div>
  </div>

  <!-- Master-Detail Workspace: Table on Left + Sliding Inspector Drawer on Right -->
  <div class="flex-1 flex gap-4 overflow-hidden relative">
    
    <!-- Left Table Container -->
    <div class="flex-1 data-card rounded-2xl overflow-hidden flex flex-col relative transition-all duration-300">
      {#if feedState.isLoadingTable}
        <div class="absolute inset-0 bg-slate-950/40 backdrop-blur-[2px] z-20 flex items-center justify-center">
          <RefreshCw class="w-6 h-6 text-blue-400 animate-spin" />
        </div>
      {/if}

      <div class="overflow-x-auto overflow-y-auto flex-1 isolate">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-[#0b101b] sticky top-0 z-20 border-b border-white/[0.08] text-slate-300 font-bold uppercase tracking-wider text-[10.5px] select-none shadow-md">
            <tr>
              <th class="py-3 px-3 w-12 text-center text-slate-500 bg-slate-950/95 font-mono">#</th>
              {#each visibleColumns() as col}
                {@const isTargetCol = filterTargetCol() !== null && (filterTargetCol() === col.toLowerCase() || filterTargetCol() === col.toLowerCase().replace(/[^a-z0-9_]/g, "_"))}
                <th
                  class="py-3 px-3.5 cursor-pointer hover:text-white whitespace-nowrap bg-slate-950/95 font-sans transition-colors {isTargetCol ? 'bg-cyan-950/40 text-cyan-300' : ''}"
                  onclick={() => feedState.setSort(col)}
                >
                  <div class="flex items-center gap-1.5">
                    <span class="{isTargetCol ? 'text-cyan-300 font-bold' : ''}">{col}</span>
                    {#if isTargetCol}
                      <span class="px-1.5 py-0.2 rounded text-[9px] font-sans font-bold bg-cyan-500/20 text-cyan-300 border border-cyan-400/30">
                        TARGET
                      </span>
                    {/if}
                    <ArrowUpDown class="w-3 h-3 text-slate-500" />
                  </div>
                </th>
              {/each}
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-800/60 text-slate-300 font-sans">
            {#if feedState.tableData && feedState.tableData.rows.length > 0}
              {#each feedState.tableData.rows as row, idx}
                {@const isSelected = feedState.inspectingRecord === row}
                <tr
                  onclick={() => (feedState.inspectingRecord = row)}
                  class="transition cursor-pointer group {isSelected
                    ? 'bg-blue-600/15 border-l-2 border-l-blue-500'
                    : 'hover:bg-slate-800/40'}"
                >
                  <!-- Row index -->
                  <td class="py-2.5 px-3 text-center text-[11px] font-mono {isSelected ? 'text-blue-400 font-bold' : 'text-slate-500 group-hover:text-blue-400'}">
                    {(feedState.currentPage - 1) * feedState.pageSize + idx + 1}
                  </td>
                  
                  <!-- Visible Columns -->
                  {#each visibleColumns() as col}
                    {@const val = row[col]}
                    {@const isTargetCol = filterTargetCol() !== null && (filterTargetCol() === col.toLowerCase() || filterTargetCol() === col.toLowerCase().replace(/[^a-z0-9_]/g, "_"))}
                    <td class="py-2.5 px-3.5 max-w-[280px] truncate text-xs font-sans {isTargetCol ? 'bg-white/[0.02]' : ''}">
                      {#if val === null || val === undefined || String(val).trim() === ""}
                        {#if isTargetCol && feedState.searchFilter.startsWith("missing:")}
                          <span class="text-rose-400/80 font-mono text-[11px] italic">missing</span>
                        {:else}
                          <span class="text-slate-600 italic text-[11px] font-mono">—</span>
                        {/if}
                      {:else if val === "false"}
                        <span class="text-slate-500 font-mono text-[11px]">false</span>
                      {:else if val === "true"}
                        <span class="text-slate-300 font-mono text-[11px]">true</span>
                      {:else if String(val).startsWith("http://") || String(val).startsWith("https://")}
                        <span class="text-cyan-400 hover:underline truncate font-mono text-[11px]" title={val}>{val}</span>
                      {:else}
                        <span class="text-slate-200 font-sans" title={val}>{val}</span>
                      {/if}
                    </td>
                  {/each}
                </tr>
              {/each}
            {:else}
              <tr>
                <td colspan={100} class="py-12 text-center text-slate-500 italic">
                  No matching records found.
                </td>
              </tr>
            {/if}
          </tbody>
        </table>
      </div>

      <!-- Table Footer with Matching Count -->
      <div class="px-4 py-2.5 bg-slate-950/90 border-t border-slate-800 flex items-center justify-between text-xs text-slate-400 shrink-0 font-sans">
        <div class="flex items-center gap-2">
          <Eye class="w-3.5 h-3.5 text-blue-400" />
          <span class="hidden sm:inline">Click any row to inspect all fields in the sidecar drawer</span>
          <span class="sm:hidden">Click row to inspect</span>
        </div>
        {#if feedState.tableData}
          <div class="font-mono flex items-center gap-2 text-xs">
            <span class="w-2 h-2 rounded-full bg-cyan-400 animate-pulse"></span>
            <span>Total Matching: <strong class="text-white font-bold">{feedState.tableData.total_records.toLocaleString()}</strong> records</span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Right: Sliding Frosted Glass Inspector Drawer (Sidecar) -->
    {#if feedState.inspectingRecord}
      <div class="w-80 lg:w-96 glass-modal-panel rounded-2xl flex flex-col overflow-hidden shadow-2xl border border-white/15 animate-in slide-in-from-right-4 duration-200 shrink-0 font-sans">
        <!-- Drawer Header -->
        <div class="px-4 py-3 border-b border-white/10 flex items-center justify-between bg-white/[0.02]">
          <div class="flex items-center gap-2">
            <span class="text-base">🔎</span>
            <div>
              <h3 class="text-xs font-bold text-white tracking-tight leading-tight">
                Record Inspector #{currentRowIndex() >= 0 ? (feedState.currentPage - 1) * feedState.pageSize + currentRowIndex() + 1 : ""}
              </h3>
              <p class="text-[10px] text-slate-400">Full field attribute breakdown</p>
            </div>
          </div>
          <div class="flex items-center gap-1.5">
            <span class="text-[9px] font-mono text-slate-400 bg-white/5 border border-white/10 px-1.5 py-0.5 rounded select-none">
              Esc
            </span>
            <button
              onclick={closeInspector}
              class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition cursor-pointer"
              title="Close Drawer (Esc)"
            >
              <X class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- Row Quality Health Score Banner -->
        <div class="px-4 py-2.5 bg-white/[0.03] border-b border-white/10 flex items-center justify-between text-xs font-sans">
          <span class="text-slate-400 text-[11px]">Row Quality:</span>
          <div class="flex items-center gap-2">
            <div class="w-16 bg-slate-800 h-1.5 rounded-full overflow-hidden">
              <div
                class="h-full rounded-full {selectedRowHealth().rate >= 90 ? 'bg-emerald-500' : selectedRowHealth().rate >= 50 ? 'bg-amber-500' : 'bg-rose-500'}"
                style="width: {selectedRowHealth().rate}%"
              ></div>
            </div>
            <span class="font-mono text-xs font-bold {selectedRowHealth().rate >= 90 ? 'text-emerald-400' : selectedRowHealth().rate >= 50 ? 'text-amber-400' : 'text-rose-400'}">
              {selectedRowHealth().rate.toFixed(0)}%
            </span>
          </div>
        </div>

        <!-- Quick Field Search Filter -->
        <div class="p-3 border-b border-white/10 bg-white/[0.01]">
          <div class="relative">
            <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              type="text"
              placeholder="Search record attributes..."
              bind:value={fieldFilterQuery}
              class="w-full bg-slate-950/80 border border-slate-800 rounded-lg pl-8 pr-3 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 font-sans transition"
            />
          </div>
        </div>

        <!-- Scrollable Fields List with Modern Typography -->
        <div class="flex-1 overflow-y-auto p-3 space-y-2 font-sans text-xs">
          {#if filteredInspectorFields().length === 0}
            <div class="py-8 text-center text-xs text-slate-500">
              No fields matching "{fieldFilterQuery}"
            </div>
          {:else}
            {#each filteredInspectorFields() as [key, value]}
              {@const isPopulated = value !== null && value !== undefined && String(value).trim() !== ""}
              <div class="p-2.5 rounded-xl bg-slate-900/60 border border-slate-800/80 hover:border-slate-700 transition flex flex-col gap-1 group">
                <div class="flex items-center justify-between text-[10.5px]">
                  <span class="font-bold {isPopulated ? 'text-blue-400' : 'text-slate-500'} truncate max-w-[200px]" title={key}>
                    {key}
                  </span>
                  {#if isPopulated}
                    <button
                      onclick={() => copyValue(key, String(value))}
                      class="text-slate-500 hover:text-white transition cursor-pointer p-0.5"
                      title="Copy value"
                    >
                      {#if copiedKey === key}
                        <Check class="w-3 h-3 text-emerald-400" />
                      {:else}
                        <Copy class="w-3 h-3" />
                      {/if}
                    </button>
                  {:else}
                    <span class="text-rose-400/80 text-[9px] font-bold font-mono">EMPTY</span>
                  {/if}
                </div>

                <div class="text-xs break-all select-all font-sans">
                  {#if isPopulated}
                    {#if String(value).startsWith("http")}
                      <a href={value} target="_blank" rel="noreferrer" class="text-cyan-400 hover:underline font-mono text-[11px]">
                        {value}
                      </a>
                    {:else}
                      <span class="text-slate-200">{value}</span>
                    {/if}
                  {:else}
                    <span class="text-slate-600 italic font-mono text-[10px]">null</span>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>

        <!-- Drawer Footer: Prev / Next / Copy JSON -->
        <div class="p-3 border-t border-white/10 bg-white/[0.02] flex items-center justify-between gap-2 font-sans">
          <div class="flex items-center gap-1 font-mono text-xs">
            <button
              onclick={selectPreviousRow}
              disabled={currentRowIndex() <= 0}
              class="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-slate-800 text-slate-200 transition cursor-pointer text-xs"
              title="Previous record in table"
            >
              ‹ Prev
            </button>
            <button
              onclick={selectNextRow}
              disabled={!feedState.tableData || currentRowIndex() >= feedState.tableData.rows.length - 1}
              class="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 disabled:opacity-30 disabled:hover:bg-slate-800 text-slate-200 transition cursor-pointer text-xs"
              title="Next record in table"
            >
              Next ›
            </button>
          </div>

          <button
            onclick={copyRecordJson}
            class="px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white font-bold text-xs flex items-center gap-1.5 transition cursor-pointer shadow-sm font-sans"
          >
            <Copy class="w-3 h-3" />
            <span>Copy JSON</span>
          </button>
        </div>
      </div>
    {/if}

  </div>
</div>

<!-- Select Columns & Export Custom Dataset Modal -->
{#if showExportColumnsModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/75 backdrop-blur-md animate-in fade-in duration-150">
    <div class="glass-modal-panel rounded-3xl w-full max-w-lg flex flex-col shadow-2xl border border-white/15 max-h-[85vh] overflow-hidden">
      <!-- Modal Header -->
      <div class="px-6 py-4 border-b border-white/10 flex items-center justify-between bg-white/[0.02]">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-xl bg-cyan-500/20 border border-cyan-400/30 flex items-center justify-center">
            <Download class="w-4 h-4 text-cyan-300" />
          </div>
          <div>
            <h3 class="text-sm font-bold text-white tracking-tight">Export Custom Columns</h3>
            <p class="text-[11.5px] text-slate-400">Select columns to export, deduplicate, and filter</p>
          </div>
        </div>
        <button
          onclick={() => (showExportColumnsModal = false)}
          class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
          title="Close"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-5 space-y-4 overflow-y-auto font-sans flex-1">
        <!-- Column Selection Section -->
        <div>
          <div class="flex items-center justify-between mb-2">
            <label class="text-[11px] font-bold uppercase tracking-wider text-slate-400">
              Columns ({selectedExportCols.length} of {feedState.tableData?.columns.length || 0} selected)
            </label>
            <div class="flex items-center gap-2 text-xs">
              <button
                type="button"
                onclick={selectAllExportCols}
                class="text-cyan-400 hover:text-cyan-300 hover:underline cursor-pointer font-medium"
              >
                Select All
              </button>
              <span class="text-slate-600">|</span>
              <button
                type="button"
                onclick={clearAllExportCols}
                class="text-slate-400 hover:text-slate-300 hover:underline cursor-pointer font-medium"
              >
                Clear All
              </button>
            </div>
          </div>

          <div class="relative mb-2">
            <Search class="w-3.5 h-3.5 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              type="text"
              placeholder="Filter columns..."
              bind:value={exportColSearchQuery}
              class="w-full bg-slate-950 border border-slate-800 rounded-xl pl-9 pr-3 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500 font-mono transition"
            />
          </div>

          <!-- Scrollable Column Picker with Checkboxes -->
          <div class="bg-slate-950/80 border border-slate-800/80 rounded-xl max-h-56 overflow-y-auto divide-y divide-slate-900 font-mono text-xs">
            {#each modalFilteredCols() as col}
              {@const isSelected = selectedExportCols.includes(col)}
              {@const fillRate = columnFillRates().get(col.toLowerCase())}
              <button
                type="button"
                onclick={() => toggleExportCol(col)}
                class="w-full text-left px-3.5 py-2 flex items-center justify-between transition cursor-pointer hover:bg-slate-900 {isSelected ? 'bg-cyan-950/40 text-cyan-200' : 'text-slate-300'}"
              >
                <div class="flex items-center gap-2.5 min-w-0">
                  <div class="w-4 h-4 rounded border flex items-center justify-center transition {isSelected ? 'bg-cyan-500 border-cyan-400 text-slate-950' : 'border-slate-700 bg-slate-900'}">
                    {#if isSelected}
                      <Check class="w-3 h-3 stroke-[3]" />
                    {/if}
                  </div>
                  <span class="truncate {isSelected ? 'font-semibold' : ''}">{col}</span>
                </div>
                {#if fillRate !== undefined}
                  <span class="text-[10px] font-sans px-1.5 py-0.5 rounded {fillRate > 80 ? 'text-emerald-400 bg-emerald-950/40' : fillRate > 20 ? 'text-amber-400 bg-amber-950/40' : 'text-slate-500 bg-slate-900'}">
                    {fillRate.toFixed(0)}%
                  </span>
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <!-- Deduplication Toggle Option -->
        <div class="p-3 rounded-xl bg-slate-900/90 border border-slate-800 flex items-center justify-between">
          <div class="space-y-0.5">
            <span class="text-xs font-semibold text-slate-200">Deduplicate Rows (Unique values only)</span>
            <p class="text-[11px] text-slate-400">
              Only outputs unique combinations of the selected columns (runs fast DuckDB SELECT DISTINCT)
            </p>
          </div>
          <input
            type="checkbox"
            bind:checked={deduplicateExport}
            disabled={includeCountsExport}
            class="w-4 h-4 rounded border-slate-700 text-cyan-500 focus:ring-0 cursor-pointer accent-cyan-500 disabled:opacity-40"
          />
        </div>

        <!-- Include Record Count & Percentage Option -->
        <div class="p-3 rounded-xl bg-slate-900/90 border border-slate-800 flex items-center justify-between">
          <div class="space-y-0.5">
            <span class="text-xs font-semibold text-slate-200">Include Record Count &amp; Percentage</span>
            <p class="text-[11px] text-slate-400">
              Adds <code class="text-cyan-300 font-mono text-[10.5px]">count</code> and <code class="text-cyan-300 font-mono text-[10.5px]">percentage</code> columns showing frequency breakdown sorted descending
            </p>
          </div>
          <input
            type="checkbox"
            bind:checked={includeCountsExport}
            class="w-4 h-4 rounded border-slate-700 text-cyan-500 focus:ring-0 cursor-pointer accent-cyan-500"
          />
        </div>

        <!-- Filter Scope Toggle Option -->
        {#if feedState.searchFilter}
          <div class="p-3 rounded-xl bg-slate-900/90 border border-slate-800 flex items-center justify-between">
            <div class="space-y-0.5">
              <span class="text-xs font-semibold text-slate-200">Apply Active Filter</span>
              <p class="text-[11px] text-slate-400 font-mono truncate max-w-[280px]">
                {feedState.searchFilter}
              </p>
            </div>
            <input
              type="checkbox"
              bind:checked={applyFilterToExport}
              class="w-4 h-4 rounded border-slate-700 text-cyan-500 focus:ring-0 cursor-pointer accent-cyan-500"
            />
          </div>
        {/if}
      </div>

      <!-- Modal Footer -->
      <div class="px-5 py-3.5 bg-slate-950/80 border-t border-white/10 flex items-center justify-between">
        <span class="text-xs text-slate-400 font-sans">
          Selected: <strong class="text-white font-mono">{selectedExportCols.length}</strong> column{selectedExportCols.length === 1 ? '' : 's'}
        </span>
        <div class="flex items-center gap-2">
          <button
            onclick={() => (showExportColumnsModal = false)}
            class="px-3 py-1.5 rounded-xl text-xs font-semibold text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
          >
            Cancel
          </button>
          <button
            onclick={triggerExportColumns}
            disabled={selectedExportCols.length === 0}
            class="px-4 py-1.5 rounded-xl text-xs font-bold text-white bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 transition cursor-pointer shadow-md shadow-cyan-600/20 flex items-center gap-1.5"
          >
            <Download class="w-3.5 h-3.5 text-white" />
            <span>Export CSV</span>
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
