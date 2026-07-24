export function requestDesktopNotification(): void {
  if ("Notification" in window && Notification.permission === "default") {
    Notification.requestPermission();
  }
}

interface NotificationOptions {
  body?: string;
  tag?: string;
  requireInteraction?: boolean;
  icon?: string;
  badge?: string;
}

export async function sendDesktopNotification(
  title: string,
  options?: NotificationOptions,
): Promise<void> {
  if (!("Notification" in window)) {
    console.warn("Desktop notifications not supported");
    return;
  }

  if (Notification.permission !== "granted") {
    return;
  }

  try {
    const notificationOptions: NotificationOptions = {
      icon: "/soroban-icon.png",
      badge: "/soroban-badge.png",
      ...options,
    };

    if ("serviceWorker" in navigator) {
      const registration = await navigator.serviceWorker.ready;
      registration.showNotification(
        title,
        notificationOptions as NotificationOptions &
          Parameters<ServiceWorkerRegistration["showNotification"]>[1],
      );
    } else {
      new Notification(title, notificationOptions);
    }
  } catch (error) {
    console.error("Failed to send desktop notification:", error);
  }
}

export function getWebsiteNotificationPermission(): NotificationPermission {
  if ("Notification" in window) {
    return Notification.permission;
  }
  return "denied";
}
