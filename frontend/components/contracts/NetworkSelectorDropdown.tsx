import { useEffect, useMemo, useRef, useState } from "react";
import { Check, ChevronDown, FlaskConical, Globe, Rocket } from "lucide-react";
import type { ContractSearchParams } from "@/types";

type NetworkFilter = NonNullable<ContractSearchParams["network"]>;

type NetworkOption = {
  value: NetworkFilter;
  label: string;
  description: string;
  status: "online" | "offline";
  Icon: typeof Globe;
};

interface NetworkSelectorDropdownProps {
  selectedNetworks: NetworkFilter[];
  onToggleNetwork: (value: NetworkFilter) => void;
  onSelectAll: () => void;
}

const NETWORK_OPTIONS: Array<Omit<NetworkOption, "status">> = [
  {
    value: "mainnet",
    label: "Mainnet",
    description: "Production deployments",
    Icon: Globe,
  },
  {
    value: "testnet",
    label: "Testnet",
    description: "Testing and staging",
    Icon: FlaskConical,
  },
  {
    value: "futurenet",
    label: "Futurenet",
    description: "Preview network",
    Icon: Rocket,
  },
];

export const ALL_NETWORK_FILTERS: NetworkFilter[] = NETWORK_OPTIONS.map(
  ({ value }) => value,
);

export function NetworkSelectorDropdown({
  selectedNetworks,
  onToggleNetwork,
  onSelectAll,
}: NetworkSelectorDropdownProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [isBrowserOnline, setIsBrowserOnline] = useState(true);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    const updateStatus = () => setIsBrowserOnline(window.navigator.onLine);
    updateStatus();
    window.addEventListener("online", updateStatus);
    window.addEventListener("offline", updateStatus);

    return () => {
      window.removeEventListener("online", updateStatus);
      window.removeEventListener("offline", updateStatus);
    };
  }, []);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const handlePointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [isOpen]);

  const networkOptions = useMemo<NetworkOption[]>(
    () =>
      NETWORK_OPTIONS.map((option) => ({
        ...option,
        status: isBrowserOnline ? "online" : "offline",
      })),
    [isBrowserOnline],
  );

  const triggerLabel = useMemo(() => {
    if (selectedNetworks.length === 0) {
      return "No networks";
    }

    if (selectedNetworks.length === ALL_NETWORK_FILTERS.length) {
      return "All networks";
    }

    if (selectedNetworks.length === 1) {
      return (
        networkOptions.find(({ value }) => value === selectedNetworks[0])
          ?.label ?? "1 network"
      );
    }

    return `${selectedNetworks.length} networks`;
  }, [networkOptions, selectedNetworks]);

  return (
    <div ref={rootRef} className="relative w-full sm:w-auto">
      <button
        type="button"
        onClick={() => setIsOpen((current) => !current)}
        className="inline-flex w-full sm:w-[17rem] items-center justify-between gap-3 rounded-lg border border-border bg-background px-3 py-3 text-sm text-foreground shadow-sm transition-colors hover:bg-accent focus:outline-none focus:ring-2 focus:ring-primary/20"
        aria-haspopup="menu"
        aria-expanded={isOpen}
        aria-label="Select deployment networks"
      >
        <span className="flex min-w-0 items-center gap-2">
          <span className="flex h-8 w-8 items-center justify-center rounded-full bg-muted text-muted-foreground">
            <Globe className="h-4 w-4" />
          </span>
          <span className="min-w-0 text-left">
            <span className="block text-xs uppercase tracking-[0.18em] text-muted-foreground">
              Networks
            </span>
            <span className="block truncate font-medium text-foreground">
              {triggerLabel}
            </span>
          </span>
        </span>
        <ChevronDown
          className={`h-4 w-4 shrink-0 text-muted-foreground transition-transform ${isOpen ? "rotate-180" : ""}`}
        />
      </button>

      {isOpen && (
        <div className="absolute left-0 right-0 top-[calc(100%+0.5rem)] z-20 overflow-hidden rounded-2xl border border-border bg-background shadow-xl sm:right-auto sm:w-[22rem]">
          <div className="border-b border-border px-4 py-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm font-semibold text-foreground">
                  Deployment networks
                </p>
                <p className="text-xs text-muted-foreground">
                  Results update as soon as you change the selection.
                </p>
              </div>
              <button
                type="button"
                onClick={onSelectAll}
                className="text-xs font-medium text-primary hover:opacity-80"
              >
                Select all
              </button>
            </div>
          </div>

          <div className="p-2">
            {networkOptions.map(
              ({ value, label, description, status, Icon }) => {
                const selected = selectedNetworks.includes(value);

                return (
                  <label
                    key={value}
                    className={`flex cursor-pointer items-center gap-3 rounded-xl px-3 py-3 transition-colors ${
                      selected ? "bg-accent" : "hover:bg-accent/70"
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={selected}
                      onChange={() => onToggleNetwork(value)}
                      className="sr-only"
                    />
                    <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
                      <Icon className="h-4 w-4" />
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="flex items-center gap-2">
                        <span className="font-medium capitalize text-foreground">
                          {label}
                        </span>
                        <span className="inline-flex items-center gap-1 rounded-full border border-border px-2 py-0.5 text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
                          <span
                            className={`h-2 w-2 rounded-full ${
                              status === "online"
                                ? "bg-emerald-500"
                                : "bg-rose-500"
                            }`}
                          />
                          {status}
                        </span>
                      </span>
                      <span className="block text-xs text-muted-foreground">
                        {description}
                      </span>
                    </span>
                    <span
                      className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-full border ${
                        selected
                          ? "border-primary bg-primary text-primary-foreground"
                          : "border-border text-transparent"
                      }`}
                      aria-hidden="true"
                    >
                      <Check className="h-3.5 w-3.5" />
                    </span>
                  </label>
                );
              },
            )}
          </div>
        </div>
      )}
    </div>
  );
}
