"use client";

import React, { useState } from "react";
import { Plus, Trash2, Search, Filter, Save } from "lucide-react";
import type {
  QueryNode,
  QueryCondition,
  QueryOperator,
  FieldOperator,
} from "@/types";

interface QueryBuilderProps {
  initialQuery?: QueryNode;
  onChange: (query: QueryNode) => void;
  onSearch: () => void;
  onSave?: () => void;
}

const FIELD_OPTIONS = [
  { value: "name", label: "Contract Name" },
  { value: "description", label: "Description" },
  { value: "category", label: "Category" },
  { value: "network", label: "Network" },
  { value: "verified", label: "Verification Status" },
  { value: "publisher", label: "Publisher Address" },
];

const OPERATOR_OPTIONS: Record<
  string,
  { value: FieldOperator; label: string }[]
> = {
  string: [
    { value: "eq", label: "equals" },
    { value: "ne", label: "not equals" },
    { value: "contains", label: "contains" },
    { value: "starts_with", label: "starts with" },
  ],
  boolean: [{ value: "eq", label: "is" }],
  select: [
    { value: "eq", label: "is" },
    { value: "ne", label: "is not" },
    { value: "in", label: "is one of" },
  ],
};

const NETWORK_OPTIONS = ["mainnet", "testnet", "futurenet"];
const CATEGORY_OPTIONS = [
  "DeFi",
  "NFT",
  "Governance",
  "Infrastructure",
  "Payment",
  "Identity",
  "Gaming",
  "Social",
];

