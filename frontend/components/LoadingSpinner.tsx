
export default function LoadingSpinner({
  size = 'md',
  className = '',
}: {
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}) {
  const sizeClasses = {
    sm: 'w-4 h-4 border-2',
    md: 'w-8 h-8 border-3',
    lg: 'w-12 h-12 border-4',
  };

  return (
    <div
      role="status"
      className={`inline-block animate-spin rounded-full border-current border-t-transparent text-primary ${sizeClasses[size]} ${className}`}
      aria-label="Loading"
    >
      <span className="sr-only">Loading...</span>
    </div>
  );
}
