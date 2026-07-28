"use client";

import BreakerDebug from '../../../components/BreakerDebug';

export default function Page() {
  return (
    <div style={{ padding: 24 }}>
      <h1>Dev: Circuit Breaker Debug</h1>
      <BreakerDebug />
    </div>
  );
}
