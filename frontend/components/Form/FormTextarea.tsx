import FormField from './FormField';
import { getFormControlId, getTextareaClassName, getAriaDescribedBy } from '@/lib/utils/form';

type Props = React.TextareaHTMLAttributes<HTMLTextAreaElement> & {
  label?: string;
  error?: string;
  description?: string;
};

export default function FormTextarea({ label, error, description, className, ...rest }: Props) {
  const id = getFormControlId(rest.id, rest.name);
  const controlClassName = getTextareaClassName(className);
  const ariaDescribedBy = getAriaDescribedBy(id, !!error);

  return (
    <FormField label={label} id={id} error={error} description={description}>
      <textarea
        {...rest}
        id={id}
        className={controlClassName}
        aria-invalid={!!error}
        aria-describedby={ariaDescribedBy}
      />
    </FormField>
  );
}
