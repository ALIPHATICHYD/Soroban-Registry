import React from 'react';
import { Check, ChevronDown, RotateCcw } from 'lucide-react';

interface FilterOption {
  value: string;
  label: string;
  count?: number;
}

interface MultiSelectDropdownProps {
  label: string;
  placeholder: string;
  options: FilterOption[];
  selectedValues: string[];
  onToggle: (value: string) => void;
  onClear: () => void;
}

interface FilterPanelProps {
  categories: FilterOption[];
  selectedCategories: string[];
  onToggleCategory: (value: string) => void;
  onClearCategories: () => void;
  networks: FilterOption[];
  selectedNetworks: string[];
  onToggleNetwork: (value: string) => void;
  onClearNetworks: () => void;
  languages: string[];
  selectedLanguages: string[];
  onToggleLanguage: (value: string) => void;
  author: string;
  onAuthorChange: (value: string) => void;
  verifiedOnly: boolean;
  onVerifiedChange: (value: boolean) => void;
  favoritesOnly?: boolean;
  onFavoritesChange?: (value: boolean) => void;
  activeFilterCount: number;
  onResetAll: () => void;
}

function getSummaryText(
  selectedValues: string[],
  options: FilterOption[],
  placeholder: string,
) {
  if (selectedValues.length === 0) {
    return placeholder;
  }

  const selectedLabels = options
    .filter((option) => selectedValues.includes(option.value))
    .map((option) => option.label);

  if (selectedLabels.length <= 2) {
    return selectedLabels.join(', ');
  }

  return `${selectedLabels.slice(0, 2).join(', ')} +${selectedLabels.length - 2}`;
}

