import {
  pinContractMetadata,
  retrievePinnedContract,
  verifyPinnedContract,
} from "../../lib/ipfsMirror";

describe("IPFS mirror helpers", () => {
  test("pins metadata with deterministic content addressing and verifies it", () => {
    const metadata = {
      contractId: "CA123",
      name: "Token",
      network: "testnet",
      wasmHash: "sha256-abc",
      version: "1.0.0",
      auditScore: 91,
    };

    const first = pinContractMetadata(metadata, "2026-04-23T00:00:00.000Z");
    const second = pinContractMetadata(metadata, "2026-04-23T00:01:00.000Z");

    expect(first.cid).toBe(second.cid);
    expect(first.cid).toStartWith("bafy");
    expect(verifyPinnedContract(first)).toBe(true);
    expect(retrievePinnedContract(first.cid, [first])).toEqual(first);
  });

  test("detects tampered metadata", () => {
    const pin = pinContractMetadata({
      contractId: "CA123",
      name: "Token",
      network: "testnet",
      wasmHash: "sha256-abc",
      version: "1.0.0",
    });

    const tampered = {
      ...pin,
      metadata: { ...pin.metadata, wasmHash: "sha256-def" },
    };

    expect(verifyPinnedContract(tampered)).toBe(false);
  });
});
