"use client";

import { useEffect, useState, type ReactElement } from "react";
import { getAllBreakerStates } from "../lib/resilience";

export default function BreakerDebug(): ReactElement {
  const [states, setStates] = useState<Record<string, unknown>>({});

  useEffect(() => {
    queueMicrotask(() => setStates(getAllBreakerStates()));
    const id = setInterval(() => setStates(getAllBreakerStates()), 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <div style={{ padding: 16 }}>
      <h2>Breaker States (client)</h2>
      <pre style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
        {JSON.stringify(states, null, 2)}
      </pre>
    </div>
  );
}
