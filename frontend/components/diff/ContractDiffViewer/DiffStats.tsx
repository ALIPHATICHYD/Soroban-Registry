'use client';

import { Plus, Minus } from 'lucide-react';

interface DiffStatsProps {
    stats: {
        added: number;
        removed: number;
        changed: number;
    };
}

export function DiffStats({ stats }: DiffStatsProps) {
    return (
        <div className="flex items-center gap-4 text-xs">
            <span className="flex items-center gap-1 text-green-600 dark:text-green-400">
                <Plus size={13} />
                {stats.added} added
            </span>
            <span className="flex items-center gap-1 text-red-600 dark:text-red-400">
                <Minus size={13} />
                {stats.removed} removed
            </span>
            {stats.changed > 0 && (
                <span className="text-muted-foreground">{stats.changed} line(s) modified</span>
            )}
            {stats.added === 0 && stats.removed === 0 && (
                <span className="text-muted-foreground">No differences</span>
            )}
        </div>
    );
}
