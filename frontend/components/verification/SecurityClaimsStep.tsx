'use client';

import { useFormContext } from 'react-hook-form';
import { FormSelect, FormTextarea } from '@/components/Form';
import type { VerificationDraft } from '@/types/verification';

export default function SecurityClaimsStep() {
  const {
    register,
    formState: { errors },
  } = useFormContext<VerificationDraft>();

  return (
    <div className="space-y-4">
      <FormSelect
        label="Audit Status"
        options={[
          { value: 'not_audited', label: 'Not audited' },
          { value: 'in_progress', label: 'Audit in progress' },
          { value: 'audited', label: 'Audited' },
        ]}
        {...register('auditStatus', { required: 'Audit status is required' })}
        error={errors.auditStatus?.message}
      />

      <FormSelect
        label="Risk Level"
        options={[
          { value: 'low', label: 'Low' },
          { value: 'medium', label: 'Medium' },
          { value: 'high', label: 'High' },
          { value: 'critical', label: 'Critical' },
        ]}
        {...register('riskLevel', { required: 'Risk level is required' })}
        error={errors.riskLevel?.message}
      />

      <FormTextarea
        label="Known Vulnerabilities"
        placeholder="List any known issues or mitigations. If none, write “None”."
        rows={5}
        {...register('knownVulnerabilities', {
          required: 'Please provide a statement about vulnerabilities',
          minLength: { value: 3, message: 'Please provide a short statement' },
        })}
        error={errors.knownVulnerabilities?.message}
      />
    </div>
  );
}

