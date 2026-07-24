"use client";

import { useState } from "react";
import { AlertTriangle, RefreshCw, ChevronDown, ChevronUp } from "lucide-react";
import { ErrorFallbackProps } from "./ErrorBoundary";

export default function ErrorFallback({
  error,
  errorInfo,
  resetError,
}: ErrorFallbackProps) {
  const [showDetails, setShowDetails] = useState(false);

  return (
    <div className="min-h-screen flex items-center justify-center p-4 bg-background">
      <div className="max-w-2xl w-full bg-card rounded-2xl shadow-xl p-8">
        <div className="flex items-start gap-4">
          <div className="flex-shrink-0">
            <AlertTriangle className="w-12 h-12 text-red-500" />
          </div>

          <div className="flex-1">
            <h1 className="text-2xl font-bold text-foreground mb-2">
              Something went wrong
            </h1>

            <p className="text-muted-foreground mb-6">
              We encountered an unexpected error. Do not worry, your data is
              safe. You can try refreshing the page or contact support if the
              problem persists.
            </p>

            <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 mb-6">
              <p className="text-sm font-medium text-red-800 dark:text-red-200">
                {error.message || "An unexpected error occurred"}
              </p>
            </div>

            <div className="flex gap-3 mb-6">
              <button
                onClick={resetError}
                className="flex items-center gap-2 px-4 py-2 bg-primary hover:opacity-90 text-primary-foreground rounded-lg transition-colors font-medium"
                aria-label="Try again"
              >
                <RefreshCw className="w-4 h-4" />
                Try Again
              </button>

              <button
                onClick={() => (window.location.href = "/")}
                className="px-4 py-2 bg-accent hover:bg-muted text-foreground rounded-lg transition-colors font-medium"
              >
                Go to Home
              </button>
            </div>

            <button
              onClick={() => setShowDetails(!showDetails)}
              className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
              aria-expanded={showDetails}
              aria-controls="error-details"
            >
              {showDetails ? (
                <>
                  <ChevronUp className="w-4 h-4" />
                  Hide technical details
                </>
              ) : (
                <>
                  <ChevronDown className="w-4 h-4" />
                  Show technical details
                </>
              )}
            </button>

            {showDetails && (
              <div
                id="error-details"
                className="mt-4 p-4 bg-accent rounded-lg overflow-auto max-h-96"
              >
                <div className="mb-4">
                  <h3 className="text-sm font-semibold text-foreground mb-2">
                    Error Details
                  </h3>
                  <pre className="text-xs text-muted-foreground whitespace-pre-wrap break-words">
                    {error.stack || error.message}
                  </pre>
                </div>

                {errorInfo?.componentStack && (
                  <div>
                    <h3 className="text-sm font-semibold text-foreground mb-2">
                      Component Stack
                    </h3>
                    <pre className="text-xs text-muted-foreground whitespace-pre-wrap break-words">
                      {errorInfo.componentStack}
                    </pre>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
