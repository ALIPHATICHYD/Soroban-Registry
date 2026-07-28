"use client";

import { useState, useCallback } from "react";
import { NotificationPreferences } from "@/types/realtime";

const DEFAULT_PREFERENCES: NotificationPreferences = {
  enableDeploymentNotifications: true,
  enableUpdateNotifications: true,
  enableDesktopNotifications: true,
  enableSoundNotification: false,
  updateTypes: ["verification_status", "security_audit"],
};

const STORAGE_KEY = "notification-preferences";

function loadPreferencesFromStorage(): NotificationPreferences {
  if (typeof window === "undefined") return DEFAULT_PREFERENCES;
  const stored = localStorage.getItem(STORAGE_KEY);
  if (!stored) return DEFAULT_PREFERENCES;
  try {
    return JSON.parse(stored) as NotificationPreferences;
  } catch {
    return DEFAULT_PREFERENCES;
  }
}

export function useNotificationPreferences() {
  const [preferences, setPreferences] = useState<NotificationPreferences>(
    loadPreferencesFromStorage,
  );
  const isLoaded = true;

  const updatePreferences = useCallback(
    (updates: Partial<NotificationPreferences>) => {
      setPreferences((prev) => {
        const updated = { ...prev, ...updates };
        localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
        return updated;
      });
    },
    [],
  );

  const toggleDeploymentNotifications = useCallback(() => {
    updatePreferences({
      enableDeploymentNotifications: !preferences.enableDeploymentNotifications,
    });
  }, [preferences.enableDeploymentNotifications, updatePreferences]);

  const toggleUpdateNotifications = useCallback(() => {
    updatePreferences({
      enableUpdateNotifications: !preferences.enableUpdateNotifications,
    });
  }, [preferences.enableUpdateNotifications, updatePreferences]);

  const toggleDesktopNotifications = useCallback(() => {
    updatePreferences({
      enableDesktopNotifications: !preferences.enableDesktopNotifications,
    });
  }, [preferences.enableDesktopNotifications, updatePreferences]);

  const toggleSoundNotification = useCallback(() => {
    updatePreferences({
      enableSoundNotification: !preferences.enableSoundNotification,
    });
  }, [preferences.enableSoundNotification, updatePreferences]);

  const updateUpdateTypes = useCallback(
    (types: string[]) => {
      updatePreferences({ updateTypes: types });
    },
    [updatePreferences],
  );

  return {
    preferences,
    isLoaded,
    updatePreferences,
    toggleDeploymentNotifications,
    toggleUpdateNotifications,
    toggleDesktopNotifications,
    toggleSoundNotification,
    updateUpdateTypes,
  };
}
