"use client";

import React, { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { AnalyticsEvent } from "@/lib/api";
// Simple relative time formatter replacing date-fns
function formatDistanceToNow(date: Date): string {
  const now = new Date();
  const diffInSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);
  
  if (diffInSeconds < 60) return 'just now';
  if (diffInSeconds < 3600) return `${Math.floor(diffInSeconds / 60)} minutes ago`;
  if (diffInSeconds < 86400) return `${Math.floor(diffInSeconds / 3600)} hours ago`;
  if (diffInSeconds < 604800) return `${Math.floor(diffInSeconds / 86400)} days ago`;
  return date.toLocaleDateString();
}
import { 
  GitCommit, 
  ShieldCheck, 
  FileCode, 
  Settings,
  AlertCircle,
  CheckCircle2
} from "lucide-react";

interface ContractTimelineProps {
  contractId: string;
}

export function ContractTimeline({ contractId }: ContractTimelineProps) {
  const [events, setEvents] = useState<AnalyticsEvent[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadHistory() {
      try {
        const response = await api.getActivityFeed({ contract_id: contractId, limit: 50 });
        setEvents(response.items);
      } catch (error) {
        console.error("Failed to load timeline:", error);
      } finally {
        setLoading(false);
      }
    }
    loadHistory();
  }, [contractId]);

  if (loading) return <div className="animate-pulse space-y-4">
    {[1, 2, 3].map(i => <div key={i} className="h-20 bg-muted rounded-lg" />)}
  </div>;

  if (events.length === 0) return (
    <div className="text-center py-12 border rounded-lg border-dashed">
      <p className="text-muted-foreground text-sm">No interaction history found for this contract.</p>
    </div>
  );

  const getEventIcon = (type: string) => {
    switch (type) {
      case 'contract_published': return <FileCode className="h-4 w-4 text-blue-500" />;
      case 'contract_verified': return <ShieldCheck className="h-4 w-4 text-green-500" />;
      case 'version_created': return <GitCommit className="h-4 w-4 text-purple-500" />;
      case 'security_scan_completed': return <CheckCircle2 className="h-4 w-4 text-emerald-500" />;
      default: return <Settings className="h-4 w-4 text-gray-500" />;
    }
  };

  return (
    <div className="relative space-y-8 before:absolute before:inset-0 before:ml-5 before:h-full before:w-0.5 before:bg-gradient-to-b before:from-transparent before:via-border before:to-transparent">
      {events.map((event) => (
        <div key={event.id} className="relative flex items-start gap-6">
          <div className="absolute left-0 mt-1 flex h-10 w-10 items-center justify-center rounded-full bg-background border shadow-sm ring-8 ring-background">
            {getEventIcon(event.event_type)}
          </div>
          <div className="flex-1 ml-12 pt-1">
            <div className="flex items-center justify-between gap-2">
              <h4 className="text-sm font-semibold capitalize">
                {event.event_type.replace(/_/g, ' ')}
              </h4>
              <time className="text-xs text-muted-foreground whitespace-nowrap">
                {formatDistanceToNow(new Date(event.created_at))}
              </time>
            </div>
            <p className="mt-1 text-sm text-muted-foreground">
              {(event.metadata?.message as string) || `Contract ${event.event_type.split('_')[1] || 'event'} recorded.`}
            </p>
          </div>
        </div>
      ))}
    </div>
  );
}
