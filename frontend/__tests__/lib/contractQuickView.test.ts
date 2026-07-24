import {
  extractAbiMethodNames,
  getQuickViewVerificationStatus,
} from "../../lib/contractQuickView";

describe("contractQuickView helpers", () => {
  test("extractAbiMethodNames returns the first five ABI function names", () => {
    const abi = {
      functions: [
        { name: "init" },
        { name: "deposit" },
        { name: "withdraw" },
        { name: "claim" },
        { name: "transfer" },
        { name: "sweep" },
      ],
    };

    expect(extractAbiMethodNames(abi)).toEqual([
      "init",
      "deposit",
      "withdraw",
      "claim",
      "transfer",
    ]);
  });

  test("extractAbiMethodNames skips malformed functions and handles missing ABI", () => {
    expect(
      extractAbiMethodNames({
        functions: [
          { name: "balance" },
          { nope: true },
          null,
          { name: " allowance " },
        ],
      }),
    ).toEqual(["balance", "allowance"]);

    expect(extractAbiMethodNames(undefined)).toEqual([]);
    expect(extractAbiMethodNames({})).toEqual([]);
  });

  test("getQuickViewVerificationStatus maps summary contracts to badge states", () => {
    expect(getQuickViewVerificationStatus({ is_verified: true })).toBe(
      "approved",
    );
    expect(getQuickViewVerificationStatus({ is_verified: false })).toBe(
      "submitted",
    );
  });
});
