import Link from 'next/link';
import { StatsResponse } from '@/types/stats';

interface TopPublishersTableProps {
  data: StatsResponse['topPublishers'];
}

const TopPublishersTable: React.FC<TopPublishersTableProps> = ({ data }) => {
  return (
    <div className="bg-card rounded-2xl border border-border p-6 h-full">
      <h3 className="text-lg font-semibold text-foreground mb-4">
        Top Publishers
      </h3>

      {!data || data.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <p className="text-sm">No publishers found yet.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-border">
            <thead>
              <tr>
                <th
                  scope="col"
                  className="px-6 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider"
                >
                  Publisher
                </th>
                <th
                  scope="col"
                  className="px-6 py-3 text-right text-xs font-medium text-muted-foreground uppercase tracking-wider"
                >
                  Contracts
                </th>
              </tr>
            </thead>
            <tbody className="bg-card divide-y divide-border">
              {data.map((publisher, index) => (
                <tr key={index} className="hover:bg-accent transition-colors">
                  <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-foreground">
                    <Link
                      href={`/publishers/${publisher.address}`}
                      className="hover:text-primary hover:underline transition-colors block"
                    >
                      {publisher.name}
                    </Link>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-right text-muted-foreground">
                    {publisher.contractsDeployed}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default TopPublishersTable;
