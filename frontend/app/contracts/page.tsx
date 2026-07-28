import { Suspense } from "react";
import { ContractsContent } from "./contracts-content";
import Navbar from "@/components/Navbar";

export const dynamic = "force-dynamic";

export default function ContractsPage() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />

      <Suspense
        fallback={
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
            <div className="text-center py-20">
              <div className="inline-block w-10 h-10 border-4 border-primary border-t-transparent rounded-full animate-spin" />
              <p className="mt-4 text-sm text-muted-foreground">
                Loading contracts...
              </p>
            </div>
          </div>
        }
      >
        <ContractsContent />
      </Suspense>
    </div>
  );
}
