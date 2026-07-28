"use client";

import { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api, AnalyticsEvent, AnalyticsEventType, ActivityFeedResponse } from '@/lib/api';
import { ContractDeploymentEvent, ContractUpdateEvent } from '@/types/realtime';
import { useRealtime } from '@/hooks/useRealtime';
import { formatPublicKey, formatShortenedText } from '@/lib/utils/formatting';
import {
  Activity,
  Upload,
  CheckCircle2,
  RefreshCcw,
  UserPlus,
  ChevronDown,
  ExternalLink,
  Filter,
  Clock,
  Zap,
  Tag,
  type LucideIcon,
} from 'lucide-react';
import type { TFunction } from 'i18next';
import Link from 'next/link';
import { useTranslation } from '@/lib/i18n/client';

const getEventConfig = (t: TFunction): Record<string, { icon: LucideIcon; label: string; color: string }> => ({
  contract_published: { icon: Upload, label: t('activityFeed.published'), color: 'text-blue-500 bg-blue-500/10' },
  contract_verified: { icon: CheckCircle2, label: t('activityFeed.verified'), color: 'text-emerald-500 bg-emerald-500/10' },
  contract_deployed: { icon: Zap, label: t('activityFeed.deployed'), color: 'text-amber-500 bg-amber-500/10' },
  version_created: { icon: RefreshCcw, label: t('activityFeed.newVersion'), color: 'text-purple-500 bg-purple-500/10' },
  contract_updated: { icon: Tag, label: t('activityFeed.updated'), color: 'text-indigo-500 bg-indigo-500/10' },
  publisher_created: { icon: UserPlus, label: t('activityFeed.newPublisher', 'New Publisher'), color: 'text-pink-500 bg-pink-500/10' },
});

