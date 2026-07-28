import { TimePeriod } from '@/types/stats';

interface TimePeriodSelectorProps {
  selectedPeriod: TimePeriod;
  onPeriodChange: (period: TimePeriod) => void;
}

const PERIODS: { label: string; value: TimePeriod }[] = [
  { label: '7 Days', value: '7d' },
  { label: '30 Days', value: '30d' },
  { label: '90 Days', value: '90d' },
  { label: 'All Time', value: 'all-time' },
];

const TimePeriodSelector: React.FC<TimePeriodSelectorProps> = ({
  selectedPeriod,
  onPeriodChange,
}) => {
  return (
    <div className="flex space-x-2 bg-accent p-1 rounded-lg">
      {PERIODS.map((period) => (
        <button
          key={period.value}
          onClick={() => onPeriodChange(period.value)}
          className={`px-4 py-2 text-sm font-medium rounded-md transition-colors ${
            selectedPeriod === period.value
              ? 'bg-card text-primary shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          {period.label}
        </button>
      ))}
    </div>
  );
};

export default TimePeriodSelector;
