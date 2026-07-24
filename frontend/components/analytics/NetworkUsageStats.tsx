import { Activity, Globe } from 'lucide-react';

interface NetworkCount {
  network: string;
  count: number;
}

export default function NetworkUsageStats({ data }: { data: NetworkCount[] }) {
  const mainnet = data.find(d => d.network.toLowerCase() === 'mainnet')?.count || 0;
  const testnet = data.find(d => d.network.toLowerCase() === 'testnet')?.count || 0;
  const total = mainnet + testnet;

  return (
    <div className="grid grid-cols-1 sm:grid-cols-3 gap-6 h-full">
      <div className="bg-card border border-border rounded-xl shadow-sm p-6 flex flex-col justify-center">
        <div className="flex items-center gap-3 mb-2">
            <div className="p-2 rounded-lg bg-primary/10">
                <Globe className="w-5 h-5 text-primary" />
            </div>
            <h3 className="text-sm font-semibold text-muted-foreground">Mainnet</h3>
        </div>
        <p className="text-3xl font-bold text-foreground">{mainnet.toLocaleString()}</p>
      </div>

      <div className="bg-card border border-border rounded-xl shadow-sm p-6 flex flex-col justify-center">
        <div className="flex items-center gap-3 mb-2">
            <div className="p-2 rounded-lg bg-blue-500/10">
                <Activity className="w-5 h-5 text-blue-500" />
            </div>
            <h3 className="text-sm font-semibold text-muted-foreground">Testnet</h3>
        </div>
        <p className="text-3xl font-bold text-foreground">{testnet.toLocaleString()}</p>
      </div>

      <div className="bg-card border border-border rounded-xl shadow-sm p-6 flex flex-col justify-center bg-gradient-to-br from-card to-primary/5">
        <div className="flex items-center gap-3 mb-2">
            <h3 className="text-sm font-semibold text-muted-foreground">Total Deployed</h3>
        </div>
        <p className="text-4xl font-black text-foreground">{total.toLocaleString()}</p>
      </div>
    </div>
  );
}
