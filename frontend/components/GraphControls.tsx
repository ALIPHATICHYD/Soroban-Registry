import { useState } from "react";
import {
  Search,
  ZoomIn,
  ZoomOut,
  Maximize2,
  Download,
  FileImage,
  GitBranch,
  Circle,
  Sparkles,
  ChevronUp,
  ChevronDown,
  Keyboard,
  ChevronLeft,
  ChevronRight,
  BarChart2,
} from "lucide-react";

interface GraphControlsProps {
  searchQuery: string;
  onSearchChange: (q: string) => void;
  networkFilter: string;
  onNetworkFilterChange: (n: string) => void;
  dependencyTypeFilter: string;
  onDependencyTypeFilterChange: (value: string) => void;
  showCyclesOnly: boolean;
  onShowCyclesOnlyChange: (value: boolean) => void;
  minCallFrequency: number;
  onMinCallFrequencyChange: (value: number) => void;
  demoMode?: boolean;
  onDemoModeChange?: (v: boolean) => void;
  demoNodeCount?: number;
  onDemoNodeCountChange?: (v: number) => void;
  explorationMode?: boolean;
  onExplorationModeChange?: (v: boolean) => void;
  totalNodes: number;
  totalEdges: number;
  cyclicEdgeCount: number;
  criticalCount: number;
  searchMatchCount: number;
  searchMatchIndex: number;
  onPrevMatch: () => void;
  onNextMatch: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onResetZoom: () => void;
  onExportSVG: () => void;
  onExportPNG: () => void;
  onPanUp?: () => void;
  onPanDown?: () => void;
  onPanLeft?: () => void;
  onPanRight?: () => void;
  // per-network node counts for the stats panel
  networkCounts?: {
    mainnet: number;
    testnet: number;
    futurenet: number;
    other: number;
  };
}

/* Shared panel class */
const panel =
  "bg-card/90 backdrop-blur-xl border border-border rounded-xl shadow-lg";
const btnBase =
  "text-muted-foreground hover:text-foreground hover:bg-accent transition-colors rounded focus-visible:ring-1 focus-visible:ring-primary focus:outline-none";

