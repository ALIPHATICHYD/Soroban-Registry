
type Props = React.PropsWithChildren<{
  label?: string;
  name?: string;
  id?: string;
  required?: boolean;
  error?: string | undefined;
  description?: string;
  className?: string;
}>;

export default function FormField({ label, id, required, error, description, children, className }: Props) {
  return (
    <div className={className}>
      {label && (
        <label htmlFor={id} className="block text-sm font-medium mb-1">
          {label}{required ? ' *' : ''}
        </label>
      )}

      {children}

      {description && <p className="text-xs text-muted-foreground mt-1">{description}</p>}

      {error && <p role="alert" className="text-xs text-red-600 mt-1">{error}</p>}
    </div>
  );
}
