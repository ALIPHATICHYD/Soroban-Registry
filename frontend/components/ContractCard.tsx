import type { Contract } from '@/types';
import {
  Box,
  Check,
  CheckCircle2,
  Clock,
  Copy,
  ExternalLink,
  Eye,
  Flame,
  Layers3,
  RefreshCw,
  Sparkles,
  Tag,
  Star,
} from 'lucide-react';
import Image from 'next/image';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import React, { useState } from 'react';
import { useAnalytics } from '@/hooks/useAnalytics';
import { useCopy } from '@/hooks/useCopy';
import { formatContractId } from '@/lib/utils/formatting';
import { useTranslation } from '@/lib/i18n/client';
import { generateSolidPlaceholder } from '@/lib/images';
import VerificationBadge from '@/components/verification/VerificationBadge';
import HealthWidget from './HealthWidget';
import ContractQuickViewModal from './contracts/ContractQuickViewModal';
import FavoriteButton from './FavoriteButton';

const LOGO_SIZE_PX = 40;
const LOGO_PLACEHOLDER = generateSolidPlaceholder('#e5e7eb');

interface ContractCardProps {
  contract: Contract;
  sortBy?: 'created_at' | 'updated_at' | 'popularity' | 'relevance';
}

const SORT_ICON_META = {
  created_at: { icon: Clock, label: 'Sorted by newest' },
  updated_at: { icon: RefreshCw, label: 'Sorted by last updated' },
  popularity: { icon: Flame, label: 'Sorted by popularity' },
  relevance: { icon: Sparkles, label: 'Sorted by relevance' },
} as const;

