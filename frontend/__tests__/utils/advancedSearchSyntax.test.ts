import { parseAdvancedContractQuery } from "@/utils/advancedSearchSyntax";

describe("parseAdvancedContractQuery", () => {
  test("extracts tag: syntax", () => {
    const parsed = parseAdvancedContractQuery("tag:DeFi token");
    expect(parsed.tags).toEqual(["DeFi"]);
    expect(parsed.cleanedSimpleQuery).toBe("token");
  });

  test("extracts #tag shorthand", () => {
    const parsed = parseAdvancedContractQuery("#yield optimizer");
    expect(parsed.tags).toEqual(["yield"]);
    expect(parsed.cleanedSimpleQuery).toBe("optimizer");
  });

  test("detects OR usage", () => {
    const parsed = parseAdvancedContractQuery("token OR bridge");
    expect(parsed.usesOr).toBe(true);
    expect(parsed.queryNode).toBeTruthy();
  });

  test("treats implicit whitespace as AND", () => {
    const parsed = parseAdvancedContractQuery("token bridge");
    expect(parsed.usesOr).toBe(false);
    expect(parsed.queryNode).toEqual({
      operator: "AND",
      conditions: [expect.any(Object), expect.any(Object)],
    });
  });

  test("supports parentheses grouping", () => {
    const parsed = parseAdvancedContractQuery("(token OR bridge) AND tag:DeFi");
    expect(parsed.usesOr).toBe(true);
    expect(parsed.tags).toEqual(["DeFi"]);
    expect(parsed.queryNode).toBeTruthy();
  });
});
