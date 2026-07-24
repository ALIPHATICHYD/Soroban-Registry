"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useAppDispatch, useAppSelector } from "@/store/hooks";
import {
  addFavorite,
  removeFavorite,
  setFavorites,
} from "@/store/slices/favoritesSlice";
import { useToast } from "./useToast";
import { api } from "@/lib/api";

const STORAGE_KEY = "soroban_registry_favorites";
const AUTH_TOKEN_KEY = "soroban_registry_token";
const MAX_FAVORITES = 500;
const RETRY_DELAY_MS = 3000;

function deduplicate(arr: string[]): string[] {
  return arr.filter((id, index) => arr.indexOf(id) === index);
}

function getAuthToken(): string | null {
  if (typeof window === "undefined") return null;
  try {
    return localStorage.getItem(AUTH_TOKEN_KEY);
  } catch {
    return null;
  }
}

function readFromLocalStorage(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return [];
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item): item is string => typeof item === "string");
  } catch {
    return [];
  }
}

function writeToLocalStorage(favorites: string[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(favorites));
  } catch {
    // ignore
  }
}

export function useFavorites() {
  const dispatch = useAppDispatch();
  const items = useAppSelector((s) => s.favorites.items);
  const [isLoading, setIsLoading] = useState(false);
  const { showError, showWarning } = useToast();

  const prevTokenRef = useRef<string | null>(null);

  // Load initial favorites
  useEffect(() => {
    const token = getAuthToken();
    prevTokenRef.current = token;

    if (token) {
      setIsLoading(true);
      api
        .getPreferences(token)
        .then((prefs) => {
          dispatch(setFavorites(deduplicate(prefs.favorites)));
        })
        .catch(() => {
          const local = readFromLocalStorage();
          dispatch(setFavorites(deduplicate(local)));
        })
        .finally(() => setIsLoading(false));
    } else {
      const local = readFromLocalStorage();
      if (local.length === 0) {
        try {
          // warn only if storage is unavailable
        } catch {
          showWarning(
            "Favorites won't be saved — browser storage is unavailable",
          );
        }
      }
      dispatch(setFavorites(deduplicate(local)));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Auth change detection (merge on login, clear on logout)
  useEffect(() => {
    const checkAuthChange = () => {
      const currentToken = getAuthToken();
      const prevToken = prevTokenRef.current;
      if (currentToken === prevToken) return;

      if (currentToken && !prevToken) {
        // Guest -> authenticated: merge local with backend
        const localFavorites = readFromLocalStorage();
        setIsLoading(true);
        api
          .getPreferences(currentToken)
          .then((prefs) => {
            const merged = deduplicate([
              ...localFavorites,
              ...prefs.favorites,
            ]).slice(0, MAX_FAVORITES);
            dispatch(setFavorites(merged));
            writeToLocalStorage(merged);
            return api.updatePreferences(currentToken, merged);
          })
          .catch(() => {})
          .finally(() => setIsLoading(false));
      } else if (!currentToken && prevToken) {
        // logout
        dispatch(setFavorites([]));
        try {
          localStorage.removeItem(STORAGE_KEY);
        } catch {}
      }

      prevTokenRef.current = currentToken;
    };

    const interval = setInterval(checkAuthChange, 1000);
    const onStorage = (e: StorageEvent) => {
      if (e.key === AUTH_TOKEN_KEY) checkAuthChange();
    };
    window.addEventListener("storage", onStorage);
    return () => {
      clearInterval(interval);
      window.removeEventListener("storage", onStorage);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const toggleFavorite = useCallback(
    (id: string) => {
      const currently = items.includes(id);
      const optimistic = currently
        ? items.filter((i) => i !== id)
        : deduplicate([...items, id]).slice(0, MAX_FAVORITES);
      dispatch(setFavorites(optimistic));
      writeToLocalStorage(optimistic);

      const token = getAuthToken();
      if (token) {
        api.updatePreferences(token, optimistic).catch(() => {
          setTimeout(() => {
            const retryToken = getAuthToken();
            if (!retryToken) return;
            api.updatePreferences(retryToken, optimistic).catch(() => {
              // Revert on failure
              dispatch(setFavorites(items));
              writeToLocalStorage(items);
              showError("Failed to save favorites. Please try again.");
            });
          }, RETRY_DELAY_MS);
        });
      }
    },
    [dispatch, items, showError],
  );

  const isFavorited = useCallback((id: string) => items.includes(id), [items]);

  const clearAllFavorites = useCallback(() => {
    dispatch(setFavorites([]));
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {}
    const token = getAuthToken();
    if (token) {
      api
        .updatePreferences(token, [])
        .catch(() => showError("Failed to clear favorites. Please try again."));
    }
  }, [dispatch, showError]);

  return {
    favorites: items,
    toggleFavorite,
    isFavorited,
    favoritesCount: items.length,
    isLoading,
    clearAllFavorites,
  };
}
