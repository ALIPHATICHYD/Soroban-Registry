import FormField from './FormField';
import { getFormControlId, getInputClassName, getAriaDescribedBy } from '@/lib/utils/form';

type Props = React.InputHTMLAttributes<HTMLInputElement> & {
  label?: string;
  error?: string;
  description?: string;
};

export default function FormInput({ label, error, description, className, ...rest }: Props) {
  const id = getFormControlId(rest.id, rest.name);
  const controlClassName = getInputClassName(className);
  const ariaDescribedBy = getAriaDescribedBy(id, !!error);

  return (
    <FormField label={label} id={id} error={error} description={description}>
      <input
        {...rest}
        id={id}
        className={controlClassName}
        aria-invalid={!!error}
        aria-describedby={ariaDescribedBy}
      />
    </FormField>
  );
}
