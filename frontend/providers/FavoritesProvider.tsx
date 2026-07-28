'use client';

import { createContext, useCallback, useEffect, useRef, useState, ReactNode } from 'react';
import { useToast } from '@/hooks/useToast';
import { api } from '@/lib/api';

const STORAGE_KEY = 'soroban_registry_favorites';
const AUTH_TOKEN_KEY = 'soroban_registry_token';
const MAX_FAVORITES = 500;
const RETRY_DELAY_MS = 3000;

export interface FavoritesContextValue {
  favorites: string[];
  toggleFavorite: (id: string) => void;
  isFavorited: (id: string) => boolean;
  favoritesCount: number;
  isLoading: boolean;
  clearAllFavorites: () => void;
}

export const FavoritesContext = createContext<FavoritesContextValue | undefined>(undefined);

interface FavoritesProviderProps {
  children: ReactNode;
}

function deduplicate(arr: string[]): string[] {
  return arr.filter((id, index) => arr.indexOf(id) === index);
}

function getAuthToken(): string | null {
  if (typeof window === 'undefined') return null;
  try {
    return localStorage.getItem(AUTH_TOKEN_KEY);
  } catch {
    return null;
  }
}

function readFromLocalStorage(): string[] {
  if (typeof window === 'undefined') return [];
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return [];
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item): item is string => typeof item === 'string');
  } catch {
    return [];
  }
}

function writeToLocalStorage(favorites: string[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(favorites));
}

function removeFromLocalStorage(): void {
  localStorage.removeItem(STORAGE_KEY);
}

async function fetchBackendFavorites(token: string): Promise<string[]> {
  const prefs = await api.getPreferences(token);
  return prefs.favorites;
}

async function patchBackendFavorites(token: string, favorites: string[]): Promise<void> {
  await api.updatePreferences(token, favorites);
}

export default function FavoritesProvider({ children }: FavoritesProviderProps) {
  const [favorites, setFavorites] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const { showError, showWarning } = useToast();

  // Track the previous auth token to detect login/logout transitions
  const prevTokenRef = useRef<string | null>(null);

  // On mount: load favorites from localStorage or backend
  useEffect(() => {
    const token = getAuthToken();
    prevTokenRef.current = token;

    if (token) {
      // Authenticated: fetch from backend as authoritative source
      // Defer setState to avoid synchronous setState during effect
      requestAnimationFrame(() => setIsLoading(true));
      fetchBackendFavorites(token)
        .then((backendFavorites) => {
          setFavorites(deduplicate(backendFavorites));
        })
        .catch(() => {
          // Fall back to localStorage if backend fetch fails
          const localFavorites = readFromLocalStorage();
          setFavorites(deduplicate(localFavorites));
        })
        .finally(() => {
          setIsLoading(false);
        });
    } else {
      // Guest: read from localStorage
      try {
        const localFavorites = readFromLocalStorage();
        requestAnimationFrame(() => setFavorites(deduplicate(localFavorites)));
      } catch {
        showWarning("Favorites won't be saved — browser storage is unavailable");
        requestAnimationFrame(() => setFavorites([]));
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Detect auth state changes (guest → authenticated or logout)
  useEffect(() => {
    const checkAuthChange = () => {
      const currentToken = getAuthToken();
      const prevToken = prevTokenRef.current;

      if (currentToken === prevToken) return;

      if (currentToken && !prevToken) {
        // Guest → authenticated: merge localStorage favorites with backend
        const localFavorites = readFromLocalStorage();
        setIsLoading(true);
        fetchBackendFavorites(currentToken)
          .then((backendFavorites) => {
            const merged = deduplicate([...localFavorites, ...backendFavorites]);
            setFavorites(merged);
            return patchBackendFavorites(currentToken, merged);
          })
          .catch(() => {
            // If merge fails, keep local favorites
          })
          .finally(() => {
            setIsLoading(false);
          });
      } else if (!currentToken && prevToken) {
        // Logout: clear in-memory state and localStorage
        setFavorites([]);
        try {
          removeFromLocalStorage();
        } catch {
          // Ignore storage errors on logout
        }
      }

      prevTokenRef.current = currentToken;
    };

    // Poll for auth state changes (storage events don't fire in same tab)
    const interval = setInterval(checkAuthChange, 1000);
    // Also listen for storage events from other tabs
    const onStorage = (e: StorageEvent) => {
      if (e.key === AUTH_TOKEN_KEY) checkAuthChange();
    };
    window.addEventListener('storage', onStorage);

    return () => {
      clearInterval(interval);
      window.removeEventListener('storage', onStorage);
    };
  }, []);

  const toggleFavorite = useCallback((id: string) => {
    setFavorites((prev) => {
      const isCurrentlyFavorited = prev.includes(id);
      const optimisticFavorites = isCurrentlyFavorited
        ? prev.filter((fav) => fav !== id)
        : deduplicate([...prev, id]);

      // Enforce max 500 entries
      const capped = optimisticFavorites.slice(0, MAX_FAVORITES);

      // Write to localStorage
      try {
        writeToLocalStorage(capped);
      } catch {
        showWarning("Favorites won't be saved — browser storage is unavailable");
      }

      // Sync to backend if authenticated
      const token = getAuthToken();
      if (token) {
        patchBackendFavorites(token, capped).catch(() => {
          // Retry once after 3 seconds
          setTimeout(() => {
            const retryToken = getAuthToken();
            if (!retryToken) return;
            patchBackendFavorites(retryToken, capped).catch(() => {
              // Revert on second failure
              setFavorites(prev);
              try {
                writeToLocalStorage(prev);
              } catch {
                // Ignore storage errors during revert
              }
              showError('Failed to save favorites. Please try again.');
            });
          }, RETRY_DELAY_MS);
        });
      }

      return capped;
    });
  }, [showError, showWarning]);

  const isFavorited = useCallback((id: string) => favorites.includes(id), [favorites]);

  const clearAllFavorites = useCallback(() => {
    setFavorites([]);

    // Remove from localStorage
    try {
      removeFromLocalStorage();
    } catch {
      // Ignore storage errors
    }

    // Sync to backend if authenticated
    const token = getAuthToken();
    if (token) {
      patchBackendFavorites(token, []).catch(() => {
        showError('Failed to clear favorites. Please try again.');
      });
    }
  }, [showError]);

  const value: FavoritesContextValue = {
    favorites,
    toggleFavorite,
    isFavorited,
    favoritesCount: favorites.length,
    isLoading,
    clearAllFavorites,
  };

  return (
    <FavoritesContext.Provider value={value}>
      {children}
    </FavoritesContext.Provider>
  );
}
