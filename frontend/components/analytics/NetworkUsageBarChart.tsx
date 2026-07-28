import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from 'recharts';

interface NetworkCount {
  network: string;
  contract_count: number;
}

const COLORS = ['#6366f1', '#3b82f6', '#10b981', '#f59e0b', '#8b5cf6'];

export default function NetworkUsageBarChart({ data }: { data: NetworkCount[] }) {
  if (!data || data.length === 0) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
        No network data available
      </div>
    );
  }

  // Sort by count descending
  const sortedData = [...data].sort((a, b) => b.contract_count - a.contract_count);

  return (
    <div className="h-[250px] w-full">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={sortedData} layout="vertical" margin={{ top: 5, right: 30, left: 40, bottom: 5 }}>
          <CartesianGrid strokeDasharray="3 3" horizontal={false} stroke="hsl(var(--border))" opacity={0.5} />
          <XAxis type="number" hide />
          <YAxis 
            dataKey="network" 
            type="category" 
            axisLine={false}
            tickLine={false}
            fontSize={12}
            width={80}
            tick={{ fill: 'hsl(var(--muted-foreground))' }}
          />
          <Tooltip 
            cursor={{ fill: 'hsl(var(--muted))', opacity: 0.4 }}
            contentStyle={{ backgroundColor: 'hsl(var(--card))', borderColor: 'hsl(var(--border))', borderRadius: '0.5rem' }}
            itemStyle={{ color: 'hsl(var(--foreground))' }}
          />
          <Bar dataKey="contract_count" radius={[0, 4, 4, 0]} barSize={20}>
            {sortedData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
