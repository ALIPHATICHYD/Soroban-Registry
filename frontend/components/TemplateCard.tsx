"use client";

import Link from "next/link";
import { Download, Tag } from "lucide-react";
import { Template } from "@/lib/api";
import { useAnalytics } from "@/hooks/useAnalytics";

const CATEGORY_COLORS: Record<string, string> = {
  token: "bg-primary/10 text-primary border-primary/20",
  dex: "bg-secondary/10 text-secondary border-secondary/20",
  bridge:
    "bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-500/20",
  oracle:
    "bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20",
  lending: "bg-pink-500/10 text-pink-600 dark:text-pink-400 border-pink-500/20",
};

export default function TemplateCard({ template }: { template: Template }) {
  const colorClass =
    CATEGORY_COLORS[template.category] ??
    "bg-muted text-muted-foreground border-border";
  const { logEvent } = useAnalytics();

  return (
    <Link
      href={`/templates/${template.slug}`}
      onClick={() =>
        logEvent("template_used", {
          template_id: template.id,
          template_slug: template.slug,
          template_name: template.name,
          category: template.category,
          version: template.version,
        })
      }
    >
      <div className="group relative overflow-hidden rounded-2xl border border-border bg-card p-6 transition-all glow-border cursor-pointer">
        <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-secondary/5 opacity-0 transition-opacity group-hover:opacity-100" />

        <div className="relative">
          <div className="flex items-start justify-between mb-3">
            <div className="flex-1">
              <div className="flex items-center gap-2 mb-1">
                <h3 className="text-lg font-semibold text-foreground group-hover:text-primary transition-colors">
                  {template.name}
                </h3>
                <span className="text-xs text-muted-foreground font-mono">
                  v{template.version}
                </span>
              </div>
              <span
                className={`inline-block px-2 py-0.5 rounded-full text-xs font-medium border ${colorClass}`}
              >
                {template.category}
              </span>
            </div>
            <div className="flex items-center gap-1 text-sm text-muted-foreground ml-2">
              <Download className="w-4 h-4" />
              <span>{template.install_count.toLocaleString()}</span>
            </div>
          </div>

          {template.description && (
            <p className="text-sm text-muted-foreground mb-4 line-clamp-2">
              {template.description}
            </p>
          )}

          {template.parameters.length > 0 && (
            <div className="flex flex-wrap gap-2 mb-4">
              {template.parameters.slice(0, 3).map((p) => (
                <span
                  key={p.name}
                  className="inline-flex items-center gap-1 px-2 py-1 rounded-md bg-accent text-xs text-muted-foreground"
                >
                  <Tag className="w-3 h-3" />
                  {p.name}
                </span>
              ))}
              {template.parameters.length > 3 && (
                <span className="px-2 py-1 text-xs text-muted-foreground">
                  +{template.parameters.length - 3} more
                </span>
              )}
            </div>
          )}

          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <code className="font-mono bg-accent px-2 py-1 rounded text-xs">
              soroban-registry template clone {template.slug} my-contract
            </code>
          </div>
        </div>
      </div>
    </Link>
  );
}
