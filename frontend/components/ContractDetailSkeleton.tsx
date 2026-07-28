import LoadingSkeleton from './LoadingSkeleton';

export default function ContractDetailSkeleton() {
  return (
    <div className="space-y-6 animate-in fade-in duration-500">
      {/* Header Skeleton */}
      <div className="flex flex-col md:flex-row md:items-start justify-between gap-4">
        <div className="space-y-3 flex-1">
          <LoadingSkeleton width="60%" height="2.5rem" />
          <LoadingSkeleton width="40%" height="1.25rem" />
          <div className="flex gap-2 mt-4">
            <LoadingSkeleton width="4rem" height="1.5rem" variant="rectangular" />
            <LoadingSkeleton width="5rem" height="1.5rem" variant="rectangular" />
          </div>
        </div>
        <div className="flex gap-2">
          <LoadingSkeleton width="6rem" height="2.5rem" variant="rectangular" />
          <LoadingSkeleton width="6rem" height="2.5rem" variant="rectangular" />
        </div>
      </div>

      {/* Stats row */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 py-6 border-y border-border">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="space-y-2">
            <LoadingSkeleton width="50%" height="1rem" />
            <LoadingSkeleton width="70%" height="1.5rem" />
          </div>
        ))}
      </div>

      {/* Main content grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        <div className="lg:col-span-2 space-y-6">
          <div className="space-y-4">
            <LoadingSkeleton width="30%" height="1.5rem" />
            <div className="space-y-2">
              <LoadingSkeleton width="100%" height="1rem" />
              <LoadingSkeleton width="100%" height="1rem" />
              <LoadingSkeleton width="90%" height="1rem" />
              <LoadingSkeleton width="95%" height="1rem" />
            </div>
          </div>
          
          <div className="space-y-4 mt-8">
            <LoadingSkeleton width="40%" height="1.5rem" />
            <LoadingSkeleton width="100%" height="15rem" variant="rectangular" />
          </div>
        </div>
        
        <div className="space-y-6">
          <div className="border border-border rounded-lg p-4 space-y-4">
            <LoadingSkeleton width="50%" height="1.25rem" />
            <LoadingSkeleton width="100%" height="3rem" />
            <LoadingSkeleton width="100%" height="3rem" />
          </div>
          <div className="border border-border rounded-lg p-4 space-y-4">
            <LoadingSkeleton width="40%" height="1.25rem" />
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <div key={i} className="flex items-center gap-3">
                  <LoadingSkeleton width="2rem" height="2rem" variant="circular" />
                  <LoadingSkeleton width="60%" height="1rem" />
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
