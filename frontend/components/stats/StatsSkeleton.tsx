
const StatsSkeleton: React.FC = () => {
  return (
    <div className="animate-pulse space-y-6">
      {/* Summary Cards Skeleton */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {[1, 2, 3].map((i) => (
          <div key={i} className="h-32 bg-muted rounded-2xl"></div>
        ))}
      </div>

      {/* Charts Row Skeleton */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="h-80 bg-muted rounded-2xl"></div>
        <div className="h-80 bg-muted rounded-2xl"></div>
      </div>

      {/* Bottom Row Skeleton */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="h-64 bg-muted rounded-2xl"></div>
        <div className="h-64 bg-muted rounded-2xl"></div>
      </div>
    </div>
  );
};

export default StatsSkeleton;
