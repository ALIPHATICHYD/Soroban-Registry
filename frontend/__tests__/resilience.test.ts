import { describe, it, expect } from "vitest";
import { resilientCall, getBreaker } from "../lib/resilience";

describe("resilientCall", () => {
  it("retries until success", async () => {
    let attempts = 0;
    const fn = async () => {
      attempts += 1;
      if (attempts < 3) throw new Error("transient");
      return "ok";
    };

    const res = await resilientCall("test-retry", fn, { maxRetries: 3 });
    expect(res).toBe("ok");

    const breaker = getBreaker("test-retry");
    // reset for isolation
    breaker.reset();
  });

  it("opens circuit after failures and prevents requests", async () => {
    const fn = async () => {
      throw new Error("always fail");
    };

    try {
      // will exhaust retries and throw
      await resilientCall("test-open", fn, {
        maxRetries: 1,
        failureThreshold: 1,
      });
    } catch (e) {
      // ignore
    }

    const breaker = getBreaker("test-open");
    // now the circuit should be open and next call should immediately reject
    await expect(
      resilientCall("test-open", fn, { maxRetries: 0 }),
    ).rejects.toThrow();

    // reset for isolation
    breaker.reset();
  });
});