export default function ActivityFeed() {
  const { t } = useTranslation('common');
  const EVENT_CONFIG = getEventConfig(t);
  const { subscribe, isConnected } = useRealtime();

  const [eventType, setEventType] = useState<AnalyticsEventType | 'all'>('all');
  const [items, setItems] = useState<AnalyticsEvent[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [isFetchingMore, setIsFetchingMore] = useState(false);

  // Initial fetch
  const { isLoading, error, data } = useQuery<ActivityFeedResponse>({
    queryKey: ['activity-feed', eventType],
    queryFn: () => api.getActivityFeed({
      event_type: eventType === 'all' ? undefined : eventType,
      limit: 20,
    }),
  });

  // Update items when data changes
  useEffect(() => {
    if (data) {
      setItems(data.items);
      setNextCursor(data.next_cursor);
    }
  }, [data]);

  // Handle real-time events
  useEffect(() => {
    const handleDeployment = (event: ContractDeploymentEvent) => {
      const newEvent: AnalyticsEvent = {
        id: Math.random().toString(36).substring(7),
        event_type: 'contract_deployed',
        contract_id: event.contractId,
        user_address: event.publisher,
        network: null,
        metadata: { name: event.contractName, version: event.version },
        created_at: event.timestamp,
      };
      if (eventType === 'all' || eventType === 'contract_deployed') {
        setItems(prev => [newEvent, ...prev].slice(0, 100));
      }
    };

    const handleUpdate = (event: ContractUpdateEvent) => {
      const newEvent: AnalyticsEvent = {
        id: Math.random().toString(36).substring(7),
        event_type: 'contract_updated',
        contract_id: event.contractId,
        user_address: null,
        network: null,
        metadata: { update_type: event.updateType, ...event.details },
        created_at: event.timestamp,
      };
      if (eventType === 'all' || eventType === 'contract_updated') {
        setItems(prev => [newEvent, ...prev].slice(0, 100));
      }
    };

    const unsubDeploy = subscribe('contract_deployed', (data: unknown) => handleDeployment(data as ContractDeploymentEvent));
    const unsubUpdate = subscribe('contract_updated', (data: unknown) => handleUpdate(data as ContractUpdateEvent));

    return () => {
      unsubDeploy();
      unsubUpdate();
    };
  }, [subscribe, eventType]);

  const fetchMore = async () => {
    if (!nextCursor || isFetchingMore) return;
    setIsFetchingMore(true);
    try {
      const res = await api.getActivityFeed({
        cursor: nextCursor,
        event_type: eventType === 'all' ? undefined : eventType,
        limit: 20,
      });
      setItems(prev => [...prev, ...res.items]);
      setNextCursor(res.next_cursor);
    } catch (err) {
      console.error('Failed to fetch more activity:', err);
    } finally {
      setIsFetchingMore(false);
    }
  };

  const formatAddress = (addr: string | null) => {
    if (!addr) return t('common.unknown', 'Unknown');
    return formatPublicKey(addr);
  };

  const formatTime = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    if (diff < 60000) return t('activityFeed.justNow');
    if (diff < 3600000) return `${Math.floor(diff / 60000)}${t('activityFeed.m_ago')}`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}${t('activityFeed.h_ago')}`;
    return date.toLocaleDateString();
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Activity className="w-6 h-6 text-primary" />
          <h2 className="text-xl font-bold text-foreground">{t('activityFeed.title')}</h2>
          {isConnected && (
            <span className="flex h-2 w-2 rounded-full bg-emerald-500 animate-pulse ml-1" title={t('activityFeed.liveUpdates')} />
          )}
        </div>

        <div className="flex items-center gap-2 bg-card border border-border rounded-lg px-3 py-1.5 shadow-sm">
          <Filter className="w-4 h-4 text-muted-foreground" />
          <select
            value={eventType}
            onChange={(e) => setEventType(e.target.value as AnalyticsEventType | 'all')}
            className="bg-transparent text-sm font-medium text-foreground focus:outline-none cursor-pointer"
          >
            <option value="all">{t('activityFeed.allEvents')}</option>
            <option value="contract_published">{t('activityFeed.published')}</option>
            <option value="contract_verified">{t('activityFeed.verified')}</option>
            <option value="contract_deployed">{t('activityFeed.deployed')}</option>
            <option value="version_created">{t('activityFeed.newVersion')}</option>
            <option value="contract_updated">{t('activityFeed.updated')}</option>
          </select>
        </div>
      </div>

      <div className="bg-card border border-border rounded-xl overflow-hidden shadow-sm">
        {isLoading && items.length === 0 ? (
          <div className="p-12 flex flex-col items-center justify-center text-muted-foreground gap-3">
            <RefreshCcw className="w-8 h-8 animate-spin text-primary/40" />
            <p className="text-sm">{t('activityFeed.loading')}</p>
          </div>
        ) : error ? (
          <div className="p-12 text-center text-red-500">
            <p>{t('activityFeed.failed')}</p>
          </div>
        ) : items.length === 0 ? (
          <div className="p-12 text-center text-muted-foreground">
            <p>{t('activityFeed.noActivity')}</p>
          </div>
        ) : (
          <div className="divide-y divide-border">
            {items.map((item) => {
              const config = EVENT_CONFIG[item.event_type] || {
                icon: Activity,
                label: item.event_type,
                color: 'text-gray-500 bg-gray-500/10',
              };
              const Icon = config.icon;
              const metadataName = typeof item.metadata?.name === 'string' ? item.metadata.name : null;
              const metadataVersion =
                typeof item.metadata?.version === 'string' || typeof item.metadata?.version === 'number'
                  ? item.metadata.version
                  : null;
              const metadataUpdateType = typeof item.metadata?.update_type === 'string' ? item.metadata.update_type : null;

              return (
                <div key={item.id} className="p-4 hover:bg-muted/30 transition-colors group">
                  <div className="flex gap-4">
                    <div className={`mt-1 p-2 rounded-full h-fit ${config.color}`}>
                      <Icon className="w-4 h-4" />
                    </div>

                    <div className="flex-1 flex flex-col gap-1">
                      <div className="flex items-center justify-between gap-2">
                        <div className="flex flex-wrap items-center gap-x-2 text-sm">
                          <span className="font-semibold text-foreground uppercase text-[10px] tracking-wider px-1.5 py-0.5 rounded border border-border bg-muted/50">
                            {config.label}
                          </span>
                          <Link
                            href={`/contracts/${item.contract_id}`}
                            className="font-medium text-primary hover:underline flex items-center gap-1"
                          >
                            {metadataName || formatShortenedText(item.contract_id, 10, '...')}
                            <ExternalLink className="w-3 h-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                          </Link>
                          {metadataVersion && (
                            <span className="text-muted-foreground">v{metadataVersion}</span>
                          )}
                        </div>
                        <div className="flex items-center gap-1 text-xs text-muted-foreground whitespace-nowrap">
                          <Clock className="w-3 h-3" />
                          {formatTime(item.created_at)}
                        </div>
                      </div>

                      <div className="text-sm text-muted-foreground flex flex-wrap items-center gap-x-2">
                        {item.user_address && (
                          <>
                            <span>{t('activityFeed.by')}</span>
                            <span className="font-mono text-foreground bg-muted px-1 rounded text-[11px]">
                              {formatAddress(item.user_address)}
                            </span>
                          </>
                        )}
                        {item.network && (
                          <span className="capitalize px-1.5 py-0.5 rounded-full bg-primary/5 text-primary text-[10px] font-bold">
                            {item.network}
                          </span>
                        )}
                      </div>

                      {item.event_type === 'contract_updated' && metadataUpdateType && (
                        <div className="mt-1 text-xs px-2 py-1 rounded bg-muted/50 border border-border inline-block w-fit">
                          <span className="font-medium">{t('activityFeed.type')}:</span>{' '}
                          {metadataUpdateType}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {nextCursor && (
        <button
          onClick={fetchMore}
          disabled={isFetchingMore}
          className="flex items-center justify-center gap-2 py-2 px-4 rounded-lg border border-border hover:bg-muted font-medium text-sm transition-colors text-foreground disabled:opacity-50"
        >
          {isFetchingMore ? (
            <RefreshCcw className="w-4 h-4 animate-spin" />
          ) : (
            <ChevronDown className="w-4 h-4" />
          )}
          {t('activityFeed.loadMore')}
        </button>
      )}
    </div>
  );
}