function MultiSelectDropdown({
  label,
  placeholder,
  options,
  selectedValues,
  onToggle,
  onClear,
}: MultiSelectDropdownProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const containerRef = React.useRef<HTMLDivElement | null>(null);

  React.useEffect(() => {
    if (!isOpen) {
      return undefined;
    }

    const handlePointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handlePointerDown);
    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('mousedown', handlePointerDown);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  return (
    <div className="space-y-2" ref={containerRef}>
      <div className="flex items-center justify-between">
        <label className="text-sm font-medium text-foreground">{label}</label>
        {selectedValues.length > 0 && (
          <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[11px] font-semibold text-primary">
            {selectedValues.length} selected
          </span>
        )}
      </div>

      <div className="relative">
        <button
          type="button"
          onClick={() => setIsOpen((current) => !current)}
          aria-expanded={isOpen}
          className="flex w-full items-center justify-between rounded-xl border border-border bg-background px-3 py-2.5 text-left text-sm text-foreground shadow-sm transition-colors hover:border-primary/40"
        >
          <span className={selectedValues.length > 0 ? 'text-foreground' : 'text-muted-foreground'}>
            {getSummaryText(selectedValues, options, placeholder)}
          </span>
          <ChevronDown
            className={`h-4 w-4 text-muted-foreground transition-transform ${isOpen ? 'rotate-180' : ''}`}
          />
        </button>

        {isOpen && (
          <div className="absolute z-20 mt-2 w-full overflow-hidden rounded-2xl border border-border bg-popover shadow-xl">
            <div className="max-h-64 overflow-y-auto p-2">
              {options.map((option) => {
                const isSelected = selectedValues.includes(option.value);

                return (
                  <button
                    key={option.value}
                    type="button"
                    onClick={() => onToggle(option.value)}
                    className={`flex w-full items-center justify-between rounded-xl px-3 py-2 text-sm transition-colors ${
                      isSelected ? 'bg-primary/10 text-foreground' : 'text-muted-foreground hover:bg-accent hover:text-foreground'
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <span
                        className={`flex h-4 w-4 items-center justify-center rounded border ${
                          isSelected
                            ? 'border-primary bg-primary text-primary-foreground'
                            : 'border-input bg-background'
                        }`}
                      >
                        {isSelected && <Check className="h-3 w-3" />}
                      </span>
                      <span>{option.label}</span>
                    </div>
                    {option.count !== undefined && (
                      <span className="rounded-full bg-muted px-2 py-0.5 text-[11px] text-muted-foreground">
                        {option.count}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function CheckboxGroup({
  title,
  options,
  selected,
  onToggle,
}: {
  title: string;
  options: string[];
  selected: string[];
  onToggle: (value: string) => void;
}) {
  return (
    <div>
      <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3">
        {title}
      </p>
      <div className="space-y-1.5">
        {options.map((option) => {
          const isSelected = selected.includes(option);
          return (
            <button
              key={option}
              type="button"
              role="checkbox"
              aria-checked={isSelected}
              onClick={() => onToggle(option)}
              className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm capitalize transition-all ${
                isSelected
                  ? 'bg-primary/10 text-primary font-medium'
                  : 'text-muted-foreground hover:text-foreground hover:bg-accent'
              }`}
            >
              <div
                className={`w-4 h-4 rounded border flex items-center justify-center flex-shrink-0 transition-colors ${
                  isSelected ? 'bg-primary border-primary' : 'border-border'
                }`}
              >
                {isSelected && (
                  <Check className="w-3 h-3 text-primary-foreground" />
                )}
              </div>
              {option}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export function FilterPanel({
  categories,
  selectedCategories,
  onToggleCategory,
  onClearCategories,
  networks,
  selectedNetworks,
  onToggleNetwork,
  onClearNetworks,
  languages,
  selectedLanguages,
  onToggleLanguage,
  author,
  onAuthorChange,
  verifiedOnly,
  onVerifiedChange,
  favoritesOnly,
  onFavoritesChange,
  activeFilterCount,
  onResetAll,
}: FilterPanelProps) {
  const [expandedSections, setExpandedSections] = React.useState<Record<string, boolean>>({
    categories: true,
    networks: true,
    languages: true,
    other: true,
  });

  const toggleSection = (section: string) => {
    setExpandedSections((current) => ({
      ...current,
      [section]: !current[section],
    }));
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-foreground">Filters</h3>
        {activeFilterCount > 0 && (
          <button
            type="button"
            onClick={onResetAll}
            className="text-xs text-muted-foreground hover:text-primary flex items-center gap-1 transition-colors"
          >
            <RotateCcw className="w-3 h-3" />
            Reset all
          </button>
        )}
      </div>

      <CheckboxGroup
        title="Network"
        options={networks.map((n) => n.value)}
        selected={selectedNetworks}
        onToggle={onToggleNetwork}
      />

      <div className="space-y-3">
        <button
          type="button"
          onClick={() => toggleSection('categories')}
          className="flex w-full items-center justify-between text-sm font-medium text-foreground hover:text-primary transition-colors"
        >
          <span>Categories</span>
          <ChevronDown
            className={`h-4 w-4 text-muted-foreground transition-transform ${expandedSections.categories ? 'rotate-180' : ''}`}
          />
        </button>

        {expandedSections.categories && (
          <MultiSelectDropdown
            label="Category filters"
            placeholder="Choose categories"
            options={categories}
            selectedValues={selectedCategories}
            onToggle={onToggleCategory}
            onClear={onClearCategories}
          />
        )}
      </div>

      <button
        type="button"
        role="checkbox"
        aria-checked={verifiedOnly}
        onClick={() => onVerifiedChange(!verifiedOnly)}
        className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-all ${
          verifiedOnly
            ? 'bg-green-500/10 text-green-600 font-medium'
            : 'text-muted-foreground hover:text-foreground hover:bg-accent'
        }`}
      >
        <div
          className={`w-4 h-4 rounded border flex items-center justify-center flex-shrink-0 transition-colors ${
            verifiedOnly ? 'bg-green-500 border-green-500' : 'border-border'
          }`}
        >
          {verifiedOnly && (
            <Check className="w-3 h-3 text-white" />
          )}
        </div>
        Verified only
      </button>

      {languages.length > 0 && (
        <CheckboxGroup
          title="Languages"
          options={languages}
          selected={selectedLanguages}
          onToggle={onToggleLanguage}
        />
      )}

      <button
        type="button"
        role="checkbox"
        aria-checked={favoritesOnly}
        onClick={() => onFavoritesChange?.(!favoritesOnly)}
        className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-all ${
          favoritesOnly
            ? "bg-yellow-500/10 text-yellow-600 font-medium"
            : "text-muted-foreground hover:text-foreground hover:bg-accent"
        }`}
      >
        <div
          className={`w-4 h-4 rounded border flex items-center justify-center flex-shrink-0 transition-colors ${
            favoritesOnly ? "bg-yellow-500 border-yellow-500" : "border-border"
          }`}
        >
          {favoritesOnly && <Check className="w-3 h-3 text-white" />}
        </div>
        Favorites only
      </button>
    </div>
  );
}
