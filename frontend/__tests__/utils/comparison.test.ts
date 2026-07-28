import {
  diffLines,
  extractAbiMethods,
  toneForMetricCell,
} from "@/utils/comparison";

describe("comparison utils", () => {
  describe("diffLines", () => {
    it("should detect additions", () => {
      const a = "line1\nline2";
      const b = "line1\nline2\nline3";
      const result = diffLines(a, b);

      expect(result).toEqual([
        { type: "context", value: "line1" },
        { type: "context", value: "line2" },
        { type: "add", value: "line3" },
      ]);
    });

    it("should detect removals", () => {
      const a = "line1\nline2\nline3";
      const b = "line1\nline3";
      const result = diffLines(a, b);

      expect(result).toEqual([
        { type: "context", value: "line1" },
        { type: "remove", value: "line2" },
        { type: "context", value: "line3" },
      ]);
    });

    it("should handle complex diffs", () => {
      const a = "A\nB\nC\nD";
      const b = "A\nX\nC\nY";
      const result = diffLines(a, b);

      expect(result).toEqual([
        { type: "context", value: "A" },
        { type: "remove", value: "B" },
        { type: "add", value: "X" },
        { type: "context", value: "C" },
        { type: "remove", value: "D" },
        { type: "add", value: "Y" },
      ]);
    });
  });

  describe("extractAbiMethods", () => {
    it("should extract names from a simple ABI array", () => {
      const abi = [{ name: "method1" }, { name: "method2" }];
      expect(extractAbiMethods(abi)).toEqual(["method1", "method2"]);
    });

    it("should extract names from nested structures", () => {
      const abi = {
        methods: [{ name: "m1" }, { export: "m2" }],
      };
      expect(extractAbiMethods(abi)).toEqual(["m1", "m2"]);
    });
  });

  describe("toneForMetricCell", () => {
    it("should return neutral if all values are equal", () => {
      expect(toneForMetricCell("deployment_count", 10, [10, 10, 10])).toBe(
        "neutral",
      );
    });

    it("should return best for the highest deployment count", () => {
      expect(toneForMetricCell("deployment_count", 50, [10, 50, 20])).toBe(
        "best",
      );
    });

    it("should return different for non-best values", () => {
      expect(toneForMetricCell("deployment_count", 10, [10, 50, 20])).toBe(
        "different",
      );
    });
  });
});
