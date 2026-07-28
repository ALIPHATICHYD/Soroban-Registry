"use client";

import { useNotificationPreferences } from "@/hooks/useNotificationPreferences";

export default function NotificationPreferencesPanel() {
  const {
    preferences,
    isLoaded,
    toggleDeploymentNotifications,
    toggleUpdateNotifications,
    toggleDesktopNotifications,
    toggleSoundNotification,
    updateUpdateTypes,
  } = useNotificationPreferences();

  if (!isLoaded) {
    return (
      <div className="p-4 text-sm text-muted-foreground">
        Loading preferences...
      </div>
    );
  }

  const updateTypeOptions = [
    { id: "verification_status", label: "Verification Status" },
    { id: "security_audit", label: "Security Audit" },
    { id: "deprecation", label: "Deprecation" },
    { id: "breaking_change", label: "Breaking Changes" },
  ];

  const handleUpdateTypeChange = (typeId: string) => {
    const types = preferences.updateTypes.includes(typeId)
      ? preferences.updateTypes.filter((t) => t !== typeId)
      : [...preferences.updateTypes, typeId];
    updateUpdateTypes(types);
  };

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-4">Notification Preferences</h3>
        <p className="text-sm text-muted-foreground mb-6">
          Customize how and when you receive notifications about new contracts
          and updates.
        </p>
      </div>

      {/* Desktop Notifications */}
      <div className="space-y-3">
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={preferences.enableDesktopNotifications}
            onChange={toggleDesktopNotifications}
            className="w-4 h-4 rounded border-border cursor-pointer"
          />
          <span className="font-medium">Enable Desktop Notifications</span>
        </label>
        <p className="text-xs text-muted-foreground ml-7">
          Receive browser notifications even when you&apos;re not actively using
          the app
        </p>
      </div>

      {/* Deployment Notifications */}
      <div className="space-y-3">
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={preferences.enableDeploymentNotifications}
            onChange={toggleDeploymentNotifications}
            disabled={!preferences.enableDesktopNotifications}
            className="w-4 h-4 rounded border-border cursor-pointer disabled:opacity-50"
          />
          <span
            className={
              preferences.enableDesktopNotifications
                ? "font-medium"
                : "font-medium text-muted-foreground"
            }
          >
            New Contract Deployments
          </span>
        </label>
        <p className="text-xs text-muted-foreground ml-7">
          Get notified when new contracts are deployed
        </p>
      </div>

      {/* Update Notifications */}
      <div className="space-y-3">
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={preferences.enableUpdateNotifications}
            onChange={toggleUpdateNotifications}
            disabled={!preferences.enableDesktopNotifications}
            className="w-4 h-4 rounded border-border cursor-pointer disabled:opacity-50"
          />
          <span
            className={
              preferences.enableDesktopNotifications
                ? "font-medium"
                : "font-medium text-muted-foreground"
            }
          >
            Contract Updates
          </span>
        </label>
        <p className="text-xs text-muted-foreground ml-7">
          Get notified about changes to existing contracts
        </p>

        {preferences.enableUpdateNotifications &&
          preferences.enableDesktopNotifications && (
            <div className="ml-7 space-y-2 pt-2 border-t border-border">
              <p className="text-xs font-medium text-muted-foreground">
                Notify me about:
              </p>
              {updateTypeOptions.map((option) => (
                <label
                  key={option.id}
                  className="flex items-center gap-2 cursor-pointer"
                >
                  <input
                    type="checkbox"
                    checked={preferences.updateTypes.includes(option.id)}
                    onChange={() => handleUpdateTypeChange(option.id)}
                    className="w-3 h-3 rounded border-border cursor-pointer"
                  />
                  <span className="text-xs">{option.label}</span>
                </label>
              ))}
            </div>
          )}
      </div>

      {/* Sound Notification */}
      <div className="space-y-3">
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={preferences.enableSoundNotification}
            onChange={toggleSoundNotification}
            disabled={!preferences.enableDesktopNotifications}
            className="w-4 h-4 rounded border-border cursor-pointer disabled:opacity-50"
          />
          <span
            className={
              preferences.enableDesktopNotifications
                ? "font-medium"
                : "font-medium text-muted-foreground"
            }
          >
            Sound Notification
          </span>
        </label>
        <p className="text-xs text-muted-foreground ml-7">
          Play a sound when new notifications arrive
        </p>
      </div>
    </div>
  );
}
