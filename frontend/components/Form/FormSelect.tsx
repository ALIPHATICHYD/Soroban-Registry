import FormField from './FormField';
import { getFormControlId, getSelectClassName, getAriaDescribedBy } from '@/lib/utils/form';

type Option = { value: string | number; label: string };

type Props = React.SelectHTMLAttributes<HTMLSelectElement> & {
  label?: string;
  options: Option[];
  error?: string;
  description?: string;
};

export default function FormSelect({ label, options, error, description, className, ...rest }: Props) {
  const id = getFormControlId(rest.id, rest.name);
  const controlClassName = getSelectClassName(className);
  const ariaDescribedBy = getAriaDescribedBy(id, !!error);

  return (
    <FormField label={label} id={id} error={error} description={description}>
      <select
        {...rest}
        id={id}
        className={controlClassName}
        aria-invalid={!!error}
        aria-describedby={ariaDescribedBy}
      >
        {options.map((opt) => (
          <option key={String(opt.value)} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </FormField>
  );
}
