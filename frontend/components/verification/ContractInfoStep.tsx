'use client';

import { useFormContext } from 'react-hook-form';
import { FormInput, FormSelect } from '@/components/Form';
import type { VerificationDraft } from '@/types/verification';

const CONTRACT_ID_REGEX = /^C[A-Z2-7]{10,}$/;

export default function ContractInfoStep() {
  const {
    register,
    formState: { errors },
  } = useFormContext<VerificationDraft>();

  return (
    <div className="space-y-4">
      <FormInput
        label="Contract Name"
        placeholder="e.g. DripWave Vault"
        {...register('contractName', { required: 'Contract name is required' })}
        error={errors.contractName?.message}
      />

      <FormInput
        label="Contract Address"
        placeholder="C..."
        {...register('contractAddress', {
          required: 'Contract address is required',
          pattern: { value: CONTRACT_ID_REGEX, message: 'Enter a valid Soroban contract ID (starts with C...)' },
        })}
        error={errors.contractAddress?.message}
      />

      <FormSelect
        label="Network"
        options={[
          { value: 'mainnet', label: 'Mainnet' },
          { value: 'testnet', label: 'Testnet' },
          { value: 'futurenet', label: 'Futurenet' },
        ]}
        {...register('network', { required: 'Network is required' })}
        error={errors.network?.message}
      />
    </div>
  );
}
