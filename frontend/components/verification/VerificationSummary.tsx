'use client';

import type { VerificationDocument, VerificationDraft, VerificationStatus } from '@/types/verification';
import { formatBytes } from '@/utils/fileValidation';
import VerificationBadge from '@/components/verification/VerificationBadge';

export default function VerificationSummary(props: {
  draft: VerificationDraft;
  documents: VerificationDocument[];
  status?: VerificationStatus;
}) {
  const { draft, documents, status } = props;

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-border bg-card p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="text-xs text-muted-foreground uppercase tracking-wide font-semibold">Mock Contract Card</p>
            <p className="text-lg font-semibold text-foreground truncate">{draft.contractName || 'Unnamed contract'}</p>
            <p className="text-xs text-muted-foreground font-mono truncate">{draft.contractAddress || 'C…'}</p>
          </div>
          {status && status === 'approved' && <VerificationBadge status="approved" />}
        </div>
      </div>

      <div className="rounded-2xl border border-border bg-card p-4">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-sm font-semibold text-foreground">Submission Summary</h3>
          {status && <VerificationBadge status={status} />}
        </div>

        <div className="mt-4 grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground uppercase tracking-wide font-semibold">Contract</p>
            <div className="text-sm">
              <p className="text-foreground font-medium">{draft.contractName || '—'}</p>
              <p className="text-muted-foreground font-mono break-all">{draft.contractAddress || '—'}</p>
              <p className="text-muted-foreground">Network: {draft.network}</p>
            </div>
          </div>

          <div className="space-y-2">
            <p className="text-xs text-muted-foreground uppercase tracking-wide font-semibold">Security</p>
            <div className="text-sm">
              <p className="text-muted-foreground">Audit status: {draft.auditStatus.replaceAll('_', ' ')}</p>
              <p className="text-muted-foreground">Risk level: {draft.riskLevel}</p>
            </div>
          </div>

          <div className="space-y-2 sm:col-span-2">
            <p className="text-xs text-muted-foreground uppercase tracking-wide font-semibold">Description</p>
            <div className="text-sm space-y-2">
              <div>
                <p className="text-foreground font-medium">Purpose</p>
                <p className="text-muted-foreground whitespace-pre-wrap">{draft.purpose || '—'}</p>
              </div>
              <div>
                <p className="text-foreground font-medium">Use case</p>
                <p className="text-muted-foreground whitespace-pre-wrap">{draft.useCase || '—'}</p>
              </div>
            </div>
          </div>

          <div className="space-y-2 sm:col-span-2">
            <p className="text-xs text-muted-foreground uppercase tracking-wide font-semibold">Documents</p>
            {documents.length === 0 ? (
              <p className="text-sm text-muted-foreground">No documents.</p>
            ) : (
              <ul className="divide-y divide-border rounded-xl border border-border overflow-hidden">
                {documents.map((doc) => (
                  <li key={doc.id} className="bg-background px-3 py-2 flex items-center justify-between gap-3">
                    <span className="text-sm text-foreground truncate">{doc.name}</span>
                    <span className="text-xs text-muted-foreground flex-shrink-0">{formatBytes(doc.sizeBytes)}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

