'use client';

import React, { useState, useMemo, useEffect, useRef } from 'react';
import type { Contract, Network } from '@/types';
import { api } from '@/lib/api';
import { 
  Calendar, 
  Info, 
  ChevronLeft, 
  ChevronRight, 
  Clock, 
  ExternalLink,
  Zap
} from 'lucide-react';
import Link from 'next/link';

interface DeploymentTimelineProps {
  initialData?: Contract[];
}

const NETWORK_COLORS: Record<Network, string> = {
  mainnet: '#06b6d4', // cyan-500
  testnet: '#8b5cf6', // violet-500
  futurenet: '#f59e0b', // amber-500
};

export default function DeploymentTimeline({ initialData = [] }: DeploymentTimelineProps) {
  const [contracts, setContracts] = useState<Contract[]>(initialData);
  const [loading, setLoading] = useState(initialData.length === 0);
  const [activeNetworks, setActiveNetworks] = useState<Network[]>(['mainnet', 'testnet', 'futurenet']);
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const [viewWindow, setViewWindow] = useState<'7d' | '30d' | '90d' | 'all'>('30d');
  
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    async function fetchDeployments() {
      if (initialData.length > 0) return;
      
      setLoading(true);
      try {
        const response = await api.getContracts({
          sort_by: 'created_at',
          sort_order: 'desc',
          page_size: 100
        });
        setContracts(response.items);
      } catch (err) {
        console.error('Failed to fetch deployments for timeline:', err);
      } finally {
        setLoading(false);
      }
    }
    
    fetchDeployments();
  }, [initialData]);

  // Filtering and windowing
  const filteredContracts = useMemo(() => {
    let filtered = contracts.filter(c => activeNetworks.includes(c.network));
    
    if (viewWindow !== 'all') {
      const now = new Date();
      const days = viewWindow === '7d' ? 7 : viewWindow === '30d' ? 30 : 90;
      const cutoff = new Date(now.getTime() - days * 24 * 60 * 60 * 1000);
      filtered = filtered.filter(c => new Date(c.created_at) >= cutoff);
    }
    
    return filtered.sort((a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime());
  }, [contracts, activeNetworks, viewWindow]);

  // Initial scroll to end (latest)
  useEffect(() => {
    if (scrollContainerRef.current && filteredContracts.length > 0) {
      setTimeout(() => {
        if (scrollContainerRef.current) {
          scrollContainerRef.current.scrollTo({
            left: scrollContainerRef.current.scrollWidth,
            behavior: 'smooth'
          });
        }
      }, 500);
    }
  }, [filteredContracts.length]);

  const latestId = useMemo(() => {
    if (contracts.length === 0) return null;
    return [...contracts].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())[0].id;
  }, [contracts]);

  const timeBounds = useMemo(() => {
    if (filteredContracts.length === 0) return { min: 0, max: 0 };
    const timestamps = filteredContracts.map(c => new Date(c.created_at).getTime());
    if (filteredContracts.length === 0) return { min: 0, max: Date.now() };
    return {
      min: Math.min(...timestamps),
      max: Math.max(...timestamps)
    };
  }, [filteredContracts]);

  const toggleNetwork = (network: Network) => {
    setActiveNetworks(prev => 
      prev.includes(network) 
        ? prev.filter(n => n !== network) 
        : [...prev, network]
    );
  };

  const scroll = (direction: 'left' | 'right') => {
    if (scrollContainerRef.current) {
      const scrollAmount = 400;
      scrollContainerRef.current.scrollBy({
        left: direction === 'left' ? -scrollAmount : scrollAmount,
        behavior: 'smooth'
      });
    }
  };

  if (loading && contracts.length === 0) {
    return (
      <div className="w-full h-64 flex items-center justify-center bg-card/50 rounded-2xl border border-border animate-pulse">
        <div className="flex flex-col items-center gap-3">
          <Clock className="w-8 h-8 text-muted-foreground animate-spin" />
          <span className="text-sm font-medium text-muted-foreground">Generating timeline...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="group relative w-full bg-card rounded-2xl border border-border shadow-sm overflow-hidden flex flex-col transition-all duration-300 hover:shadow-md">
      {/* Header controls */}
      <div className="px-6 py-4 border-b border-border flex flex-col md:flex-row md:items-center justify-between gap-4 bg-muted/20">
        <div className="flex items-center gap-2">
          <div className="p-2 bg-primary/10 rounded-lg text-primary">
            <Zap className="w-5 h-5" />
          </div>
          <div>
            <h3 className="text-lg font-bold text-foreground">Deployment Timeline</h3>
            <p className="text-xs text-muted-foreground">Historical view of registry submissions</p>
          </div>
        </div>

        <div className="flex items-center flex-wrap gap-2">
          {/* Network Toggles */}
          <div className="flex items-center p-1 bg-background rounded-xl border border-border">
            {(['mainnet', 'testnet', 'futurenet'] as Network[]).map(net => (
              <button
                key={net}
                onClick={() => toggleNetwork(net)}
                className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                  activeNetworks.includes(net)
                    ? 'bg-secondary text-secondary-foreground shadow-sm'
                    : 'text-muted-foreground hover:bg-muted'
                }`}
              >
                {net.charAt(0).toUpperCase() + net.slice(1)}
              </button>
            ))}
          </div>

          {/* Time Window Select */}
          <select 
            value={viewWindow}
            onChange={(e) => setViewWindow(e.target.value as '7d'|'30d'|'90d'|'all')}
            className="bg-background border border-border rounded-xl px-3 py-1.5 text-xs font-semibold text-foreground focus:ring-2 focus:ring-primary outline-none cursor-pointer"
          >
            <option value="7d">Last 7 Days</option>
            <option value="30d">Last 30 Days</option>
            <option value="90d">Last 90 Days</option>
            <option value="all">Recent 100</option>
          </select>
          
          <button
            onClick={() => {
                if (scrollContainerRef.current) {
                    scrollContainerRef.current.scrollTo({
                        left: scrollContainerRef.current.scrollWidth,
                        behavior: 'smooth'
                    });
                }
            }}
            className="p-1.5 bg-primary/10 hover:bg-primary/20 text-primary rounded-xl transition-colors border border-primary/20"
            title="Go to Latest"
          >
            <ChevronRight className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Timeline container */}
      <div className="relative flex-1 min-h-[180px] p-8 overflow-hidden">
        {/* Navigation Arrows (Desktop) */}
        <button 
          onClick={() => scroll('left')}
          className="absolute left-4 top-1/2 -translate-y-1/2 z-20 p-2 rounded-full bg-background border border-border shadow-lg text-foreground transition-opacity hover:bg-muted opacity-0 group-hover:opacity-100 hidden md:flex"
        >
          <ChevronLeft className="w-5 h-5" />
        </button>
        <button 
          onClick={() => scroll('right')}
          className="absolute right-4 top-1/2 -translate-y-1/2 z-20 p-2 rounded-full bg-background border border-border shadow-lg text-foreground transition-opacity hover:bg-muted opacity-0 group-hover:opacity-100 hidden md:flex"
        >
          <ChevronRight className="w-5 h-5" />
        </button>

        {/* Scrollable area */}
        <div 
          ref={scrollContainerRef}
          className="w-full h-full overflow-x-auto no-scrollbar py-12 px-12"
        >
          <div 
            className="relative h-1 min-w-[1200px] bg-gradient-to-r from-transparent via-border to-transparent self-center top-1/2 -translate-y-1/2"
            style={{ width: filteredContracts.length * 40 + 'px' }}
          >
            {/* Markers */}
            {filteredContracts.map((contract) => {
              const timestamp = new Date(contract.created_at).getTime();
              const range = timeBounds.max - timeBounds.min || 1;
              const percent = ((timestamp - timeBounds.min) / range) * 100;
              const isLatest = contract.id === latestId;
              const color = NETWORK_COLORS[contract.network];

              return (
                <div 
                  key={contract.id}
                  className="absolute top-1/2 -translate-y-1/2"
                  style={{ left: `${percent}%` }}
                  onMouseEnter={() => setHoveredId(contract.id)}
                  onMouseLeave={() => setHoveredId(null)}
                >
                  {/* Point */}
                  <div 
                    className={`relative w-4 h-4 rounded-full border-2 border-background cursor-pointer transition-transform duration-300 hover:scale-150 shadow-sm z-10 ${isLatest ? 'animate-pulse ring-4 ring-primary/20' : ''}`}
                    style={{ backgroundColor: color }}
                  />

                  {/* Stalk (vertical line) */}
                  <div 
                    className="absolute bottom-4 left-1/2 -translate-x-1/2 w-px h-6 bg-border"
                  />

                  {/* Short Date Label (Top) */}
                  <div className="absolute bottom-10 left-1/2 -translate-x-1/2 whitespace-nowrap text-[10px] font-mono font-bold text-muted-foreground">
                    {new Date(contract.created_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}
                  </div>

                  {/* Tooltip (Hover) */}
                  <div 
                    className={`absolute bottom-full left-1/2 -translate-x-1/2 mb-16 w-64 p-4 rounded-xl glass shadow-2xl border border-primary/20 transition-all duration-300 pointer-events-none z-50 ${
                      hoveredId === contract.id ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'
                    }`}
                  >
                    <div className="flex flex-col gap-2">
                      <div className="flex items-center justify-between">
                        <span className="px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider text-white" style={{ backgroundColor: color }}>
                          {contract.network}
                        </span>
                        {isLatest && (
                          <span className="flex items-center gap-1 text-[10px] font-bold text-primary animate-pulse">
                            <Zap className="w-3 h-3 fill-primary" /> NEW
                          </span>
                        )}
                      </div>
                      
                      <h4 className="text-sm font-bold text-foreground truncate">{contract.name}</h4>
                      
                      <p className="text-[10px] font-mono text-muted-foreground break-all bg-muted/30 p-1.5 rounded">
                        {contract.contract_id}
                      </p>

                      <div className="flex items-center justify-between mt-1 pt-2 border-t border-border/50">
                        <div className="flex items-center gap-1 text-[10px] text-muted-foreground">
                          <Calendar className="w-3 h-3" />
                          {new Date(contract.created_at).toLocaleString()}
                        </div>
                        <Link 
                          href={`/contracts/${contract.id}`}
                          className="p-1 rounded-md bg-primary/10 text-primary hover:bg-primary/20 transition-colors pointer-events-auto"
                        >
                          <ExternalLink className="w-3 h-3" />
                        </Link>
                      </div>
                    </div>
                    {/* Tooltip Arrow */}
                    <div className="absolute top-full left-1/2 -translate-x-1/2 border-8 border-transparent border-t-card/40 backdrop-blur-md" />
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
      
      {/* Legend / Footer */}
      <div className="px-6 py-3 border-t border-border bg-muted/10 flex items-center justify-between gap-4">
        <div className="flex items-center gap-4">
          {(['mainnet', 'testnet', 'futurenet'] as Network[]).map(net => (
            <div key={net} className="flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full" style={{ backgroundColor: NETWORK_COLORS[net] }} />
              <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-tight">{net}</span>
            </div>
          ))}
        </div>
        
        <div className="text-[10px] font-medium text-muted-foreground italic flex items-center gap-1">
          <Info className="w-3 h-3" />
          Scroll horizontally for history
        </div>
      </div>
    </div>
  );
}
