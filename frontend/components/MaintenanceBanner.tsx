"use client";

import type { MaintenanceWindow } from "@/types";

interface MaintenanceBannerProps {
  window: MaintenanceWindow;
}

export default function MaintenanceBanner({ window }: MaintenanceBannerProps) {
  const formatDate = (date: string) => {
    return new Date(date).toLocaleString();
  };

  return (
    <div className="bg-yellow-500/10 border-l-4 border-yellow-500 p-4 mb-4 rounded-r-xl">
      <div className="flex">
        <div className="flex-shrink-0">
          <svg
            className="h-5 w-5 text-yellow-500"
            viewBox="0 0 20 20"
            fill="currentColor"
          >
            <path
              fillRule="evenodd"
              d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
              clipRule="evenodd"
            />
          </svg>
        </div>
        <div className="ml-3">
          <h3 className="text-sm font-medium text-yellow-600 dark:text-yellow-400">
            Contract in Maintenance Mode
          </h3>
          <div className="mt-2 text-sm text-yellow-600 dark:text-yellow-300">
            <p>{window.message}</p>
            {window.scheduled_end_at && (
              <p className="mt-1">
                Scheduled to resume: {formatDate(window.scheduled_end_at)}
              </p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
