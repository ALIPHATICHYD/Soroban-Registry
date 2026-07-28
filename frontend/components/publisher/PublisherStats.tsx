import React from "react";
import { PublisherResponse } from "@/types/publisher";
import { Award, ShieldCheck, ShieldAlert, BarChart2 } from "lucide-react";

interface PublisherStatsProps {
  publisher: PublisherResponse;
}

export function PublisherStats({ publisher }: PublisherStatsProps) {
  const successRate =
    publisher.totalContracts > 0
      ? (publisher.verifiedContracts / publisher.totalContracts) * 100
      : 0;

  const getSuccessColor = (rate: number) => {
    if (rate >= 70) return "text-green-600 bg-green-50 dark:bg-green-900/20";
    if (rate >= 40) return "text-yellow-600 bg-yellow-50 dark:bg-yellow-900/20";
    return "text-red-600 bg-red-50 dark:bg-red-900/20";
  };

  const statItems = [
    {
      label: "Total Contracts",
      value: publisher.totalContracts,
      icon: BarChart2,
      color: "text-blue-600 bg-blue-50 dark:bg-blue-900/20",
    },
    {
      label: "Verification Success",
      value: `${successRate.toFixed(1)}%`,
      icon: Award,
      color: getSuccessColor(successRate),
    },
    {
      label: "Verified Contracts",
      value: publisher.verifiedContracts,
      icon: ShieldCheck,
      color: "text-green-600 bg-green-50 dark:bg-green-900/20",
    },
    {
      label: "Failed Verifications",
      value: publisher.failedVerifications,
      icon: ShieldAlert,
      color: "text-red-600 bg-red-50 dark:bg-red-900/20",
    },
  ];

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      {statItems.map((item) => (
        <div
          key={item.label}
          className="bg-card p-4 rounded-2xl shadow-sm border border-border hover:shadow-md transition-shadow"
        >
          <div className={`p-3 rounded-lg w-fit mb-3 ${item.color}`}>
            <item.icon className="w-6 h-6" />
          </div>
          <p className="text-sm text-muted-foreground font-medium">
            {item.label}
          </p>
          <p className="text-2xl font-bold text-foreground mt-1">
            {item.value}
          </p>
        </div>
      ))}
    </div>
  );
}
