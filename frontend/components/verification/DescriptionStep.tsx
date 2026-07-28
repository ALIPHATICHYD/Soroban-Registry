'use client';

import { useFormContext } from 'react-hook-form';
import { FormTextarea } from '@/components/Form';
import type { VerificationDraft } from '@/types/verification';

export default function DescriptionStep() {
  const {
    register,
    formState: { errors },
  } = useFormContext<VerificationDraft>();

  return (
    <div className="space-y-4">
      <FormTextarea
        label="Purpose"
        placeholder="What does this contract do?"
        rows={4}
        {...register('purpose', {
          required: 'Purpose is required',
          minLength: { value: 20, message: 'Please provide at least 20 characters' },
        })}
        error={errors.purpose?.message}
      />

      <FormTextarea
        label="Use Case"
        placeholder="How will users interact with it?"
        rows={4}
        {...register('useCase', {
          required: 'Use case is required',
          minLength: { value: 20, message: 'Please provide at least 20 characters' },
        })}
        error={errors.useCase?.message}
      />
    </div>
  );
}

