import { Suspense } from "react";
import { GraphContent } from "./graph-content";
import Navbar from "@/components/Navbar";

export const dynamic = "force-dynamic";

export const metadata = {
  title: "Dependency Graph — Soroban Registry",
  description:
    "Interactive visualization of contract dependencies in the Soroban smart contract ecosystem.",
};

export default function GraphPage() {
  return (
    <div className="min-h-screen bg-background">
      <Navbar />

      <Suspense
        fallback={
          <div className="flex items-center justify-center h-[calc(100vh-4rem)]">
            <div className="text-center">
              <div className="inline-block w-10 h-10 border-4 border-primary border-t-transparent rounded-full animate-spin mb-4" />
              <p className="text-muted-foreground text-sm">
                Loading dependency graph…
              </p>
            </div>
          </div>
        }
      >
        <GraphContent />
      </Suspense>
    </div>
  );
}
