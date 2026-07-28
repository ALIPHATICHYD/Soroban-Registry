
const AnalyticsSkeleton: React.FC = () => (
  <div className="animate-pulse space-y-6">
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <div className="h-80 bg-muted rounded-2xl" />
      <div className="h-80 bg-muted rounded-2xl" />
    </div>
    <div className="h-72 bg-muted rounded-2xl" />
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <div className="h-80 bg-muted rounded-2xl" />
      <div className="h-80 bg-muted rounded-2xl" />
    </div>
    <div className="h-96 bg-muted rounded-2xl" />
  </div>
);

export default AnalyticsSkeleton;
