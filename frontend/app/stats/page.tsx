"use client";

import React, { useState } from "react";
import { useStats } from "@/hooks/useStats";
import { TimePeriod } from "@/types/stats";
import StatsSummaryCards from "@/components/stats/StatsSummaryCards";
import CategoryPieChart from "@/components/stats/CategoryPieChart";
import DeploymentsTrendChart from "@/components/stats/DeploymentsTrendChart";
import TopPublishersTable from "@/components/stats/TopPublishersTable";
import NetworkDistribution from "@/components/stats/NetworkDistribution";
import TimePeriodSelector from "@/components/stats/TimePeriodSelector";
import StatsSkeleton from "@/components/stats/StatsSkeleton";
import Navbar from "@/components/Navbar";
import { AlertCircle, RefreshCw } from "lucide-react";
import { useTranslation } from "@/lib/i18n/client";

export default function StatsPage() {
  const { t } = useTranslation("common");
  const [period, setPeriod] = useState<TimePeriod>("30d");
  const { data, loading, error, refetch } = useStats(period);

  if (error) {
    return (
      <div className="min-h-screen bg-background">
        <Navbar />
        <div className="flex items-center justify-center min-h-[calc(100vh-80px)] p-4">
          <div className="bg-card p-8 rounded-2xl shadow-lg max-w-md w-full text-center border border-red-500/20">
            <AlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
            <h2 className="text-xl font-bold text-foreground mb-2">
              {t("stats.failed")}
            </h2>
            <p className="text-muted-foreground mb-6">
              {error.message ||
                "An unexpected error occurred while fetching data."}
            </p>
            <button
              onClick={() => refetch()}
              className="inline-flex items-center px-4 py-2 bg-primary hover:opacity-90 text-primary-foreground font-medium rounded-lg transition-colors"
            >
              <RefreshCw className="w-4 h-4 mr-2" />
              {t("stats.tryAgain")}
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background">
      <Navbar />
      <div className="py-8 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto space-y-8">
          {/* Header Section */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
            <div>
              <h1 className="text-3xl font-bold text-foreground">
                {t("stats.title")}
              </h1>
              <p className="mt-1 text-sm text-muted-foreground">
                {t("stats.subtitle")}
              </p>
            </div>
            <TimePeriodSelector
              selectedPeriod={period}
              onPeriodChange={setPeriod}
            />
          </div>

          {loading || !data ? (
            <StatsSkeleton />
          ) : (
            <div className="space-y-6 animate-in fade-in duration-500">
              {/* Summary Cards */}
              <StatsSummaryCards data={data} />

              {/* Charts Row */}
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 h-[400px]">
                <DeploymentsTrendChart data={data.deploymentsTrend} />
                <CategoryPieChart data={data.contractsByCategory} />
              </div>

              {/* Bottom Section */}
              <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <div className="lg:col-span-2">
                  <TopPublishersTable data={data.topPublishers} />
                </div>
                <div>
                  <NetworkDistribution data={data.networkBreakdown} />
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
