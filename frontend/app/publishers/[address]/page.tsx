"use client";

import { Suspense } from "react";
import { useQuery } from "@tanstack/react-query";
import { useParams } from "next/navigation";
import { getPublisher } from "@/lib/api/publishers";
import { PublisherHeader } from "@/components/publisher/PublisherHeader";
import { PublisherStats } from "@/components/publisher/PublisherStats";
import { PublisherContractsList } from "@/components/publisher/PublisherContractsList";
import { PublisherActivityTimeline } from "@/components/publisher/PublisherActivityTimeline";
import Navbar from "@/components/Navbar";
import { AlertCircle } from "lucide-react";

function PublisherProfileContent() {
  const params = useParams<{ address?: string | string[] }>() ?? {};
  const addressParam = params.address;
  const address = Array.isArray(addressParam) ? addressParam[0] : addressParam;

  const {
    data: publisher,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["publisher", address],
    queryFn: () => getPublisher(address!),
    enabled: !!address,
  });

  if (!address) {
    return (
      <div className="min-h-screen bg-background text-foreground">
        <Navbar />
        <div className="max-w-4xl mx-auto px-4 py-10">
          <div className="rounded-2xl border border-border bg-card p-6">
            <div className="text-sm font-semibold text-foreground">
              Missing publisher address
            </div>
            <div className="mt-1 text-sm text-muted-foreground">
              Open this page from a publisher link.
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-pulse">
        <div className="bg-muted h-64 rounded-xl mb-8"></div>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="bg-muted h-32 rounded-xl"></div>
          ))}
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          <div className="lg:col-span-2 bg-muted h-96 rounded-xl"></div>
          <div className="bg-muted h-96 rounded-xl"></div>
        </div>
      </div>
    );
  }

  if (error || !publisher) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 text-center">
        <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-red-100 text-red-600 mb-4">
          <AlertCircle className="w-8 h-8" />
        </div>
        <h2 className="text-2xl font-bold text-foreground mb-2">
          Publisher Not Found
        </h2>
        <p className="text-muted-foreground max-w-md mx-auto">
          We couldn&apos;t find a publisher with address{" "}
          <span className="font-mono bg-accent px-1 py-0.5 rounded">
            {address}
          </span>
          .
        </p>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8 animate-in fade-in duration-500">
      <PublisherHeader publisher={publisher} />

      <PublisherStats publisher={publisher} />

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        <div className="lg:col-span-2">
          <PublisherContractsList contracts={publisher.contracts} />
        </div>

        <div className="lg:col-span-1">
          <PublisherActivityTimeline activity={publisher.activity} />
        </div>
      </div>
    </div>
  );
}

export default function PublisherPage() {
  return (
    <div className="min-h-screen bg-background text-foreground pb-12">
      <Navbar />
      <Suspense fallback={null}>
        <PublisherProfileContent />
      </Suspense>
    </div>
  );
}
