import Link from 'next/link';
import { ArrowUpRight, Minus, ArrowDownRight } from 'lucide-react';

export default function TrendingContractsTable({ data }: { data: Record<string, unknown>[] }) {
  if (!data || data.length === 0) {
    return <div className="h-32 flex items-center justify-center text-muted-foreground text-sm border-t border-border">No trending contracts available</div>;
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-left text-sm whitespace-nowrap">
        <thead className="bg-muted/50 text-muted-foreground text-xs uppercase tracking-wider">
          <tr>
            <th className="px-5 py-3 font-medium">Rank</th>
            <th className="px-5 py-3 font-medium">Contract Name</th>
            <th className="px-5 py-3 font-medium">Network</th>
            <th className="px-5 py-3 font-medium">Weekly Interactions</th>
            <th className="px-5 py-3 font-medium">Trend</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-border">
          {data.map((contract, idx) => {
            const current = (contract.interactions_this_week as number) || 0;
            const previous = (contract.interactions_last_week as number) || 0;
            let trendIcon = <Minus className="w-4 h-4 text-muted-foreground" />;
            let trendClass = "text-muted-foreground bg-accent";
            let percentChange = 0;

            if (previous > 0) {
              percentChange = Math.round(((current - previous) / previous) * 100);
            } else if (current > 0) {
               percentChange = 100;
            }

            if (current > previous) {
              trendIcon = <ArrowUpRight className="w-4 h-4" />;
              trendClass = "text-emerald-500 bg-emerald-500/10";
            } else if (current < previous) {
              trendIcon = <ArrowDownRight className="w-4 h-4" />;
              trendClass = "text-red-500 bg-red-500/10";
            }

            return (
              <tr key={contract.id as React.Key} className="hover:bg-muted/30 transition-colors">
                <td className="px-5 py-4 font-semibold text-foreground">#{idx + 1}</td>
                <td className="px-5 py-4">
                  <Link href={`/contracts/${contract.id as string}`} className="font-semibold text-foreground hover:text-primary transition-colors pr-8">
                    {(contract.name as string) || 'Unnamed'}
                  </Link>
                  <p className="text-xs text-muted-foreground font-mono mt-0.5">{(contract.contract_id as string).substring(0, 8)}...{(contract.contract_id as string).slice(-8)}</p>
                </td>
                <td className="px-5 py-4">
                  <span className="px-2.5 py-1 rounded-full text-xs font-medium bg-secondary text-secondary-foreground">
                    {contract.network as string}
                  </span>
                </td>
                <td className="px-5 py-4 text-foreground font-semibold">
                  {current.toLocaleString()}
                </td>
                <td className="px-5 py-4">
                  <div className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${trendClass}`}>
                    {trendIcon}
                    {percentChange > 0 ? '+' : ''}{percentChange}%
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
