'use client';

import { useQuery } from '@tanstack/react-query';
import { api, GraphNode, GraphEdge } from '@/lib/api';
import DependencyGraph from '@/components/DependencyGraph';
import GraphControls from '@/components/GraphControls';
import { useState, useCallback, useRef, useMemo, useEffect } from 'react';
import { AlertCircle, Sparkles, ExternalLink, X } from 'lucide-react';
import { useAnalytics } from '@/hooks/useAnalytics';
import type { DependencyGraphHandle } from '@/components/DependencyGraph';
import { useTranslation } from '@/lib/i18n/client';

// Generate synthetic demo data for testing at scale
function generateDemoData(nodeCount: number): { nodes: GraphNode[]; edges: GraphEdge[] } {
    const networks: ('mainnet' | 'testnet' | 'futurenet')[] = ['mainnet', 'testnet', 'futurenet'];
    const categories = ['DeFi', 'NFT', 'DAO', 'Oracle', 'Bridge', 'DEX', 'Lending', 'Staking', 'Wallet', 'Token'];
    const tagOptions = ['soroban', 'stellar', 'defi', 'amm', 'lending', 'governance', 'token', 'nft', 'oracle', 'bridge'];
    const nameAdjectives = ['Swift', 'Quantum', 'Solar', 'Stellar', 'Bright', 'Nova', 'Cosmic', 'Nebula', 'Astral', 'Lunar'];
    const nameNouns = ['Swap', 'Vault', 'Pool', 'Bridge', 'Oracle', 'Token', 'Lend', 'Stake', 'DAO', 'Mint'];

    const nodes: GraphNode[] = [];
    for (let i = 0; i < nodeCount; i++) {
        const adj = nameAdjectives[Math.floor(Math.random() * nameAdjectives.length)];
        const noun = nameNouns[Math.floor(Math.random() * nameNouns.length)];
        const tagCount = 1 + Math.floor(Math.random() * 3);
        const tags: string[] = [];
        for (let t = 0; t < tagCount; t++) {
            const tag = tagOptions[Math.floor(Math.random() * tagOptions.length)];
            if (!tags.includes(tag)) tags.push(tag);
        }
        nodes.push({
            id: `demo-${i}`,
            contract_id: `C${Array.from({ length: 55 }, () => 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'[Math.floor(Math.random() * 32)]).join('')}`,
            name: `${adj}${noun}${i > 0 ? i : ''}`,
            network: networks[Math.floor(Math.random() * networks.length)],
            is_verified: Math.random() > 0.6,
            category: categories[Math.floor(Math.random() * categories.length)],
            tags,
        });
    }

    const edges: GraphEdge[] = [];
    const edgeCount = Math.min(nodeCount * 2, nodeCount * (nodeCount - 1) / 2);
    const edgeSet = new Set<string>();

    const pushEdge = (source: number, target: number, isCircular = false) => {
        if (source === target) return;
        const key = `${source}-${target}`;
        if (edgeSet.has(key)) return;
        edgeSet.add(key);
        const callFrequency = 1 + Math.floor(Math.random() * 250);
        const isEstimated = Math.random() > 0.6;
        edges.push({
            source: nodes[source].id,
            target: nodes[target].id,
            dependency_type: Math.random() > 0.7 ? 'imports' : 'calls',
            call_frequency: callFrequency,
            call_volume: callFrequency * (1 + Math.floor(Math.random() * 8)),
            is_estimated: isEstimated,
            is_circular: isCircular,
        });
    };

    for (let i = 0; i < edgeCount; i++) {
        const sourceIdx = Math.floor(Math.random() * nodeCount);
        const targetIdx = Math.floor(Math.pow(Math.random(), 2) * nodeCount);
        pushEdge(sourceIdx, targetIdx);
    }

    const cycleGroups = Math.min(8, Math.floor(nodeCount / 10));
    for (let i = 0; i < cycleGroups; i++) {
        const a = i * 3;
        const b = a + 1;
        const c = a + 2;
        if (c >= nodeCount) break;
        pushEdge(a, b, true);
        pushEdge(b, c, true);
        pushEdge(c, a, true);
    }

    return { nodes, edges };
}

