import { StatsResponse } from '@/types/stats';

interface NetworkDistributionProps {
  data: StatsResponse['networkBreakdown'];
}

const NetworkDistribution: React.FC<NetworkDistributionProps> = ({ data }) => {
  const total = data.reduce((acc, curr) => acc + curr.contracts, 0);

  return (
    <div className="bg-card rounded-2xl border border-border p-6 h-full">
      <h3 className="text-lg font-semibold text-foreground mb-4">
        Network Distribution
      </h3>
      <div className="space-y-4">
        {data.map((item) => {
          const percentage = total > 0 ? (item.contracts / total) * 100 : 0;
          return (
            <div key={item.network}>
              <div className="flex justify-between items-center mb-1">
                <span className="text-sm font-medium text-foreground">
                  {item.network}
                </span>
                <span className="text-sm text-muted-foreground">
                  {item.contracts} ({percentage.toFixed(1)}%)
                </span>
              </div>
              <div className="w-full bg-muted rounded-full h-2.5">
                <div
                  className="bg-primary h-2.5 rounded-full"
                  style={{ width: `${percentage}%` }}
                ></div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default NetworkDistribution;
