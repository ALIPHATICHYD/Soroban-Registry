'use client';

import { Check } from 'lucide-react';
import type { VerificationStepKey } from '@/types/verification';

const LABELS: Record<VerificationStepKey, string> = {
  contractInfo: 'Contract Info',
  description: 'Description',
  securityClaims: 'Security Claims',
  documents: 'Documents',
  review: 'Review',
};

export default function Stepper(props: {
  steps: VerificationStepKey[];
  activeIndex: number;
  onStepClick?: (index: number) => void;
}) {
  const { steps, activeIndex, onStepClick } = props;

  return (
    <ol className="flex flex-wrap items-center gap-3 sm:gap-4">
      {steps.map((step, idx) => {
        const isActive = idx === activeIndex;
        const isComplete = idx < activeIndex;
        const isClickable = !!onStepClick && idx <= activeIndex;

        return (
          <li key={step} className="flex items-center gap-2">
            <button
              type="button"
              onClick={isClickable ? () => onStepClick(idx) : undefined}
              className={
                'flex items-center gap-2 rounded-full pr-3 transition-colors ' +
                (isClickable ? 'hover:bg-accent cursor-pointer' : 'cursor-default') +
                (isActive ? ' bg-primary/10' : '')
              }
              aria-current={isActive ? 'step' : undefined}
              aria-disabled={!isClickable}
            >
              <span
                className={
                  'w-7 h-7 rounded-full border flex items-center justify-center text-xs font-semibold ' +
                  (isComplete ? 'bg-primary text-primary-foreground border-primary' : '') +
                  (isActive ? 'border-primary text-primary' : '') +
                  (!isActive && !isComplete ? 'border-border text-muted-foreground bg-card' : '')
                }
              >
                {isComplete ? <Check className="w-4 h-4" /> : idx + 1}
              </span>
              <span className={'text-xs sm:text-sm font-medium ' + (isActive ? 'text-foreground' : 'text-muted-foreground')}>
                {LABELS[step]}
              </span>
            </button>
            {idx < steps.length - 1 && <span className="hidden sm:block w-8 h-px bg-border" />}
          </li>
        );
      })}
    </ol>
  );
}

