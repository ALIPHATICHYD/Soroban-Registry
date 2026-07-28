'use client';

import { ChevronDown } from 'lucide-react';
import { ContractVersion } from '@/types';

interface VersionSelectProps {
    label: string;
    versions: ContractVersion[];
    value: string;
    onChange: (v: string) => void;
    exclude?: string;
}

export function VersionSelect({ label, versions, value, onChange, exclude }: VersionSelectProps) {
    const opts = versions.filter((v) => v.version !== exclude);
    return (
        <div className="flex flex-col gap-1">
            <label className="text-xs font-semibold text-muted-foreground">{label}</label>
            <div className="relative">
                <select
                    value={value}
                    onChange={(e) => onChange(e.target.value)}
                    className="w-full appearance-none rounded-xl border border-border bg-background pl-3 pr-8 py-2 text-sm text-foreground focus:border-primary/60 focus:outline-none"
                >
                    {opts.map((v) => (
                        <option key={v.id} value={v.version}>
                            {v.version}
                            {v.commit_hash ? ` (${v.commit_hash.slice(0, 7)})` : ""}
                        </option>
                    ))}
                </select>
                <ChevronDown
                    size={14}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none"
                />
            </div>
        </div>
    );
}
