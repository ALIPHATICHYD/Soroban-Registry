import assert from "node:assert/strict";
import test from "node:test";
import { createMockContractsDataAccess } from "./contractsDataAccess.mock.js";

test("mock DAL supports contract CRUD", async () => {
  const dal = createMockContractsDataAccess();

  const created = await dal.createContract({
    contract_id: "CABC123",
    wasm_hash: "hash-1",
    name: "Payments Router",
    publisher_id: "publisher-1",
    network: "testnet",
    tags: ["payments"],
  });

  assert.ok(created.id);
  assert.equal(created.name, "Payments Router");

  const fetched = await dal.getContractById(created.id);
  assert.ok(fetched);
  assert.equal(fetched?.contract_id, "CABC123");

  const updated = await dal.updateContract(created.id, {
    category: "defi",
    is_verified: true,
  });
  assert.equal(updated?.category, "defi");
  assert.equal(updated?.is_verified, true);

  const list = await dal.listContracts();
  assert.equal(list.length, 1);

  const deleted = await dal.deleteContract(created.id);
  assert.equal(deleted, true);
});
