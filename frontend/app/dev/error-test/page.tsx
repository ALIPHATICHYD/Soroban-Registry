"use client";

import { useState } from "react";

export default function ErrorTestPage() {
  const [doThrow, setDoThrow] = useState(false);

  if (doThrow) {
    // Throw during render so ErrorBoundary catches it
    throw new Error("Dev test error: intentional render exception");
  }

  return (
    <div className="p-8">
      <h1 className="text-2xl font-bold mb-4">Error Boundary Test</h1>
      <p className="mb-4">
        Click the button to trigger a client-side error and verify the global
        error boundary.
      </p>
      <button
        onClick={() => setDoThrow(true)}
        className="px-4 py-2 bg-red-500 text-white rounded-lg"
      >
        Throw Error
      </button>
    </div>
  );
}
