import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from 'recharts';

interface CategoryCount {
  category: string;
  count: number;
}

const COLORS = ['#6366f1', '#ec4899', '#8b5cf6', '#14b8a6', '#f59e0b', '#3b82f6', '#10b981'];

export default function CategoryDistributionPie({ data }: { data: CategoryCount[] }) {
  if (!data || data.length === 0) {
    return <div className="h-full flex items-center justify-center text-muted-foreground text-sm">No category data available</div>;
  }

  return (
    <div className="h-[250px] w-full">
      <ResponsiveContainer width="100%" height="100%">
        <PieChart>
          <Pie
            data={data}
            cx="50%"
            cy="50%"
            innerRadius={60}
            outerRadius={80}
            paddingAngle={5}
            dataKey="count"
            nameKey="category"
          >
            {data.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip 
            contentStyle={{ backgroundColor: 'hsl(var(--card))', borderColor: 'hsl(var(--border))', borderRadius: '0.5rem' }}
            itemStyle={{ color: 'hsl(var(--foreground))' }}
            formatter={(value) => [typeof value === 'number' ? value : (value ?? 0), 'Contracts']}
          />
        </PieChart>
      </ResponsiveContainer>
      
      {/* Custom Legend */}
      <div className="flex flex-wrap items-center justify-center gap-3 mt-4">
        {data.map((entry, index) => (
          <div key={entry.category} className="flex items-center gap-1.5 text-xs">
            <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: COLORS[index % COLORS.length] }}></span>
            <span className="text-muted-foreground">{entry.category}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
