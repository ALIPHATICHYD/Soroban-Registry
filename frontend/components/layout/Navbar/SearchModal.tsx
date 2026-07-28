"use client";

import React, { useState, useRef, useEffect } from "react";
import Link from "next/link";
import { Search, Zap, Layers, TrendingUp, Users, Code2 } from "lucide-react";
import styles from "./Navbar.module.css";

interface SearchModalProps {
  isOpen: boolean;
  onClose: () => void;
}

const SUGGESTIONS = [
  { label: "Token Contract", href: "/contracts?q=token", icon: Zap },
  { label: "NFT Marketplace", href: "/contracts?q=nft", icon: Layers },
  { label: "DeFi Protocols", href: "/contracts?q=defi", icon: TrendingUp },
  { label: "DAO Governance", href: "/contracts?q=dao", icon: Users },
  { label: "Smart Contract SDK", href: "/contracts?q=sdk", icon: Code2 },
];

export function SearchModal({ isOpen, onClose }: SearchModalProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => {
        inputRef.current?.focus();
        setQuery("");
      }, 0);
    }
  }, [isOpen]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  if (!isOpen) return null;

  return (
    <div
      className={styles.modalOverlay}
      role="dialog"
      aria-modal="true"
      aria-label="Search"
    >
      <div className={styles.modalBackdrop} onClick={onClose} />

      <div className={styles.modalPanel}>
        <div className="flex items-center gap-3 px-4 py-3 border-b border-border">
          <Search className="w-5 h-5 text-muted-foreground flex-shrink-0" />
          <input
            ref={inputRef}
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search contracts, publishers, templates…"
            className="flex-1 bg-transparent text-foreground text-sm focus:outline-none placeholder:text-muted-foreground"
          />
          <button
            onClick={onClose}
            className="flex items-center gap-1 px-2 py-1 rounded-md border border-border text-xs text-muted-foreground hover:border-primary/50 transition-colors"
            aria-label="Close search"
          >
            <span>ESC</span>
          </button>
        </div>

        <div className="p-2">
          <p className="px-3 py-1.5 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
            {query ? "Results" : "Popular searches"}
          </p>
          <ul>
            {SUGGESTIONS.map(({ label, href, icon: Icon }) => (
              <li key={href}>
                <Link
                  href={`${href}${query ? `&q=${encodeURIComponent(query)}` : ""}`}
                  onClick={onClose}
                  className="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-muted-foreground hover:text-foreground hover:bg-accent transition-colors group"
                >
                  <span className="w-7 h-7 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0 group-hover:bg-primary/20 transition-colors">
                    <Icon className="w-3.5 h-3.5 text-primary" />
                  </span>
                  {label}
                </Link>
              </li>
            ))}
          </ul>
        </div>

        <div className="px-4 py-2.5 border-t border-border bg-accent/30 flex items-center justify-between">
          <span className="text-xs text-muted-foreground">
            Press{" "}
            <kbd className="px-1.5 py-0.5 rounded border border-border bg-card text-foreground font-mono text-[10px]">
              ↑↓
            </kbd>{" "}
            to navigate
          </span>
          <Link
            href={`/contracts${query ? `?q=${encodeURIComponent(query)}` : ""}`}
            onClick={onClose}
            className="text-xs text-primary hover:underline"
          >
            Browse all →
          </Link>
        </div>
      </div>
    </div>
  );
}
