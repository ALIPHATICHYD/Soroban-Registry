'use client';

import type { QueryNode, FieldOperator } from '@/types';
import { Info } from 'lucide-react';

interface QuerySummaryProps {
  query: QueryNode;
  className?: string;
}

const FIELD_LABELS: Record<string, string> = {
  name: 'Name',
  description: 'Description',
  category: 'Category',
  network: 'Network',
  verified: 'Verification',
  publisher: 'Publisher',
};

const OP_LABELS: Record<FieldOperator, string> = {
  eq: 'is',
  ne: 'is not',
  gt: 'is greater than',
  lt: 'is less than',
  in: 'is one of',
  contains: 'contains',
  starts_with: 'starts with',
};

export default function QuerySummary({ query, className = '' }: QuerySummaryProps) {
  const renderNode = (node: QueryNode): string => {
    if ('conditions' in node) {
      const { conditions, operator } = node;
      if (conditions.length === 0) return '';

      const parts = conditions.map((c) => renderNode(c)).filter(Boolean);

      if (parts.length === 1) return parts[0];

      const joined = parts.join(` ${operator} `);
      return `(${joined})`;
    }

    const field = FIELD_LABELS[node.field] || node.field;
    const op = OP_LABELS[node.operator] || node.operator;
    let val: string;

    if (node.field === 'verified') {
      val = node.value ? 'Verified' : 'Unverified';
    } else if (Array.isArray(node.value)) {
      val = `[${node.value.join(', ')}]`;
    } else if (typeof node.value === 'string') {
      val = `'${node.value}'`;
    } else {
      val = String(node.value);
    }

    return `${field} ${op} ${val}`;
  };

  const summary = renderNode(query);

  if (!summary) return null;

  return (
    <div className={`flex items-start gap-3 p-4 bg-primary/5 border border-primary/20 rounded-xl ${className}`}>
      <div className="p-2 bg-primary/10 rounded-lg shrink-0">
        <Info className="w-4 h-4 text-primary" />
      </div>
      <div>
        <h4 className="text-xs font-bold text-primary uppercase tracking-wider mb-1">Query Summary</h4>
        <p className="text-sm text-foreground leading-relaxed font-medium">
          Showing contracts where <span className="text-primary">{summary}</span>
        </p>
      </div>
    </div>
  );
}
