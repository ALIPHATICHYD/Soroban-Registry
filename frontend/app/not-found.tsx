import Link from "next/link";
import { SearchX } from "lucide-react";
import Navbar from "@/components/Navbar";

export default function NotFound() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />

      <main className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-24 text-center">
        <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-primary/10 text-primary mb-6">
          <SearchX className="w-10 h-10" aria-hidden="true" />
        </div>

        <h1 className="text-4xl sm:text-5xl font-bold mb-4">Page not found</h1>
        <p className="text-lg text-muted-foreground mb-10">
          The page you are looking for does not exist or may have moved. Try
          browsing contracts or return to the homepage.
        </p>

        <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
          <Link
            href="/contracts"
            className="px-6 py-3 rounded-lg btn-glow text-primary-foreground font-medium"
          >
            Browse Contracts
          </Link>
          <Link
            href="/"
            className="px-6 py-3 rounded-lg border border-border text-foreground hover:bg-muted transition-colors font-medium"
          >
            Go Home
          </Link>
        </div>
      </main>
    </div>
  );
}
