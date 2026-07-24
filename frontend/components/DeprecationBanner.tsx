'use client';

import Link from 'next/link';
import type { DeprecationInfo } from '@/types';
import { AlertTriangle, ArrowUpRight } from 'lucide-react';

function formatDate(value?: string | null) {
  if (!value) return '—';
  return new Date(value).toLocaleDateString();
}

function formatCountdown(days?: number | null) {
  if (days === null || days === undefined) return '—';
  if (days <= 0) return '0 days';
  return `${days} days`;
}

type Props = {
  info: DeprecationInfo;
};

export default function DeprecationBanner({ info }: Props) {
  if (info.status === 'active') return null;

  const isRetired = info.status === 'retired';
  const badge = isRetired ? 'Retired' : 'Deprecated';
  const headline = isRetired
    ? 'This contract has been retired.'
    : 'This contract is deprecated and scheduled for retirement.';

  return (
    <div className="rounded-xl border border-amber-200 bg-amber-50 text-amber-900 px-5 py-4">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex items-start gap-3">
          <div className="mt-1">
            <AlertTriangle className="h-5 w-5" />
          </div>
          <div>
            <div className="inline-flex items-center gap-2 text-xs font-semibold uppercase tracking-wide">
              <span className="rounded-full bg-amber-200 px-2 py-1">{badge}</span>
              <span>Retires {formatDate(info.retirement_at)}</span>
            </div>
            <h3 className="text-base font-semibold mt-2">{headline}</h3>
            {info.notes ? (
              <p className="text-sm mt-1 text-amber-800">{info.notes}</p>
            ) : null}
            <div className="mt-3 text-sm text-amber-800 flex flex-wrap gap-4">
              <span>Countdown: {formatCountdown(info.days_remaining)}</span>
              <span>Dependents notified: {info.dependents_notified}</span>
            </div>
          </div>
        </div>

        <div className="flex flex-col gap-2">
          {info.replacement_contract_id ? (
            <Link
              href={`/contracts/${info.replacement_contract_id}`}
              className="inline-flex items-center gap-2 rounded-md bg-amber-200 px-3 py-2 text-sm font-medium text-amber-900 hover:bg-amber-300 transition-colors"
            >
              View replacement
              <ArrowUpRight className="h-4 w-4" />
            </Link>
          ) : null}
          {info.migration_guide_url ? (
            <a
              href={info.migration_guide_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-md border border-amber-300 px-3 py-2 text-sm font-medium text-amber-900 hover:bg-amber-100 transition-colors"
            >
              Migration guide
              <ArrowUpRight className="h-4 w-4" />
            </a>
          ) : null}
        </div>
      </div>
    </div>
  );
}
