'use client';

import { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import type { AnalyticsEventType, AnalyticsEvent } from '@/types';
import { api } from '@/lib/api';
import { 
  History, 
  Search, 
  Filter, 
  Download, 
  Info,
  Calendar,
  CheckCircle2,
  FileCode2,
  AlertCircle,
  X,
  ArrowRight,
  Package,
  ShieldCheck,
  Zap
} from 'lucide-react';

interface ContractTimelineProps {
  contractId: string;
}

export default function ContractTimeline({ contractId }: ContractTimelineProps) {
  const [searchTerm, setSearchTerm] = useState('');
  const [typeFilter, setTypeFilter] = useState<AnalyticsEventType | 'all'>('all');
  const [selectedEvent, setSelectedEvent] = useState<AnalyticsEvent | null>(null);

  const { data, isLoading, error } = useQuery({
    queryKey: ['contract-timeline', contractId],
    queryFn: () => api.getActivityFeed({ contract_id: contractId, limit: 100 }),
  });

  const filteredEvents = useMemo(() => {
    if (!data?.items) return [];
    return data.items.filter(event => {
      const matchesSearch = searchTerm === '' || 
        JSON.stringify(event.metadata).toLowerCase().includes(searchTerm.toLowerCase()) ||
        event.event_type.toLowerCase().includes(searchTerm.toLowerCase());
      const matchesType = typeFilter === 'all' || event.event_type === typeFilter;
      return matchesSearch && matchesType;
    });
  }, [data, searchTerm, typeFilter]);

  const eventIcons: Record<string, React.ReactNode> = {
    contract_published: <Package className="w-4 h-4" />,
    contract_verified: <ShieldCheck className="w-4 h-4 text-green-500" />,
    contract_deployed: <Zap className="w-4 h-4 text-blue-500" />,
    version_created: <FileCode2 className="w-4 h-4 text-purple-500" />,
    contract_updated: <CheckCircle2 className="w-4 h-4 text-orange-500" />,
  };

  const getEventLabel = (type: string) => {
    return type.split('_').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ');
  };

  const handleExport = () => {
    const csvContent = "data:text/csv;charset=utf-8," 
      + "Date,Type,User,Network,Metadata\n"
      + filteredEvents.map(e => `${e.created_at},${e.event_type},${e.user_address || ''},${e.network || ''},"${JSON.stringify(e.metadata).replace(/"/g, '""')}"`).join("\n");
    
    const encodedUri = encodeURI(csvContent);
    const link = document.createElement("a");
    link.setAttribute("href", encodedUri);
    link.setAttribute("download", `contract_${contractId}_history.csv`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 bg-card p-4 rounded-xl border border-border shadow-sm">
        <div className="flex items-center gap-2">
          <History className="w-5 h-5 text-primary" />
          <h3 className="text-lg font-semibold">Contract Timeline</h3>
          <span className="text-xs bg-muted px-2 py-0.5 rounded-full text-muted-foreground">
            {filteredEvents.length} events
          </span>
        </div>
        
        <div className="flex flex-wrap items-center gap-3">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input 
              type="text" 
              placeholder="Search history..." 
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-9 pr-4 py-2 rounded-lg border border-border bg-background text-sm focus:ring-2 focus:ring-primary/20 transition-all w-full md:w-64"
            />
          </div>
          
          <select 
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value as AnalyticsEventType | 'all')}
            className="px-3 py-2 rounded-lg border border-border bg-background text-sm focus:ring-2 focus:ring-primary/20 outline-none"
          >
            <option value="all">All Events</option>
            <option value="contract_published">Published</option>
            <option value="contract_verified">Verified</option>
            <option value="version_created">New Version</option>
            <option value="contract_deployed">Deployed</option>
          </select>

          <button 
            onClick={handleExport}
            className="flex items-center gap-2 px-4 py-2 bg-secondary hover:bg-secondary/80 text-secondary-foreground rounded-lg text-sm font-medium transition-colors"
          >
            <Download className="w-4 h-4" />
            Export
          </button>
        </div>
      </div>

      {isLoading ? (
        <div className="flex flex-col items-center justify-center py-20 gap-4">
          <div className="w-10 h-10 border-4 border-primary border-t-transparent rounded-full animate-spin"></div>
          <p className="text-muted-foreground animate-pulse">Loading interaction history...</p>
        </div>
      ) : error ? (
        <div className="flex flex-col items-center justify-center py-20 bg-red-50 dark:bg-red-950/20 rounded-2xl border border-red-100 dark:border-red-900/30">
          <AlertCircle className="w-12 h-12 text-red-500 mb-4" />
          <p className="text-red-600 dark:text-red-400 font-medium">Failed to load timeline</p>
          <button onClick={() => window.location.reload()} className="mt-4 text-sm underline opacity-70 hover:opacity-100">Try again</button>
        </div>
      ) : filteredEvents.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center bg-muted/30 rounded-2xl border border-dashed border-border">
          <Calendar className="w-12 h-12 text-muted-foreground/30 mb-4" />
          <h4 className="text-lg font-medium text-muted-foreground">No events found</h4>
          <p className="text-sm text-muted-foreground/60 max-w-xs mx-auto">Try adjusting your filters or search term to find what you&apos;re looking for.</p>
        </div>
      ) : (
        <div className="relative pl-8 space-y-8 before:absolute before:left-[11px] before:top-2 before:bottom-2 before:w-0.5 before:bg-gradient-to-b before:from-primary/50 before:to-transparent">
          {filteredEvents.map((event, index) => (
            <div key={event.id} className="relative group">
              <div className={`absolute -left-8 top-1 w-6 h-6 rounded-full border-4 border-background shadow-sm flex items-center justify-center z-10 transition-transform group-hover:scale-110 ${index === 0 ? 'bg-primary' : 'bg-muted-foreground/20'}`}>
                <div className={`w-1.5 h-1.5 rounded-full ${index === 0 ? 'bg-primary-foreground' : 'bg-muted-foreground'}`}></div>
              </div>
              
              <div className="bg-card hover:bg-accent/30 p-4 rounded-xl border border-border shadow-sm transition-all hover:shadow-md cursor-pointer" onClick={() => setSelectedEvent(event)}>
                <div className="flex items-start justify-between gap-4">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="p-1.5 rounded-md bg-background border border-border">
                        {eventIcons[event.event_type] || <Info className="w-4 h-4" />}
                      </span>
                      <h4 className="font-bold text-foreground">
                        {getEventLabel(event.event_type)}
                      </h4>
                      {event.network && (
                        <span className="text-[10px] uppercase tracking-wider font-bold px-1.5 py-0.5 rounded bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300">
                          {event.network}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-3 text-xs text-muted-foreground">
                      <span className="flex items-center gap-1">
                        <Calendar className="w-3 h-3" />
                        {new Date(event.created_at).toLocaleString(undefined, { 
                          dateStyle: 'medium', 
                          timeStyle: 'short' 
                        })}
                      </span>
                      {event.user_address && (
                        <span className="flex items-center gap-1 font-mono">
                          <ArrowRight className="w-3 h-3 opacity-50" />
                          {event.user_address.slice(0, 4)}...{event.user_address.slice(-4)}
                        </span>
                      )}
                    </div>
                  </div>
                  
                  <button className="p-2 hover:bg-background rounded-full transition-colors text-muted-foreground">
                    <Info className="w-4 h-4" />
                  </button>
                </div>
                
                {event.metadata && (
                  <div className="mt-3 text-sm text-muted-foreground line-clamp-1 italic bg-muted/20 p-2 rounded">
                    {JSON.stringify(event.metadata)}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Details Modal */}
      {selectedEvent && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm transition-opacity" onClick={() => setSelectedEvent(null)}>
          <div className="bg-card w-full max-w-2xl rounded-2xl shadow-2xl border border-border overflow-hidden animate-in fade-in zoom-in duration-200" onClick={e => e.stopPropagation()}>
            <div className="p-6 border-b border-border flex items-center justify-between bg-muted/30">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-xl bg-background border border-border shadow-inner">
                  {eventIcons[selectedEvent.event_type] || <Info className="w-6 h-6" />}
                </div>
                <div>
                  <h3 className="text-xl font-bold">{getEventLabel(selectedEvent.event_type)}</h3>
                  <p className="text-sm text-muted-foreground">Detailed event information</p>
                </div>
              </div>
              <button onClick={() => setSelectedEvent(null)} className="p-2 hover:bg-background rounded-full transition-colors">
                <X className="w-5 h-5" />
              </button>
            </div>
            
            <div className="p-6 space-y-6 max-height-[70vh] overflow-y-auto">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="space-y-1 bg-muted/20 p-3 rounded-lg border border-border">
                  <span className="text-[10px] uppercase font-bold text-muted-foreground tracking-widest">Event ID</span>
                  <p className="font-mono text-sm break-all">{selectedEvent.id}</p>
                </div>
                <div className="space-y-1 bg-muted/20 p-3 rounded-lg border border-border">
                  <span className="text-[10px] uppercase font-bold text-muted-foreground tracking-widest">Timestamp</span>
                  <p className="text-sm">{new Date(selectedEvent.created_at).toLocaleString()}</p>
                </div>
                <div className="space-y-1 bg-muted/20 p-3 rounded-lg border border-border">
                  <span className="text-[10px] uppercase font-bold text-muted-foreground tracking-widest">Network</span>
                  <p className="text-sm">{selectedEvent.network || 'N/A'}</p>
                </div>
                <div className="space-y-1 bg-muted/20 p-3 rounded-lg border border-border">
                  <span className="text-[10px] uppercase font-bold text-muted-foreground tracking-widest">Initiator</span>
                  <p className="font-mono text-sm break-all">{selectedEvent.user_address || 'System'}</p>
                </div>
              </div>

              <div className="space-y-2">
                <span className="text-[10px] uppercase font-bold text-muted-foreground tracking-widest">Event Metadata</span>
                <pre className="bg-black text-green-400 p-4 rounded-xl overflow-x-auto text-xs font-mono border border-border shadow-inner">
                  {JSON.stringify(selectedEvent.metadata, null, 2)}
                </pre>
              </div>
            </div>
            
            <div className="p-4 bg-muted/30 border-t border-border flex justify-end">
              <button onClick={() => setSelectedEvent(null)} className="px-6 py-2 bg-primary text-primary-foreground rounded-lg font-medium hover:bg-primary/90 transition-colors">
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