export default function ContractCard({ contract, sortBy }: ContractCardProps) {
  const { t } = useTranslation('common');
  const { logEvent } = useAnalytics();
  const router = useRouter();
  const { copy, copied, isCopying } = useCopy();
  const [quickViewOpen, setQuickViewOpen] = React.useState(false);
  const [logoError, setLogoError] = React.useState(false);

  const networkColors = {
    mainnet: 'bg-green-500/10 text-green-500 border-green-500/20',
    testnet: 'bg-blue-500/10 text-blue-500 border-blue-500/20',
    futurenet: 'bg-purple-500/10 text-purple-500 border-purple-500/20',
  };
  const networkDots = {
    mainnet: 'bg-green-400',
    testnet: 'bg-blue-400',
    futurenet: 'bg-purple-400',
  };

  const address = formatContractId(contract.contract_id);
  const categoryLabel = contract.category || 'uncategorized';
  const deploymentCount =
    typeof (contract as Contract & { deployment_count?: number }).deployment_count === 'number'
      ? (contract as Contract & { deployment_count?: number }).deployment_count
      : typeof (contract as Contract & { deployments?: number }).deployments === 'number'
        ? (contract as Contract & { deployments?: number }).deployments
        : '—';

  const handleViewDetails = (event: React.MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    router.push(`/contracts/${contract.id}`);
  };

  const handleOpenQuickView = (event: React.MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    setQuickViewOpen(true);
    logEvent('contract_quick_view_opened', {
      contract_id: contract.id,
      contract_name: contract.name,
      network: contract.network,
    });
  };

  const copyAddress = async () => {
    await copy(contract.contract_id, {
      successEventName: 'contract_address_copied',
      failureEventName: 'contract_address_copy_failed',
      successMessage: 'Contract address copied',
      failureMessage: 'Unable to copy contract address',
      analyticsParams: {
        contract_id: contract.id,
        contract_name: contract.name,
      },
    });
  };

  const handleCopyAddress = async (event: React.MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    await copyAddress();
  };

  return (
    <>
      <Link
        href={`/contracts/${contract.id}`}
        onClick={() =>
          logEvent('contract_viewed', {
            contract_id: contract.id,
            contract_name: contract.name,
            network: contract.network,
          })
        }
      >
        <div className="group relative h-full overflow-hidden rounded-2xl border border-border bg-card transition-all card-hover glow-border gradient-border-card">
          <div className="absolute inset-0 bg-linear-to-br from-primary/5 to-secondary/5 opacity-0 transition-opacity group-hover:opacity-100" />

          <div className="relative flex h-full flex-col p-6">
            {sortBy && SORT_ICON_META[sortBy] && (
              <span
                className="absolute right-3 top-3 inline-flex items-center gap-1 rounded-full border border-border bg-background/95 px-2 py-1 text-[10px] font-semibold text-muted-foreground"
                aria-label={SORT_ICON_META[sortBy].label}
                title={SORT_ICON_META[sortBy].label}
              >
                {React.createElement(SORT_ICON_META[sortBy].icon, { className: 'h-3 w-3' })}
              </span>
            )}

            <div className="mb-3 flex items-start justify-between gap-3">
              <div
                className="relative flex shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border bg-accent"
                style={{ width: LOGO_SIZE_PX, height: LOGO_SIZE_PX }}
                aria-hidden="true"
              >
                {contract.logo_url && !logoError ? (
                  <Image
                    src={contract.logo_url}
                    alt=""
                    width={LOGO_SIZE_PX}
                    height={LOGO_SIZE_PX}
                    sizes={`${LOGO_SIZE_PX}px`}
                    placeholder="blur"
                    blurDataURL={LOGO_PLACEHOLDER}
                    loading="lazy"
                    onError={() => setLogoError(true)}
                    className="h-full w-full object-cover"
                  />
                ) : (
                  <Box className="h-5 w-5 text-muted-foreground" />
                )}
              </div>

              <div className="min-w-0 flex-1">
                <div className="mb-1 flex items-center gap-2">
                  <h3 className="truncate text-lg font-semibold text-foreground transition-colors group-hover:text-primary">
                    {contract.name}
                  </h3>
                  <VerificationBadge status={contract.is_verified ? 'approved' : 'unverified'} level={contract.verification_level} />
                </div>
                <p className="font-mono text-xs text-muted-foreground">
                  {address}
                </p>
              </div>

              <span
                className={`ml-3 inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide ${networkColors[contract.network]}`}
              >
                <span className={`h-2 w-2 rounded-full ${networkDots[contract.network]}`} />
                {contract.network}
              </span>
            </div>

            <div className="mb-4 flex items-center justify-between gap-2">
              <span className="inline-flex max-w-[60%] items-center gap-1 rounded-lg bg-primary/10 px-2.5 py-1 text-xs font-medium text-primary">
                <Tag className="h-3 w-3 shrink-0" />
                <span className="truncate">{categoryLabel}</span>
              </span>
              <VerificationBadge status={contract.is_verified ? 'approved' : 'unverified'} level={contract.verification_level} />
            </div>

            {contract.description && (
              <p className="mb-4 line-clamp-2 text-sm leading-relaxed text-muted-foreground">
                {contract.description}
              </p>
            )}

            <div className="mb-4 grid grid-cols-1 gap-2 text-xs text-muted-foreground sm:grid-cols-2">
              <div className="flex items-center gap-1.5">
                <Layers3 className="h-3.5 w-3.5" />
                <span>{t('contractCard.deployments')}: {deploymentCount}</span>
              </div>
              <div className="flex min-w-0 items-center gap-1.5">
                <Clock className="h-3.5 w-3.5 shrink-0" />
                <span className="truncate">
                  {t('contractCard.updated')}: {new Date(contract.updated_at).toLocaleDateString()}
                </span>
              </div>
            </div>

            <p className="mb-4 truncate font-mono text-xs text-muted-foreground">{address}</p>

            <div className="mb-4" onClick={(event: React.MouseEvent) => event.preventDefault()}>
              <HealthWidget contract={contract} />
            </div>

            <div className="mt-auto flex flex-wrap items-center gap-2 border-t border-border pt-4">
              <button
                type="button"
                onClick={handleOpenQuickView}
                className="inline-flex items-center gap-1 rounded-md border border-border bg-card px-2.5 py-1 text-xs font-medium text-foreground transition-colors hover:bg-accent"
              >
                <Eye className="h-3.5 w-3.5" />
                <span>{t('contractCard.quickView')}</span>
              </button>
              <button
                type="button"
                onClick={handleViewDetails}
                className="inline-flex items-center gap-1 rounded-md border border-border bg-card px-2.5 py-1 text-xs font-medium text-foreground transition-colors hover:bg-accent"
              >
                <span>{t('contractCard.viewDetails')}</span>
                <ExternalLink className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                onClick={handleCopyAddress}
                onKeyDown={(event) => {
                  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'c') {
                    event.preventDefault();
                    event.stopPropagation();
                    void copyAddress();
                  }
                }}
                disabled={isCopying}
                className="inline-flex items-center gap-1 rounded-md border border-border bg-card px-2.5 py-1 text-xs font-medium text-foreground transition-colors hover:bg-accent"
              >
                {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                <span>{copied ? t('contractCard.copied') : t('contractCard.copyAddress')}</span>
              </button>
              <div className="ml-auto flex items-center gap-1">
                <FavoriteButton contractId={contract.id} size="sm" />
                {contract.favorites_count !== undefined && (
                  <span className="text-xs text-muted-foreground ml-1 font-medium bg-accent px-1.5 py-0.5 rounded-md">
                    {contract.favorites_count}
                  </span>
                )}
              </div>
            </div>
          </div>
        </div>
      </Link>

      <ContractQuickViewModal
        contract={contract}
        isOpen={quickViewOpen}
        onClose={() => setQuickViewOpen(false)}
      />
    </>
  );
}

