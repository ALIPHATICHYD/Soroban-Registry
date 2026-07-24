import FormField from './FormField';

type Props = React.InputHTMLAttributes<HTMLInputElement> & {
  label?: string;
  error?: string;
  description?: string;
};

export default function FormCheckbox({ label, error, description, className, ...rest }: Props) {
  const id = rest.id || rest.name;
  return (
    <FormField label={label} id={id} error={error} description={description}>
      <div className="flex items-center gap-2">
        <input
          {...rest}
          id={id}
          type="checkbox"
          className={'w-4 h-4 rounded ' + (className || '')}
          aria-invalid={!!error}
        />
        {label && <label htmlFor={id} className="text-sm">{label}</label>}
      </div>
    </FormField>
  );
}
