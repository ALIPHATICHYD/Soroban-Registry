import React from "react";
import { ActivityEvent } from "@/types/publisher";
import { CheckCircle, XCircle, FileCode, Clock } from "lucide-react";

interface PublisherActivityTimelineProps {
  activity: ActivityEvent[];
}

const getRelativeTime = (timestamp: string) => {
  const now = new Date();
  const date = new Date(timestamp);
  const diffInSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (diffInSeconds < 60) return `${diffInSeconds}s ago`;
  const diffInMinutes = Math.floor(diffInSeconds / 60);
  if (diffInMinutes < 60) return `${diffInMinutes}m ago`;
  const diffInHours = Math.floor(diffInMinutes / 60);
  if (diffInHours < 24) return `${diffInHours}h ago`;
  const diffInDays = Math.floor(diffInHours / 24);
  if (diffInDays < 30) return `${diffInDays}d ago`;
  const diffInMonths = Math.floor(diffInDays / 30);
  if (diffInMonths < 12) return `${diffInMonths}mo ago`;
  return `${Math.floor(diffInMonths / 12)}y ago`;
};

export function PublisherActivityTimeline({
  activity,
}: PublisherActivityTimelineProps) {
  const sortedActivity = [...activity].sort(
    (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime(),
  );

  return (
    <div className="bg-card rounded-2xl shadow-sm border border-border p-6 h-full">
      <h3 className="text-lg font-bold text-foreground mb-6 flex items-center gap-2">
        <Clock className="w-5 h-5 text-muted-foreground" />
        Recent Activity
      </h3>

      <div className="relative border-l border-border ml-3 space-y-8">
        {sortedActivity.length > 0 ? (
          sortedActivity.map((event) => {
            let Icon = FileCode;
            let colorClass =
              "bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400";
            let title = "Published Contract";

            if (event.type === "verification_success") {
              Icon = CheckCircle;
              colorClass =
                "bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400";
              title = "Verification Success";
            } else if (event.type === "verification_failed") {
              Icon = XCircle;
              colorClass =
                "bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400";
              title = "Verification Failed";
            }

            return (
              <div key={event.id} className="relative pl-8">
                <span
                  className={`absolute -left-3 top-0 flex items-center justify-center w-6 h-6 rounded-full ring-4 ring-card ${colorClass}`}
                >
                  <Icon className="w-3.5 h-3.5" />
                </span>

                <div className="flex flex-col sm:flex-row sm:justify-between sm:items-start gap-1">
                  <div>
                    <h4 className="text-sm font-semibold text-foreground">
                      {title}
                    </h4>
                    <p className="text-sm text-muted-foreground mt-0.5">
                      {event.contractName}
                    </p>
                  </div>
                  <time className="text-xs text-muted-foreground font-mono whitespace-nowrap">
                    {getRelativeTime(event.timestamp)}
                  </time>
                </div>
              </div>
            );
          })
        ) : (
          <p className="pl-8 text-sm text-muted-foreground italic">
            No recent activity recorded.
          </p>
        )}
      </div>
    </div>
  );
}