export function GraphContent() {
    const { t } = useTranslation('common');
    const [networkFilter, setNetworkFilter] = useState<string>('');
    const [dependencyTypeFilter, setDependencyTypeFilter] = useState<string>('');
    const [showCyclesOnly, setShowCyclesOnly] = useState(false);
    const [minCallFrequency, setMinCallFrequency] = useState(0);
    const [searchQuery, setSearchQuery] = useState('');
    const [demoMode, setDemoMode] = useState(false);
    const [demoNodeCount, setDemoNodeCount] = useState(200);
    const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
    const [explorationMode, setExplorationMode] = useState(false);
    const [expandedNodeIds, setExpandedNodeIds] = useState<Set<string>>(new Set());
    const [explorationNodes, setExplorationNodes] = useState<GraphNode[]>([]);
    const [explorationEdges, setExplorationEdges] = useState<GraphEdge[]>([]);
    const [searchMatchIndex, setSearchMatchIndex] = useState(0);
    const graphRef = useRef<DependencyGraphHandle | null>(null);
    const { logEvent } = useAnalytics();

    // Reset exploration state when mode is toggled off
    useEffect(() => {
        if (!explorationMode) {
            requestAnimationFrame(() => {
                setExplorationNodes([]);
                setExplorationEdges([]);
                setExpandedNodeIds(new Set());
            });
        }
    }, [explorationMode]);

    const fetchLocalGraph = useCallback(async (id: string) => {
        if (!id) return;
        try {
            const localData = await api.getContractLocalGraph(id, 1);
            setExplorationNodes(prev => {
                const existingIds = new Set(prev.map(n => n.id));
                const newNodes = localData.nodes.filter(n => !existingIds.has(n.id));
                return [...prev, ...newNodes];
            });
            setExplorationEdges(prev => {
                const existingKeys = new Set(prev.map(e => `${e.source}-${e.target}`));
                const newEdges = localData.edges.filter(e => !existingKeys.has(`${e.source}-${e.target}`));
                return [...prev, ...newEdges];
            });
        } catch (err) {
            console.error('Failed to fetch local graph:', err);
        }
    }, []);

    const { data: apiData, isLoading, error } = useQuery({
        queryKey: ['contract-graph', networkFilter],
        queryFn: () => api.getContractGraph(networkFilter || undefined),
        enabled: !demoMode,
    });

    useEffect(() => {
        if (!error) return;
        logEvent('error_event', {
            source: 'graph_page',
            message: 'Failed to load contract graph data',
            network_filter: networkFilter || 'all',
        });
    }, [error, networkFilter, logEvent]);

    useEffect(() => {
        if (!searchQuery.trim()) return;
        logEvent('search_performed', {
            keyword: searchQuery.trim(),
            source: 'graph_page',
            network_filter: networkFilter || 'all',
            demo_mode: demoMode,
        });
    }, [searchQuery, networkFilter, demoMode, logEvent]);

    const demoData = useMemo(
        () => (demoMode ? generateDemoData(demoNodeCount) : null),
        [demoMode, demoNodeCount]
    );

    const filteredDemoData = useMemo(() => {
        if (!demoData || !networkFilter) return demoData;
        const filteredNodes = demoData.nodes.filter((n) => n.network === networkFilter);
        const nodeIds = new Set(filteredNodes.map((n) => n.id));
        const filteredEdges = demoData.edges.filter(
            (e) => nodeIds.has(e.source) && nodeIds.has(e.target)
        );
        return { nodes: filteredNodes, edges: filteredEdges };
    }, [demoData, networkFilter]);

    const graphData = demoMode ? filteredDemoData : apiData;

    const rawNodes = useMemo(
        () => (graphData && Array.isArray(graphData.nodes) ? graphData.nodes : []),
        [graphData]
    );
    const rawEdges = useMemo(
        () => (graphData && Array.isArray(graphData.edges) ? graphData.edges : []),
        [graphData]
    );

    const filteredGraph = useMemo(() => {
        if (explorationMode) {
            return { nodes: explorationNodes, edges: explorationEdges };
        }

        const nodes = rawNodes.filter((n) => {
            if (networkFilter && n.network !== networkFilter) return false;
            if (searchQuery.trim()) {
                const q = searchQuery.toLowerCase();
                return n.name.toLowerCase().includes(q) || n.contract_id.toLowerCase().includes(q);
            }
            return true;
        });

        const nodeIds = new Set(nodes.map((n) => n.id));
        const edges = rawEdges.filter((e) => {
            if (!nodeIds.has(e.source) || !nodeIds.has(e.target)) return false;
            if (dependencyTypeFilter && e.dependency_type !== dependencyTypeFilter) return false;
            if (minCallFrequency > 0 && (e.call_frequency || 0) < minCallFrequency) return false;
            if (showCyclesOnly && !e.is_circular) return false;
            return true;
        });

        return { nodes, edges };
    }, [explorationMode, explorationNodes, explorationEdges, rawNodes, rawEdges, networkFilter, searchQuery, dependencyTypeFilter, minCallFrequency, showCyclesOnly]);

    const nodes = filteredGraph.nodes;
    const edges = filteredGraph.edges;

    const dependentCounts = useMemo(() => {
        const counts = new Map<string, number>();
        for (const edge of edges) {
            counts.set(edge.target, (counts.get(edge.target) || 0) + 1);
        }
        return counts;
    }, [edges]);

    const dependencyCounts = useMemo(() => {
        const counts = new Map<string, number>();
        for (const edge of edges) {
            counts.set(edge.source, (counts.get(edge.source) || 0) + 1);
        }
        return counts;
    }, [edges]);

    const criticalCount = useMemo(() => {
        let count = 0;
        dependentCounts.forEach((v) => { if (v >= 5) count++; });
        return count;
    }, [dependentCounts]);

    const networkCounts = useMemo(() => {
        const counts = { mainnet: 0, testnet: 0, futurenet: 0, other: 0 };
        for (const node of nodes) {
            const n = node.network?.toLowerCase() ?? '';
            if (n === 'mainnet') counts.mainnet++;
            else if (n === 'testnet') counts.testnet++;
            else if (n === 'futurenet') counts.futurenet++;
            else counts.other++;
        }
        return counts;
    }, [nodes]);

    const handleNodeClick = (node: GraphNode | null) => {
        setSelectedNode(node);
        if (node && explorationMode) {
            fetchLocalGraph(node.id);
        }
        if (node) {
            logEvent('node_selected', {
                contract_id: node.contract_id,
                name: node.name,
                source: 'graph_page',
            });
        }
    };

    const handleExpandNode = (id: string) => {
        setExpandedNodeIds((prev: Set<string>) => {
            const next = new Set(prev);
            if (next.has(id)) {
                next.delete(id);
            } else {
                next.add(id);
                if (explorationMode) {
                    fetchLocalGraph(id);
                }
            }
            return next;
        });
        logEvent('node_expanded', { node_id: id });
    };

    const searchMatches = useMemo(() => {
        if (!searchQuery || nodes.length === 0) return [];
        const q = searchQuery.toLowerCase();
        return nodes
            .filter((n) => n.name.toLowerCase().includes(q) || n.contract_id.toLowerCase().includes(q))
            .map((n) => n.id);
    }, [searchQuery, nodes]);

    useEffect(() => {
        requestAnimationFrame(() => setSearchMatchIndex(0));
    }, [searchQuery]);

    useEffect(() => {
        if (searchMatches.length > 0 && graphRef.current) {
            graphRef.current.focusOnNode(searchMatches[searchMatchIndex] || searchMatches[0]);
        }
    }, [searchMatches, searchMatchIndex]);

    const handlePrevMatch = useCallback(() => {
        setSearchMatchIndex((i) => (i - 1 + searchMatches.length) % searchMatches.length);
    }, [searchMatches.length]);

    const handleNextMatch = useCallback(() => {
        setSearchMatchIndex((i) => (i + 1) % searchMatches.length);
    }, [searchMatches.length]);

    const cyclicEdgeCount = useMemo(
        () => edges.filter((edge) => edge.is_circular).length,
        [edges]
    );

    // Keyboard shortcuts
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
            const g = graphRef.current;
            if (!g) return;
            switch (e.key) {
                case '=':
                case '+':
                    e.preventDefault();
                    g.zoomIn();
                    break;
                case '-':
                    e.preventDefault();
                    g.zoomOut();
                    break;
                case 'r':
                case 'R':
                    e.preventDefault();
                    g.resetZoom();
                    break;
                case 'ArrowUp':
                    e.preventDefault();
                    g.panUp();
                    break;
                case 'ArrowDown':
                    e.preventDefault();
                    g.panDown();
                    break;
                case 'ArrowLeft':
                    e.preventDefault();
                    g.panLeft();
                    break;
                case 'ArrowRight':
                    e.preventDefault();
                    g.panRight();
                    break;
                case 'Escape':
                    setSelectedNode(null);
                    break;
            }
        };
        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, []);

    if (!demoMode && isLoading) {
        return (
            <div className="flex items-center justify-center h-[calc(100vh-4rem)] bg-background">
                <div className="text-center">
                    <div className="inline-block w-10 h-10 border-4 border-primary border-t-transparent rounded-full animate-spin mb-4" />
                    <p className="text-muted-foreground text-sm">{t('graph.loading')}</p>
                </div>
            </div>
        );
    }

    if (!demoMode && error) {
        return (
            <div className="relative h-[calc(100vh-4rem)] overflow-hidden bg-background">
                <div className="absolute inset-0 flex items-center justify-center z-20">
                    <div className="text-center gradient-border-card p-10 max-w-md">
                        <div className="w-14 h-14 rounded-2xl bg-amber-500/10 flex items-center justify-center mx-auto mb-4">
                            <AlertCircle className="w-7 h-7 text-amber-500" />
                        </div>
                        <h3 className="text-xl font-semibold text-foreground mb-2">{t('graph.apiUnavailable')}</h3>
                        <p className="text-muted-foreground mb-6 text-sm leading-relaxed">
                            {t('graph.apiUnavailableDesc')}
                        </p>
                        <button
                            onClick={() => setDemoMode(true)}
                            className="btn-glow px-6 py-2.5 bg-primary hover:brightness-110 text-primary-foreground rounded-lg font-medium transition-all inline-flex items-center gap-2"
                        >
                            <Sparkles className="w-4 h-4" />
                            Enable Demo Mode
                        </button>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="relative h-[calc(100vh-4rem)] overflow-hidden bg-background">
            {/* Graph Canvas */}
            <div className="w-full h-full bg-background relative">
                <DependencyGraph
                    ref={graphRef}
                    nodes={nodes}
                    edges={edges}
                    searchQuery={searchQuery}
                    dependentCounts={dependentCounts}
                    onNodeClick={handleNodeClick}
                    selectedNode={selectedNode}
                />
            </div>

            {/* Controls Overlay */}
            <GraphControls
                searchQuery={searchQuery}
                onSearchChange={setSearchQuery}
                networkFilter={networkFilter}
                onNetworkFilterChange={setNetworkFilter}
                dependencyTypeFilter={dependencyTypeFilter}
                onDependencyTypeFilterChange={setDependencyTypeFilter}
                showCyclesOnly={showCyclesOnly}
                onShowCyclesOnlyChange={setShowCyclesOnly}
                minCallFrequency={minCallFrequency}
                onMinCallFrequencyChange={setMinCallFrequency}
                demoMode={demoMode}
                onDemoModeChange={setDemoMode}
                demoNodeCount={demoNodeCount}
                onDemoNodeCountChange={setDemoNodeCount}
                totalNodes={nodes.length}
                totalEdges={edges.length}
                cyclicEdgeCount={cyclicEdgeCount}
                criticalCount={criticalCount}
                explorationMode={explorationMode}
                onExplorationModeChange={setExplorationMode}
                searchMatchCount={searchMatches.length}
                searchMatchIndex={searchMatchIndex}
                onPrevMatch={handlePrevMatch}
                onNextMatch={handleNextMatch}
                onZoomIn={() => graphRef.current?.zoomIn()}
                onZoomOut={() => graphRef.current?.zoomOut()}
                onResetZoom={() => graphRef.current?.resetZoom()}
                onExportSVG={() => graphRef.current?.exportSVG()}
                onExportPNG={() => graphRef.current?.exportPNG()}
                onPanUp={() => graphRef.current?.panUp()}
                onPanDown={() => graphRef.current?.panDown()}
                onPanLeft={() => graphRef.current?.panLeft()}
                onPanRight={() => graphRef.current?.panRight()}
                networkCounts={networkCounts}
            />

            {/* Selected Node Panel */}
            {selectedNode && (
                <div className="absolute bottom-4 left-4 z-30 w-80 bg-card/90 backdrop-blur-xl border border-border rounded-xl shadow-lg overflow-hidden">
                    <div className="p-4 pb-3">
                        <div className="flex items-start justify-between">
                            <div className="flex-1 min-w-0 pr-2">
                                <h3 className="font-semibold text-foreground text-base truncate">{selectedNode.name}</h3>
                                <p className="text-[10px] text-muted-foreground font-mono truncate mt-0.5">{selectedNode.contract_id}</p>
                            </div>
                            <button
                                onClick={() => setSelectedNode(null)}
                                className="text-muted-foreground hover:text-foreground transition-colors shrink-0 p-1 rounded hover:bg-accent focus-visible:ring-1 focus-visible:ring-primary focus:outline-none"
                                aria-label="Close panel"
                            >
                                <X className="w-4 h-4" />
                            </button>
                        </div>
                    </div>

                    <div className="grid grid-cols-3 gap-px bg-border">
                        <div className="bg-card p-2.5 text-center">
                            <div className="text-lg font-bold text-foreground">{dependentCounts.get(selectedNode.id) || 0}</div>
                            <div className="text-[10px] text-muted-foreground">{t('graph.dependents')}</div>
                        </div>
                        <div className="bg-card p-2.5 text-center">
                            <div className="text-lg font-bold text-foreground">{dependencyCounts.get(selectedNode.id) || 0}</div>
                            <div className="text-[10px] text-muted-foreground">{t('graph.dependencies')}</div>
                        </div>
                        <div className="bg-card p-2.5 text-center">
                            <div className={`text-sm font-bold ${selectedNode.is_verified ? 'text-green-500' : 'text-muted-foreground'}`}>
                                {selectedNode.is_verified ? `✓ ${t('common.yes', 'Yes')}` : '—'}
                            </div>
                            <div className="text-[10px] text-muted-foreground">{t('graph.verified')}</div>
                        </div>
                    </div>

                    <div className="p-4 pt-3 space-y-3">
                        <div className="space-y-1.5 text-sm">
                            <div className="flex justify-between">
                                <span className="text-muted-foreground">{t('graph.network')}</span>
                                <span className={`font-medium px-2 py-0.5 rounded-full text-xs ${
                                    selectedNode.network === 'mainnet' ? 'text-green-600 bg-green-500/10' :
                                    selectedNode.network === 'testnet' ? 'text-blue-600 bg-blue-500/10' :
                                    'text-purple-600 bg-purple-500/10'
                                }`}>{selectedNode.network}</span>
                            </div>
                            {selectedNode.category && (
                                <div className="flex justify-between">
                                    <span className="text-muted-foreground">{t('graph.type')}</span>
                                    <span className="text-foreground font-medium">{selectedNode.category}</span>
                                </div>
                            )}
                        </div>

                        <div className="grid grid-cols-2 gap-2 pt-2">
                            <button
                                onClick={() => handleExpandNode(selectedNode.id)}
                                className={`flex items-center justify-center gap-1.5 py-2 rounded-lg text-xs font-semibold transition-all focus-visible:ring-1 focus:outline-none ${
                                    expandedNodeIds.has(selectedNode.id)
                                        ? 'bg-accent text-foreground border border-border hover:bg-accent/80'
                                        : 'bg-primary/20 text-primary border border-primary/30 hover:bg-primary/30'
                                }`}
                            >
                                <Sparkles className="w-3 h-3" />
                                {expandedNodeIds.has(selectedNode.id) ? 'Collapse' : 'Expand'}
                            </button>
                            <a
                                href={`/contracts/${selectedNode.contract_id}`}
                                className="flex items-center justify-center gap-1.5 py-2 bg-primary text-primary-foreground rounded-lg text-xs font-semibold btn-glow hover:brightness-110 transition-all focus-visible:ring-1 focus-visible:ring-primary focus:outline-none"
                            >
                                <ExternalLink className="w-3 h-3" />
                                Details
                            </a>
                        </div>
                    </div>
                </div>
            )}

            {/* Empty State */}
            {!demoMode && nodes.length === 0 && !isLoading && (
                <div className="absolute inset-0 flex items-center justify-center z-20">
                    <div className="text-center gradient-border-card p-10 max-w-md">
                        <div className="w-14 h-14 rounded-2xl bg-primary/10 flex items-center justify-center mx-auto mb-4">
                            <Sparkles className="w-7 h-7 text-primary" />
                        </div>
                        <h3 className="text-xl font-semibold text-foreground mb-2">{t('graph.noContracts')}</h3>
                        <p className="text-muted-foreground mb-6 text-sm leading-relaxed">
                            {t('graph.noContractsDesc')}
                        </p>
                        <button
                            onClick={() => setDemoMode(true)}
                            className="btn-glow px-6 py-2.5 bg-primary hover:brightness-110 text-primary-foreground rounded-lg font-medium transition-all inline-flex items-center gap-2"
                        >
                            <Sparkles className="w-4 h-4" />
                            Enable Demo Mode
                        </button>
                    </div>
                </div>
            )}
        </div>
    );
}
