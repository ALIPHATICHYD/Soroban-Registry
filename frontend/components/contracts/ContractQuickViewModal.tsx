"use client";

import { useEffect, useId, useMemo, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { ExternalLink, Globe, Layers3, Loader2, Tag, X } from "lucide-react";
import { api } from "@/lib/api";
import type { Contract } from "@/types";
import {
  extractAbiMethodNames,
  getQuickViewVerificationStatus,
} from "@/lib/contractQuickView";
import VerificationBadge from "@/components/verification/VerificationBadge";

interface ContractQuickViewModalProps {
  contract: Contract;
  isOpen: boolean;
  onClose: () => void;
}

export default function ContractQuickViewModal(
  props: ContractQuickViewModalProps,
) {
  const { contract, isOpen, onClose } = props;
  const titleId = useId();
  const closeButtonRef = useRef<HTMLButtonElement | null>(null);

  const { data: contractDetails, isFetching: isContractRefreshing } = useQuery({
    queryKey: ["contract-quick-view", contract.id],
    queryFn: () => api.getContract(contract.id),
    enabled: isOpen,
  });

  const {
    data: abiResponse,
    isLoading: isAbiLoading,
    error: abiError,
  } = useQuery({
    queryKey: ["contract-quick-view-abi", contract.id],
    queryFn: () => api.getContractAbi(contract.id),
    enabled: isOpen,
  });

  const displayContract = contractDetails ?? contract;
  const methodNames = useMemo(
    () => extractAbiMethodNames(abiResponse?.abi),
    [abiResponse],
  );
  const verificationStatus = getQuickViewVerificationStatus(displayContract);

  useEffect(() => {
    if (!isOpen) {
      return undefined;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    const focusTimer = window.setTimeout(
      () => closeButtonRef.current?.focus(),
      0,
    );

    return () => {
      window.clearTimeout(focusTimer);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [isOpen, onClose]);

  useEffect(() => {
    if (!isOpen) {
      return undefined;
    }

    const { body, documentElement } = document;
    const previousOverflow = body.style.overflow;
    const previousPaddingRight = body.style.paddingRight;
    const scrollbarWidth = window.innerWidth - documentElement.clientWidth;

    body.style.overflow = "hidden";
    if (scrollbarWidth > 0) {
      body.style.paddingRight = `${scrollbarWidth}px`;
    }

    return () => {
      body.style.overflow = previousOverflow;
      body.style.paddingRight = previousPaddingRight;
    };
  }, [isOpen]);

  if (!isOpen) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-[220] flex items-end justify-center px-4 py-4 sm:items-center sm:py-8">
      <div
        className="absolute inset-0 bg-background/70 backdrop-blur-sm"
        onClick={onClose}
        aria-hidden="true"
      />

      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="relative w-full max-w-2xl overflow-hidden rounded-3xl border border-border bg-card shadow-2xl animate-modal-in"
      >
        <div className="max-h-[min(88vh,720px)] overflow-y-auto">
          <div className="flex items-start justify-between gap-4 border-b border-border px-5 py-4 sm:px-6">
            <div className="min-w-0">
              <div className="mb-2 flex flex-wrap items-center gap-2">
                <span className="inline-flex items-center gap-1 rounded-full border border-primary/20 bg-primary/10 px-2.5 py-1 text-[11px] font-semibold uppercase tracking-wide text-primary">
                  Quick View
                </span>
                <VerificationBadge status={verificationStatus} />
                {isContractRefreshing && (
                  <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    Refreshing
                  </span>
                )}
              </div>
              <h2
                id={titleId}
                className="truncate text-2xl font-bold text-foreground"
              >
                {displayContract.name}
              </h2>
              <p className="mt-1 font-mono text-xs text-muted-foreground">
                {displayContract.contract_id}
              </p>
            </div>

            <button
              ref={closeButtonRef}
              type="button"
              onClick={onClose}
              className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-border bg-background text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
              aria-label="Close quick view"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          <div className="space-y-6 px-5 py-5 sm:px-6 sm:py-6">
            {displayContract.description && (
              <p className="text-sm leading-6 text-muted-foreground">
                {displayContract.description}
              </p>
            )}

            <div className="grid gap-3 sm:grid-cols-3">
              <div className="rounded-2xl border border-border bg-background p-4">
                <div className="mb-2 inline-flex h-8 w-8 items-center justify-center rounded-xl bg-primary/10 text-primary">
                  <Tag className="h-4 w-4" />
                </div>
                <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Category
                </div>
                <div className="mt-1 text-sm font-medium text-foreground">
                  {displayContract.category || "Uncategorized"}
                </div>
              </div>

              <div className="rounded-2xl border border-border bg-background p-4">
                <div className="mb-2 inline-flex h-8 w-8 items-center justify-center rounded-xl bg-primary/10 text-primary">
                  <Globe className="h-4 w-4" />
                </div>
                <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Network
                </div>
                <div className="mt-1 text-sm font-medium capitalize text-foreground">
                  {displayContract.network}
                </div>
              </div>

              <div className="rounded-2xl border border-border bg-background p-4">
                <div className="mb-2 inline-flex h-8 w-8 items-center justify-center rounded-xl bg-primary/10 text-primary">
                  <Layers3 className="h-4 w-4" />
                </div>
                <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Publisher
                </div>
                <div className="mt-1 truncate text-sm font-medium text-foreground">
                  {displayContract.publisher_id}
                </div>
              </div>
            </div>

            <section className="rounded-3xl border border-border bg-background p-4 sm:p-5">
              <div className="mb-4 flex items-center justify-between gap-3">
                <div>
                  <h3 className="text-base font-semibold text-foreground">
                    Methods Preview
                  </h3>
                  <p className="text-sm text-muted-foreground">
                    First five ABI functions exposed by this contract.
                  </p>
                </div>
                <span className="rounded-full border border-border px-3 py-1 text-xs font-medium text-muted-foreground">
                  {methodNames.length}/5 shown
                </span>
              </div>

              {isAbiLoading ? (
                <div className="flex items-center gap-2 rounded-2xl border border-dashed border-border px-4 py-5 text-sm text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Loading ABI methods…
                </div>
              ) : abiError ? (
                <div className="rounded-2xl border border-dashed border-border px-4 py-5 text-sm text-muted-foreground">
                  Unable to load ABI methods for this contract right now.
                </div>
              ) : methodNames.length > 0 ? (
                <ol className="space-y-2">
                  {methodNames.map((methodName) => (
                    <li
                      key={methodName}
                      className="flex items-center justify-between gap-3 rounded-2xl border border-border bg-card px-4 py-3"
                    >
                      <code className="truncate text-sm font-medium text-foreground">
                        {methodName}
                      </code>
                      <span className="text-xs text-muted-foreground">
                        method
                      </span>
                    </li>
                  ))}
                </ol>
              ) : (
                <div className="rounded-2xl border border-dashed border-border px-4 py-5 text-sm text-muted-foreground">
                  No ABI methods are available for preview on this contract yet.
                </div>
              )}
            </section>
          </div>

          <div className="flex flex-col gap-3 border-t border-border px-5 py-4 sm:flex-row sm:items-center sm:justify-between sm:px-6">
            <p className="text-xs text-muted-foreground">
              Press <span className="font-mono text-foreground">Esc</span> to
              close.
            </p>
            <Link
              href={`/contracts/${contract.id}`}
              onClick={onClose}
              className="inline-flex items-center justify-center gap-2 rounded-xl bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90"
            >
              View Full Details
              <ExternalLink className="h-4 w-4" />
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
}
