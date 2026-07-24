import LoadingSkeleton from "./LoadingSkeleton";

export default function ExampleCardSkeleton() {
  return (
    <div className="bg-card rounded-2xl border border-border overflow-hidden">
      {/* Header */}
      <div className="p-6 border-b border-border">
        <div className="flex items-start justify-between mb-4">
          <div className="flex-1">
            <LoadingSkeleton width="60%" height="1.5rem" className="mb-2" />
            <LoadingSkeleton width="5rem" height="1.5rem" className="rounded" />
          </div>

          <div className="flex items-center gap-2">
            <LoadingSkeleton
              width="4rem"
              height="2.5rem"
              className="rounded-lg"
            />
            <LoadingSkeleton
              width="4rem"
              height="2.5rem"
              className="rounded-lg"
            />
          </div>
        </div>

        <div className="space-y-2">
          <LoadingSkeleton width="100%" height="0.875rem" />
          <LoadingSkeleton width="90%" height="0.875rem" />
        </div>
      </div>

      {/* Code Section */}
      <div className="p-6">
        {/* Tabs */}
        <div className="flex items-center gap-4 mb-4 border-b border-border pb-2">
          <LoadingSkeleton width="10rem" height="1.25rem" />
          <LoadingSkeleton width="6rem" height="1.25rem" />
        </div>

        {/* Code Block */}
        <div className="space-y-3">
          <LoadingSkeleton width="100%" height="1rem" />
          <LoadingSkeleton width="95%" height="1rem" />
          <LoadingSkeleton width="85%" height="1rem" />
          <LoadingSkeleton width="90%" height="1rem" />
          <LoadingSkeleton width="80%" height="1rem" />
          <LoadingSkeleton width="100%" height="1rem" />
          <LoadingSkeleton width="75%" height="1rem" />
        </div>
      </div>
    </div>
  );
}
