import LoadingSkeleton from "./LoadingSkeleton";

export default function TemplateCardSkeleton() {
  return (
    <div className="relative overflow-hidden rounded-2xl border border-border bg-card p-6">
      <div className="relative">
        {/* Header */}
        <div className="flex items-start justify-between mb-3">
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <LoadingSkeleton width="50%" height="1.5rem" />
              <LoadingSkeleton width="3rem" height="1rem" />
            </div>
            <LoadingSkeleton
              width="4rem"
              height="1.5rem"
              className="rounded-full mt-2"
            />
          </div>
          <LoadingSkeleton width="4rem" height="1.25rem" />
        </div>

        {/* Description */}
        <div className="mb-4 space-y-2">
          <LoadingSkeleton width="100%" height="0.875rem" />
          <LoadingSkeleton width="80%" height="0.875rem" />
        </div>

        {/* Parameters/Tags */}
        <div className="flex flex-wrap gap-2 mb-4">
          <LoadingSkeleton
            width="4.5rem"
            height="1.75rem"
            className="rounded-md"
          />
          <LoadingSkeleton
            width="5.5rem"
            height="1.75rem"
            className="rounded-md"
          />
          <LoadingSkeleton
            width="4rem"
            height="1.75rem"
            className="rounded-md"
          />
        </div>

        {/* Command */}
        <LoadingSkeleton width="100%" height="2rem" className="rounded" />
      </div>
    </div>
  );
}