export default function QueryBuilder({
  initialQuery,
  onChange,
  onSearch,
  onSave,
}: QueryBuilderProps) {
  const [query, setQuery] = useState<QueryNode>(
    initialQuery || {
      operator: "AND",
      conditions: [{ field: "name", operator: "contains", value: "" }],
    },
  );

  const updateQuery = (newQuery: QueryNode) => {
    setQuery(newQuery);
    onChange(newQuery);
  };

  const handleAddCondition = (path: number[] = []) => {
    const newQuery = { ...query };
    let target: QueryNode = newQuery;

    for (const index of path) {
      target = (target as { operator: QueryOperator; conditions: QueryNode[] })
        .conditions[index];
    }

    if ("conditions" in target) {
      target.conditions.push({
        field: "name",
        operator: "contains",
        value: "",
      });
      updateQuery(newQuery);
    }
  };

  const handleAddGroup = (path: number[] = []) => {
    const newQuery = { ...query };
    let target: QueryNode = newQuery;

    for (const index of path) {
      target = (target as { operator: QueryOperator; conditions: QueryNode[] })
        .conditions[index];
    }

    if ("conditions" in target) {
      target.conditions.push({
        operator: "OR",
        conditions: [{ field: "category", operator: "eq", value: "DeFi" }],
      });
      updateQuery(newQuery);
    }
  };

  const handleRemove = (path: number[]) => {
    if (path.length === 0) return;

    const newQuery = { ...query };
    const lastIndex = path[path.length - 1];
    const parentPath = path.slice(0, -1);

    let target: QueryNode = newQuery;
    for (const index of parentPath) {
      target = (target as { operator: QueryOperator; conditions: QueryNode[] })
        .conditions[index];
    }

    if ("conditions" in target) {
      target.conditions.splice(lastIndex, 1);
      // Don't leave empty groups
      if (target.conditions.length === 0 && parentPath.length > 0) {
        handleRemove(parentPath);
        return;
      }
      updateQuery(newQuery);
    }
  };

  const handleUpdateCondition = (
    path: number[],
    updates: Partial<QueryCondition>,
  ) => {
    const newQuery = { ...query };
    let target: QueryNode = newQuery;

    for (const index of path) {
      target = (target as { operator: QueryOperator; conditions: QueryNode[] })
        .conditions[index];
    }

    Object.assign(target, updates);
    updateQuery(newQuery);
  };

  const handleUpdateGroupOperator = (
    path: number[],
    operator: QueryOperator,
  ) => {
    const newQuery = { ...query };
    let target: QueryNode = newQuery;

    if (path.length === 0) {
      (
        target as { operator: QueryOperator; conditions: QueryNode[] }
      ).operator = operator;
    } else {
      for (const index of path) {
        target = (
          target as { operator: QueryOperator; conditions: QueryNode[] }
        ).conditions[index];
      }
      (
        target as { operator: QueryOperator; conditions: QueryNode[] }
      ).operator = operator;
    }
    updateQuery(newQuery);
  };

  const renderNode = (node: QueryNode, path: number[] = []) => {
    if ("operator" in node) {
      const group = node as {
        operator: QueryOperator;
        conditions: QueryNode[];
      };
      return (
        <div
          key={path.join("-")}
          className="border-l-2 border-primary/20 ml-2 pl-4 py-2 my-2 bg-primary/5 rounded-r-lg"
        >
          <div className="flex items-center gap-3 mb-3">
            <select
              value={group.operator}
              onChange={(e) =>
                handleUpdateGroupOperator(path, e.target.value as QueryOperator)
              }
              className="bg-primary text-primary-foreground text-xs font-bold px-2 py-1 rounded cursor-pointer hover:bg-primary/90 outline-none"
            >
              <option value="AND">AND</option>
              <option value="OR">OR</option>
            </select>
            <span className="text-xs text-muted-foreground uppercase font-medium">
              Group
            </span>

            <div className="flex-1" />

            <button
              onClick={() => handleAddCondition(path)}
              className="p-1 hover:bg-primary/10 rounded-md text-primary transition-colors"
              title="Add Condition"
            >
              <Plus className="w-4 h-4" />
            </button>
            <button
              onClick={() => handleAddGroup(path)}
              className="p-1 hover:bg-primary/10 rounded-md text-primary transition-colors"
              title="Add Nested Group"
            >
              <Filter className="w-4 h-4" />
            </button>
            {path.length > 0 && (
              <button
                onClick={() => handleRemove(path)}
                className="p-1 hover:bg-red-500/10 rounded-md text-red-500 transition-colors"
                title="Remove Group"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            )}
          </div>

          <div className="space-y-2">
            {group.conditions.map((child, i) =>
              renderNode(child, [...path, i]),
            )}
          </div>
        </div>
      );
    }

    const condition = node as QueryCondition;
    return (
      <div
        key={path.join("-")}
        className="flex flex-wrap items-center gap-2 p-2 bg-card border border-border rounded-lg group hover:border-primary/30 transition-colors shadow-sm"
      >
        <select
          value={condition.field}
          onChange={(e) =>
            handleUpdateCondition(path, { field: e.target.value, value: "" })
          }
          className="bg-background text-sm px-2 py-1 rounded border border-border outline-none focus:ring-1 focus:ring-primary min-w-[120px]"
        >
          {FIELD_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>

        <select
          value={condition.operator}
          onChange={(e) =>
            handleUpdateCondition(path, {
              operator: e.target.value as FieldOperator,
            })
          }
          className="bg-background text-sm px-2 py-1 rounded border border-border outline-none focus:ring-1 focus:ring-primary min-w-[100px]"
        >
          {getOperatorsForField(condition.field).map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>

        {renderValueInput(condition, path)}

        <button
          onClick={() => handleRemove(path)}
          className="ml-auto p-1.5 text-muted-foreground hover:text-red-500 hover:bg-red-500/10 rounded-md opacity-0 group-hover:opacity-100 transition-all"
        >
          <Trash2 className="w-4 h-4" />
        </button>
      </div>
    );
  };

  const getOperatorsForField = (field: string) => {
    switch (field) {
      case "verified":
        return OPERATOR_OPTIONS.boolean;
      case "category":
      case "network":
        return OPERATOR_OPTIONS.select;
      default:
        return OPERATOR_OPTIONS.string;
    }
  };

  const renderValueInput = (node: QueryCondition, path: number[]) => {
    if (node.field === "network") {
      return (
        <select
          value={
            typeof node.value === "string"
              ? node.value
              : String(node.value || "")
          }
          onChange={(e) =>
            handleUpdateCondition(path, { value: e.target.value })
          }
          className="bg-background text-sm px-2 py-1 rounded border border-border outline-none focus:ring-1 focus:ring-primary flex-1 min-w-[150px]"
        >
          <option value="">Select Network...</option>
          {NETWORK_OPTIONS.map((opt) => (
            <option key={opt} value={opt}>
              {opt}
            </option>
          ))}
        </select>
      );
    }

    if (node.field === "category") {
      return (
        <select
          value={
            typeof node.value === "string"
              ? node.value
              : String(node.value || "")
          }
          onChange={(e) =>
            handleUpdateCondition(path, { value: e.target.value })
          }
          className="bg-background text-sm px-2 py-1 rounded border border-border outline-none focus:ring-1 focus:ring-primary flex-1 min-w-[150px]"
        >
          <option value="">Select Category...</option>
          {CATEGORY_OPTIONS.map((opt) => (
            <option key={opt} value={opt}>
              {opt}
            </option>
          ))}
        </select>
      );
    }

    if (node.field === "verified") {
      return (
        <select
          value={String(node.value)}
          onChange={(e) =>
            handleUpdateCondition(path, { value: e.target.value === "true" })
          }
          className="bg-background text-sm px-2 py-1 rounded border border-border outline-none focus:ring-1 focus:ring-primary flex-1 min-w-[150px]"
        >
          <option value="true">Verified</option>
          <option value="false">Unverified</option>
        </select>
      );
    }

    return (
      <input
        type="text"
        value={
          typeof node.value === "string" ? node.value : String(node.value || "")
        }
        onChange={(e) => handleUpdateCondition(path, { value: e.target.value })}
        placeholder="Enter value..."
        className="bg-background text-sm px-3 py-1 rounded border border-border outline-none focus:ring-1 focus:ring-primary flex-1 min-w-[150px]"
      />
    );
  };

  return (
    <div className="bg-background rounded-xl border border-border shadow-xl overflow-hidden animate-in fade-in slide-in-from-top-4 duration-300">
      <div className="p-4 border-b border-border bg-muted/30 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="p-2 bg-primary/10 rounded-lg">
            <Filter className="w-5 h-5 text-primary" />
          </div>
          <div>
            <h3 className="font-semibold text-foreground">
              Advanced Query Builder
            </h3>
            <p className="text-xs text-muted-foreground">
              Construct complex filters with Boolean logic
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {onSave && (
            <button
              onClick={onSave}
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg border border-border hover:bg-accent text-sm font-medium transition-colors"
            >
              <Save className="w-4 h-4" />
              Save
            </button>
          )}
        </div>
      </div>

      <div className="p-6 max-h-[60vh] overflow-y-auto">
        {renderNode(query)}
      </div>

      <div className="p-4 bg-muted/30 border-t border-border flex items-center justify-between">
        <button
          onClick={() =>
            updateQuery({
              operator: "AND",
              conditions: [{ field: "name", operator: "contains", value: "" }],
            })
          }
          className="text-sm text-red-500 hover:text-red-600 font-medium px-3 py-2 rounded-lg hover:bg-red-500/5 transition-colors"
        >
          Reset Builder
        </button>

        <button
          onClick={onSearch}
          className="flex items-center gap-2 px-6 py-2 rounded-lg bg-primary text-primary-foreground font-semibold hover:opacity-90 transition-opacity btn-glow"
        >
          <Search className="w-4 h-4" />
          Run Advanced Search
        </button>
      </div>
    </div>
  );
}
