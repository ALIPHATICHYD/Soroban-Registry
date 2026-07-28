import FormField from './FormField';

type Option = { value: string | number; label: string };

type Props = {
  name: string;
  label?: string;
  options: Option[];
  value?: string | number;
  onChange?: (value: string | number) => void;
  error?: string;
  description?: string;
};

export default function FormRadioGroup({ name, label, options, value, onChange, error, description }: Props) {
  return (
    <FormField label={label} id={name} error={error} description={description}>
      <div role="radiogroup" aria-labelledby={name} className="flex flex-col gap-2">
        {options.map((opt) => (
          <label key={String(opt.value)} className="inline-flex items-center gap-2">
            <input
              type="radio"
              name={name}
              value={opt.value}
              checked={String(value) === String(opt.value)}
              onChange={() => onChange && onChange(opt.value)}
              className="w-4 h-4"
            />
            <span className="text-sm">{opt.label}</span>
          </label>
        ))}
      </div>
    </FormField>
  );
}
