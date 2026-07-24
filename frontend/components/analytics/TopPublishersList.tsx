import { Award, Users } from 'lucide-react';

interface Publisher {
  publisher_id: string;
  name: string;
  contract_count: number;
  total_views: number;
}

export default function TopPublishersList({ data }: { data: Publisher[] }) {
  if (!data || data.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-center bg-card rounded-xl border border-border">
        <Users className="w-10 h-10 text-muted-foreground mb-3 opacity-20" />
        <p className="text-sm text-muted-foreground">No publisher data available</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {data.map((publisher, index) => (
        <div 
          key={publisher.publisher_id} 
          className="flex items-center justify-between p-3 rounded-lg hover:bg-muted/50 transition-colors border border-transparent hover:border-border group"
        >
          <div className="flex items-center gap-3">
            <div className={`
              w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold
              ${index === 0 ? 'bg-yellow-500/20 text-yellow-500' : 
                index === 1 ? 'bg-slate-400/20 text-slate-400' : 
                index === 2 ? 'bg-orange-400/20 text-orange-400' : 
                'bg-muted text-muted-foreground'}
            `}>
              {index + 1}
            </div>
            <div>
              <p className="text-sm font-medium text-foreground group-hover:text-primary transition-colors">
                {publisher.name || 'Anonymous'}
              </p>
              <div className="flex items-center gap-2 mt-0.5">
                <span className="text-[10px] text-muted-foreground flex items-center gap-1">
                  <Award className="w-3 h-3" /> {publisher.contract_count} Contracts
                </span>
              </div>
            </div>
          </div>
          <div className="text-right">
            <p className="text-xs font-semibold text-foreground">
              {publisher.total_views.toLocaleString()}
            </p>
            <p className="text-[10px] text-muted-foreground">Views</p>
          </div>
        </div>
      ))}
    </div>
  );
}
