'use client';

import { FunnelStage } from '@/types/analytics';
import { Users } from 'lucide-react';

interface EngagementFunnelProps {
  data: FunnelStage[];
}

const STAGE_COLORS = [
  { bg: '#3b82f6', light: 'rgba(59,130,246,0.12)' },
  { bg: '#8b5cf6', light: 'rgba(139,92,246,0.12)' },
  { bg: '#06b6d4', light: 'rgba(6,182,212,0.12)' },
  { bg: '#10b981', light: 'rgba(16,185,129,0.12)' },
  { bg: '#f59e0b', light: 'rgba(245,158,11,0.12)' },
];

const EngagementFunnel: React.FC<EngagementFunnelProps> = ({ data }) => {
  const maxUsers = data[0]?.users ?? 1;

  return (
    <div className="bg-card rounded-2xl border border-border p-6 flex flex-col h-full">
      <div className="mb-5">
        <h3 className="text-lg font-semibold text-foreground">User Engagement Funnel</h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          Conversion from visitors to deployed contracts
        </p>
      </div>

      <div className="flex gap-4 flex-1">
        {/* Funnel bars */}
        <div className="flex flex-col gap-2 flex-1 justify-center">
          {data.map((stage, i) => {
            const widthPct = (stage.users / maxUsers) * 100;
            const dropOff = i > 0 ? data[i - 1].users - stage.users : 0;
            const dropPct = i > 0 ? Math.round((dropOff / data[i - 1].users) * 100) : 0;
            const color = STAGE_COLORS[i % STAGE_COLORS.length];

            return (
              <div key={stage.stage} className="group relative">
                <div className="flex items-center gap-3 mb-1">
                  <span className="text-xs text-muted-foreground w-28 shrink-0">{stage.stage}</span>
                  <div className="flex-1 h-8 rounded-md overflow-hidden bg-muted/40 relative">
                    <div
                      className="h-full rounded-md flex items-center px-3 transition-all duration-500"
                      style={{
                        width: `${widthPct}%`,
                        backgroundColor: color.bg,
                        minWidth: 40,
                      }}
                    >
                      <span className="text-xs font-semibold text-white whitespace-nowrap">
                        {stage.users.toLocaleString()}
                      </span>
                    </div>
                  </div>
                  <span
                    className="text-xs font-bold w-10 text-right"
                    style={{ color: color.bg }}
                  >
                    {stage.percentage}%
                  </span>
                </div>
                {i > 0 && dropOff > 0 && (
                  <div className="flex items-center gap-1 pl-32 mb-0.5">
                    <span className="text-[10px] text-red-400">
                      -{dropOff.toLocaleString()} dropped off ({dropPct}%)
                    </span>
                  </div>
                )}
              </div>
            );
          })}
        </div>

        {/* Sidebar summary */}
        <div className="hidden sm:flex flex-col gap-3 justify-center w-28 shrink-0">
          <div className="rounded-xl border border-border p-3 text-center">
            <Users className="w-4 h-4 text-primary mx-auto mb-1" />
            <p className="text-[10px] text-muted-foreground">Conversion</p>
            <p className="text-base font-bold text-foreground">
              {data[data.length - 1]?.percentage ?? 0}%
            </p>
          </div>
          <div className="rounded-xl border border-border p-3 text-center">
            <p className="text-[10px] text-muted-foreground">Total Visitors</p>
            <p className="text-sm font-bold text-foreground">
              {(data[0]?.users ?? 0).toLocaleString()}
            </p>
          </div>
          <div className="rounded-xl border border-border p-3 text-center">
            <p className="text-[10px] text-muted-foreground">Deployed</p>
            <p className="text-sm font-bold text-foreground">
              {(data[data.length - 1]?.users ?? 0).toLocaleString()}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default EngagementFunnel;
