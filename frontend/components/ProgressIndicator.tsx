
export default function ProgressIndicator({
  progress,
  label = 'Loading...',
  className = '',
}: {
  progress: number;
  label?: string;
  className?: string;
}) {
  return (
    <div className={`w-full ${className}`} role="progressbar" aria-valuenow={progress} aria-valuemin={0} aria-valuemax={100}>
      <div className="flex justify-between mb-1">
        <span className="text-sm font-medium text-primary">{label}</span>
        <span className="text-sm font-medium text-primary">{Math.round(progress)}%</span>
      </div>
      <div className="w-full bg-muted rounded-full h-2.5">
        <div
          className="bg-primary h-2.5 rounded-full transition-all duration-300 ease-in-out"
          style={{ width: `${Math.max(0, Math.min(100, progress))}%` }}
        ></div>
      </div>
    </div>
  );
}
