import Link from 'next/link';
import { Clock, ExternalLink } from 'lucide-react';

export default function RecentAdditionsTimeline({ data }: { data: Record<string, unknown>[] }) {
  if (!data || data.length === 0) {
    return <div className="h-full flex items-center justify-center text-muted-foreground text-sm">No recent additions</div>;
  }

  return (
    <div className="space-y-4">
      <div className="relative border-l border-border ml-3 space-y-6 pb-2">
        {data.map((contract) => {
          const formattedDate = new Date(contract.created_at as string | number).toLocaleDateString(undefined, {
            month: 'short',
            day: 'numeric',
            year: 'numeric'
          });

          return (
            <div key={contract.id as React.Key} className="relative pl-6">
              <span className="absolute -left-1.5 top-1.5 w-3 h-3 rounded-full bg-primary ring-4 ring-background"></span>
              <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-2">
                <div>
                  <Link href={`/contracts/${contract.id as string}`} className="text-sm font-semibold text-foreground hover:text-primary transition-colors inline-flex items-center gap-1.5">
                    {(contract.name as string) || 'Unnamed Contract'}
                    <ExternalLink className="w-3 h-3 text-muted-foreground" />
                  </Link>
                  <p className="text-xs text-muted-foreground mt-1 font-mono">{(contract.contract_id as string).substring(0, 8)}...{(contract.contract_id as string).slice(-8)}</p>
                </div>
                <div className="flex items-center gap-1.5 text-xs text-muted-foreground whitespace-nowrap">
                  <Clock className="w-3.5 h-3.5" />
                  {formattedDate}
                </div>
              </div>
              <div className="mt-2 text-xs">
                <span className="px-2 py-0.5 rounded bg-secondary text-secondary-foreground font-medium">{contract.network as string}</span>
                {(contract.category as string | undefined) && (
                  <span className="ml-2 px-2 py-0.5 rounded border border-border text-muted-foreground">{contract.category as string}</span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
