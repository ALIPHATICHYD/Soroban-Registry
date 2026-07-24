import { StatsResponse } from '@/types/stats';
import { FileText, CheckCircle2, Users } from 'lucide-react';

interface StatsSummaryCardsProps {
  data: StatsResponse;
}

const StatsSummaryCards: React.FC<StatsSummaryCardsProps> = ({ data }) => {
  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
      <div className="rounded-2xl border border-border p-6 bg-card transition-all hover:shadow-lg hover:border-primary/50">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-muted-foreground">Total Contracts</p>
            <h3 className="text-2xl font-bold text-foreground mt-1">
              {data.totalContracts.toLocaleString()}
            </h3>
          </div>
          <div className="p-3 bg-primary/10 rounded-lg">
            <FileText className="w-6 h-6 text-primary" />
          </div>
        </div>
      </div>

      <div className="rounded-2xl border border-border p-6 bg-card transition-all hover:shadow-lg hover:border-green-500/50">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-muted-foreground">Verified Contracts</p>
            <h3 className="text-2xl font-bold text-foreground mt-1">
              {data.verifiedPercentage}%
            </h3>
          </div>
          <div className="p-3 bg-green-500/10 rounded-lg">
            <CheckCircle2 className="w-6 h-6 text-green-600" />
          </div>
        </div>
      </div>

      <div className="rounded-2xl border border-border p-6 bg-card transition-all hover:shadow-lg hover:border-secondary/50">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-muted-foreground">Total Publishers</p>
            <h3 className="text-2xl font-bold text-foreground mt-1">
              {data.totalPublishers.toLocaleString()}
            </h3>
          </div>
          <div className="p-3 bg-secondary/10 rounded-lg">
            <Users className="w-6 h-6 text-secondary" />
          </div>
        </div>
      </div>
    </div>
  );
};

export default StatsSummaryCards;