export default function GraphControls({
  searchQuery,
  onSearchChange,
  networkFilter,
  onNetworkFilterChange,
  dependencyTypeFilter,
  onDependencyTypeFilterChange,
  showCyclesOnly,
  onShowCyclesOnlyChange,
  minCallFrequency,
  onMinCallFrequencyChange,
  demoMode = false,
  onDemoModeChange = () => {},
  demoNodeCount = 100,
  onDemoNodeCountChange = () => {},
  totalNodes,
  totalEdges,
  cyclicEdgeCount,
  criticalCount,
  searchMatchCount,
  searchMatchIndex,
  onPrevMatch,
  onNextMatch,
  onZoomIn,
  onZoomOut,
  onResetZoom,
  onExportSVG,
  onExportPNG,
  onPanUp,
  onPanDown,
  onPanLeft,
  onPanRight,
  networkCounts,
  explorationMode = false,
  onExplorationModeChange = () => {},
}: GraphControlsProps) {
  const [statsOpen, setStatsOpen] = useState(false);
  return (
    <>
      {/* Top-left: Search + Filters */}
      <div
        className="absolute top-4 left-4 z-30 flex flex-col gap-2.5 max-w-xs"
        role="region"
        aria-label="Graph search and filters"
      >
        {/* Search */}
        <div className={`${panel} overflow-hidden`}>
          <div className="relative flex items-center">
            <Search className="absolute left-3 w-4 h-4 text-muted-foreground" />
            <input
              id="graph-search"
              type="search"
              value={searchQuery}
              onChange={(e) => onSearchChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && searchMatchCount > 0) onNextMatch();
              }}
              placeholder="Search contracts…"
              aria-label="Search graph nodes"
              aria-controls="graph-search-status"
              className="w-full pl-9 pr-3 py-2.5 bg-transparent text-sm text-foreground placeholder-muted-foreground focus:outline-none focus-visible:ring-1 focus-visible:ring-primary rounded"
            />
            {/* Live region for search result announcement */}
            <span
              id="graph-search-status"
              aria-live="polite"
              className="sr-only"
            >
              {searchQuery && searchMatchCount > 0
                ? `${searchMatchCount} match${searchMatchCount !== 1 ? "es" : ""} found, showing ${searchMatchIndex + 1}`
                : searchQuery && searchMatchCount === 0
                  ? "No matches found"
                  : ""}
            </span>
            {searchQuery && searchMatchCount > 0 && (
              <div className="flex items-center gap-0.5 pr-1.5 shrink-0">
                <span className="text-xs text-muted-foreground tabular-nums px-1">
                  {searchMatchIndex + 1}/{searchMatchCount}
                </span>
                <button
                  onClick={onPrevMatch}
                  className={`p-0.5 ${btnBase}`}
                  aria-label="Previous search match"
                  title="Previous match"
                >
                  <ChevronUp className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={onNextMatch}
                  className={`p-0.5 ${btnBase}`}
                  aria-label="Next search match"
                  title="Next match"
                >
                  <ChevronDown className="w-3.5 h-3.5" />
                </button>
              </div>
            )}
            {searchQuery && searchMatchCount === 0 && (
              <span className="text-xs text-red-400 pr-2.5 shrink-0">
                No results
              </span>
            )}
          </div>
        </div>

        {/* Filters */}
        <div className={`${panel} p-3 space-y-3`}>
          <div>
            <label className="text-[10px] text-muted-foreground uppercase tracking-wider mb-1.5 block font-medium">
              Network
            </label>
            <select
              id="graph-network-filter"
              value={networkFilter}
              onChange={(e) => onNetworkFilterChange(e.target.value)}
              className="w-full px-3 py-1.5 rounded-lg bg-background border border-border text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary cursor-pointer"
            >
              <option value="">All Networks</option>
              <option value="mainnet">Mainnet</option>
              <option value="testnet">Testnet</option>
              <option value="futurenet">Futurenet</option>
            </select>
          </div>

          <div>
            <label className="text-[10px] text-muted-foreground uppercase tracking-wider mb-1.5 block font-medium">
              Dependency Type
            </label>
            <select
              id="graph-dependency-type-filter"
              value={dependencyTypeFilter}
              onChange={(e) => onDependencyTypeFilterChange(e.target.value)}
              className="w-full px-3 py-1.5 rounded-lg bg-background border border-border text-sm text-foreground focus:outline-none focus:ring-1 focus:ring-primary cursor-pointer"
            >
              <option value="">All Types</option>
              <option value="calls">Calls</option>
              <option value="imports">Imports</option>
            </select>
          </div>

          <div>
            <label className="text-[10px] text-muted-foreground uppercase tracking-wider mb-1.5 block font-medium">
              Min Call Frequency:{" "}
              <span className="text-foreground font-medium">
                {minCallFrequency}
              </span>
            </label>
            <input
              id="graph-min-call-frequency"
              type="range"
              min={0}
              max={250}
              step={5}
              value={minCallFrequency}
              onChange={(e) => onMinCallFrequencyChange(Number(e.target.value))}
              className="w-full h-1.5 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
            />
          </div>

          <div>
            <button
              onClick={() => onShowCyclesOnlyChange(!showCyclesOnly)}
              className={`flex items-center gap-2 w-full text-sm transition-colors ${showCyclesOnly ? "text-red-500" : "text-muted-foreground hover:text-foreground"}`}
            >
              <div
                className={`w-4 h-4 rounded border flex items-center justify-center flex-shrink-0 transition-colors ${showCyclesOnly ? "bg-red-500 border-red-500" : "border-border"}`}
              >
                {showCyclesOnly && (
                  <svg
                    className="w-3 h-3 text-white"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    strokeWidth={3}
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                )}
              </div>
              <span className="font-medium">
                Show Circular Dependencies Only
              </span>
            </button>

            <button
              onClick={() => onExplorationModeChange(!explorationMode)}
              className={`flex items-center gap-2 w-full text-sm mt-3 transition-colors ${explorationMode ? "text-primary" : "text-muted-foreground hover:text-foreground"}`}
            >
              <div
                className={`w-4 h-4 rounded border flex items-center justify-center flex-shrink-0 transition-colors ${explorationMode ? "bg-primary border-primary" : "border-border"}`}
              >
                {explorationMode && (
                  <svg
                    className="w-3 h-3 text-white"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    strokeWidth={3}
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                )}
              </div>
              <GitBranch className="w-3.5 h-3.5 text-primary/70" />
              <span className="font-medium">
                Exploration Mode (Click to Expand)
              </span>
            </button>
          </div>

          {/* Demo Mode */}
          <div className="border-t border-border pt-3">
            <button
              onClick={() => onDemoModeChange(!demoMode)}
              className={`flex items-center gap-2 w-full text-sm transition-colors ${demoMode ? "text-amber-500" : "text-muted-foreground hover:text-foreground"}`}
            >
              <div
                className={`w-4 h-4 rounded border flex items-center justify-center flex-shrink-0 transition-colors ${demoMode ? "bg-amber-500 border-amber-500" : "border-border"}`}
              >
                {demoMode && (
                  <svg
                    className="w-3 h-3 text-white"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    strokeWidth={3}
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                )}
              </div>
              <Sparkles className="w-3.5 h-3.5 text-amber-400" />
              <span className="font-medium">Demo Mode</span>
            </button>
            {demoMode && (
              <div className="mt-2.5">
                <label className="text-[10px] text-muted-foreground mb-1 block">
                  Contracts:{" "}
                  <span className="text-foreground font-medium">
                    {demoNodeCount.toLocaleString()}
                  </span>
                </label>
                <input
                  id="graph-demo-count"
                  type="range"
                  min={50}
                  max={10000}
                  step={50}
                  value={demoNodeCount}
                  onChange={(e) =>
                    onDemoNodeCountChange(Number(e.target.value))
                  }
                  className="w-full h-1.5 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
                />
                <div className="flex justify-between text-[10px] text-muted-foreground mt-0.5">
                  <span>50</span>
                  <span>10,000</span>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Legend */}
        <div className={`${panel} p-3`}>
          <p className="text-[10px] text-muted-foreground uppercase tracking-wider mb-2 font-medium">
            Legend
          </p>
          <div className="space-y-1.5 text-xs">
            <p className="text-[10px] text-muted-foreground/60 uppercase tracking-wider mb-1">
              Network
            </p>
            <div className="flex items-center gap-2">
              <Circle className="w-3 h-3 text-green-500 fill-green-500" />
              <span className="text-muted-foreground">Mainnet</span>
            </div>
            <div className="flex items-center gap-2">
              <Circle className="w-3 h-3 text-blue-500 fill-blue-500" />
              <span className="text-muted-foreground">Testnet</span>
            </div>
            <div className="flex items-center gap-2">
              <Circle className="w-3 h-3 text-purple-500 fill-purple-500" />
              <span className="text-muted-foreground">Futurenet</span>
            </div>
            <div className="border-t border-border my-1.5" />
            <p className="text-[10px] text-muted-foreground/60 uppercase tracking-wider mb-1">
              Node Size
            </p>
            <div className="flex items-center gap-2 pt-0.5">
              <div className="w-3 h-3 rounded-full border-2 border-amber-400" />
              <span className="text-muted-foreground">Critical (≥5 deps)</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-muted-foreground mx-0.5" />
              <span className="text-muted-foreground/70">
                Larger = more dependents
              </span>
            </div>
            <div className="border-t border-border my-1.5" />
            <p className="text-[10px] text-muted-foreground/60 uppercase tracking-wider mb-1">
              Edges
            </p>
            <div className="flex items-center gap-2">
              <GitBranch className="w-3 h-3 text-muted-foreground" />
              <span className="text-muted-foreground/70">
                Arrow = dependency direction
              </span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-[2px] bg-red-500 border-t border-dashed border-red-300" />
              <span className="text-muted-foreground/70">
                Red dashed = circular dependency
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Top-right: Graph Stats panel (collapsible) */}
      <div className="absolute top-4 right-4 z-30">
        <div className={`${panel} overflow-hidden`}>
          {/* Header row — always visible */}
          <button
            id="graph-stats-toggle"
            onClick={() => setStatsOpen((o) => !o)}
            className="flex items-center gap-3 px-4 py-2.5 w-full hover:bg-accent transition-colors"
            aria-expanded={statsOpen}
            aria-controls="graph-stats-body"
            aria-label="Toggle graph statistics panel"
          >
            <BarChart2 className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
            <div className="flex items-center gap-3 flex-1">
              <div className="text-center">
                <div className="text-sm font-bold text-foreground leading-none">
                  {totalNodes.toLocaleString()}
                </div>
                <div className="text-[9px] text-muted-foreground uppercase tracking-wider">
                  Nodes
                </div>
              </div>
              <div className="w-px h-6 bg-border" />
              <div className="text-center">
                <div className="text-sm font-bold text-foreground leading-none">
                  {totalEdges.toLocaleString()}
                </div>
                <div className="text-[9px] text-muted-foreground uppercase tracking-wider">
                  Edges
                </div>
              </div>
              <div className="w-px h-6 bg-border" />
              <div className="text-center">
                <div className="text-sm font-bold text-amber-500 leading-none">
                  {criticalCount}
                </div>
                <div className="text-[9px] text-muted-foreground uppercase tracking-wider">
                  Critical
                </div>
              </div>
              <div className="w-px h-6 bg-border" />
              <div className="text-center">
                <div className="text-sm font-bold text-red-500 leading-none">
                  {cyclicEdgeCount}
                </div>
                <div className="text-[9px] text-muted-foreground uppercase tracking-wider">
                  Cycles
                </div>
              </div>
            </div>
            <ChevronDown
              className={`w-3 h-3 text-muted-foreground transition-transform duration-200 ${statsOpen ? "rotate-180" : ""}`}
            />
          </button>

          {/* Expandable body — network breakdown */}
          {statsOpen && (
            <div
              id="graph-stats-body"
              className="border-t border-border px-4 py-3 space-y-2"
            >
              <p className="text-[10px] text-muted-foreground uppercase tracking-wider mb-2">
                Network Breakdown
              </p>
              {[
                {
                  label: "Mainnet",
                  color: "bg-green-500",
                  count: networkCounts?.mainnet ?? 0,
                },
                {
                  label: "Testnet",
                  color: "bg-blue-500",
                  count: networkCounts?.testnet ?? 0,
                },
                {
                  label: "Futurenet",
                  color: "bg-purple-500",
                  count: networkCounts?.futurenet ?? 0,
                },
                {
                  label: "Other",
                  color: "bg-muted-foreground",
                  count: networkCounts?.other ?? 0,
                },
              ].map(({ label, color, count }) =>
                count > 0 ? (
                  <div key={label} className="flex items-center gap-2">
                    <div className={`w-2 h-2 rounded-full ${color} shrink-0`} />
                    <span className="text-[11px] text-muted-foreground flex-1">
                      {label}
                    </span>
                    <span className="text-[11px] font-mono text-foreground">
                      {count.toLocaleString()}
                    </span>
                    <div className="w-16 bg-muted rounded-full h-1 overflow-hidden">
                      <div
                        className={`h-1 rounded-full ${color}`}
                        style={{
                          width: `${Math.round((count / Math.max(totalNodes, 1)) * 100)}%`,
                        }}
                      />
                    </div>
                  </div>
                ) : null,
              )}
              <div className="border-t border-border pt-2 mt-1">
                <div className="flex justify-between text-[10px]">
                  <span className="text-muted-foreground">Avg edges/node</span>
                  <span className="text-foreground font-mono">
                    {totalNodes > 0
                      ? (totalEdges / totalNodes).toFixed(1)
                      : "0.0"}
                  </span>
                </div>
                {networkFilter !== "all" && (
                  <div className="flex justify-between text-[10px] mt-1">
                    <span className="text-muted-foreground">Active filter</span>
                    <span className="text-primary font-mono capitalize">
                      {networkFilter}
                    </span>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Bottom-left: Keyboard shortcut hints */}
      <div
        className="absolute bottom-4 left-4 z-30 hidden lg:block"
        role="complementary"
        aria-label="Keyboard shortcuts reference"
      >
        <div className={`${panel} p-3`}>
          <div className="flex items-center gap-1.5 mb-2">
            <Keyboard className="w-3 h-3 text-muted-foreground" />
            <p className="text-[10px] text-muted-foreground font-medium uppercase tracking-wider">
              Shortcuts
            </p>
          </div>
          <div className="space-y-1 text-[10px]">
            {[
              { label: "Zoom in/out", key: "+ / -" },
              { label: "Pan", key: "↑ ↓ ← →" },
              { label: "Reset view", key: "R" },
              { label: "Deselect", key: "Esc" },
            ].map(({ label, key }) => (
              <div key={label} className="flex justify-between gap-4">
                <span className="text-muted-foreground">{label}</span>
                <kbd className="font-mono bg-muted text-muted-foreground px-1.5 py-0.5 rounded text-[9px]">
                  {key}
                </kbd>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Bottom-right: Zoom + Export controls */}
      <div
        className="absolute bottom-4 right-4 z-30 flex flex-col gap-2"
        role="group"
        aria-label="Graph view controls"
      >
        {/* Pan d-pad */}
        <div
          className={`${panel} overflow-hidden p-1`}
          role="group"
          aria-label="Pan controls"
        >
          <div className="grid grid-cols-3 gap-0.5 w-[84px]">
            <div />
            <button
              id="graph-pan-up"
              onClick={onPanUp}
              className={`flex items-center justify-center h-7 ${btnBase}`}
              aria-label="Pan up"
              title="Pan up (↑)"
            >
              <ChevronUp className="w-3.5 h-3.5" />
            </button>
            <div />
            <button
              id="graph-pan-left"
              onClick={onPanLeft}
              className={`flex items-center justify-center h-7 ${btnBase}`}
              aria-label="Pan left"
              title="Pan left (←)"
            >
              <ChevronLeft className="w-3.5 h-3.5" />
            </button>
            <button
              id="graph-reset-view"
              onClick={onResetZoom}
              className={`flex items-center justify-center h-7 ${btnBase}`}
              aria-label="Reset view to fit all nodes"
              title="Reset view (R)"
            >
              <Maximize2 className="w-3 h-3" />
            </button>
            <button
              id="graph-pan-right"
              onClick={onPanRight}
              className={`flex items-center justify-center h-7 ${btnBase}`}
              aria-label="Pan right"
              title="Pan right (→)"
            >
              <ChevronRight className="w-3.5 h-3.5" />
            </button>
            <div />
            <button
              id="graph-pan-down"
              onClick={onPanDown}
              className={`flex items-center justify-center h-7 ${btnBase}`}
              aria-label="Pan down"
              title="Pan down (↓)"
            >
              <ChevronDown className="w-3.5 h-3.5" />
            </button>
            <div />
          </div>
        </div>

        {/* Zoom controls */}
        <div
          className={`${panel} overflow-hidden`}
          role="group"
          aria-label="Zoom controls"
        >
          <button
            id="graph-zoom-in"
            onClick={onZoomIn}
            className={`flex items-center justify-center w-9 h-9 ${btnBase}`}
            aria-label="Zoom in (+ key)"
            title="Zoom in (+)"
          >
            <ZoomIn className="w-4 h-4" />
          </button>
          <div className="border-t border-border" role="separator" />
          <button
            id="graph-zoom-out"
            onClick={onZoomOut}
            className={`flex items-center justify-center w-9 h-9 ${btnBase}`}
            aria-label="Zoom out (- key)"
            title="Zoom out (-)"
          >
            <ZoomOut className="w-4 h-4" />
          </button>
          <div className="border-t border-border" role="separator" />
          <button
            id="graph-reset-zoom"
            onClick={onResetZoom}
            className={`flex items-center justify-center w-9 h-9 ${btnBase}`}
            aria-label="Reset zoom (R key)"
            title="Reset zoom (R)"
          >
            <Maximize2 className="w-4 h-4" />
          </button>
        </div>

        {/* Export controls */}
        <div
          className={`${panel} overflow-hidden`}
          role="group"
          aria-label="Export controls"
        >
          <button
            id="graph-export-svg"
            onClick={onExportSVG}
            className={`flex items-center justify-center w-9 h-9 ${btnBase}`}
            aria-label="Export graph as SVG file"
            title="Export as SVG"
          >
            <Download className="w-4 h-4" />
          </button>
          <div className="border-t border-border" role="separator" />
          <button
            id="graph-export-png"
            onClick={onExportPNG}
            className={`flex items-center justify-center w-9 h-9 ${btnBase}`}
            aria-label="Export graph as PNG image"
            title="Export as PNG"
          >
            <FileImage className="w-4 h-4" />
          </button>
        </div>
      </div>
    </>
  );
}
