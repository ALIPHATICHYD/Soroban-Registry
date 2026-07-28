import { api } from "@/lib/api";
import fetchMock from "jest-fetch-mock";

describe("api", () => {
  beforeEach(() => {
    fetchMock.resetMocks();
  });

  describe("getContracts", () => {
    it("should fetch contracts with default parameters", async () => {
      const mockData = {
        items: [{ id: "1", name: "Test Contract" }],
        total: 1,
        page: 1,
        per_page: 10,
        total_pages: 1,
      };
      fetchMock.mockResponseOnce(JSON.stringify(mockData));

      const result = await api.getContracts();

      expect(result).toEqual(mockData);
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/contracts?page=1&per_page=10"),
        expect.any(Object),
      );
    });

    it("should handle API errors gracefully", async () => {
      fetchMock.mockResponseOnce(JSON.stringify({ error: "Not Found" }), {
        status: 404,
      });

      await expect(api.getContracts()).rejects.toThrow();
    });

    it("should handle network failures", async () => {
      fetchMock.mockRejectOnce(new Error("Network failure"));

      await expect(api.getContracts()).rejects.toThrow("Network failure");
    });
  });

  describe("getContract", () => {
    it("should fetch a single contract by id", async () => {
      const mockContract = { id: "test-id", name: "Test Contract" };
      fetchMock.mockResponseOnce(JSON.stringify(mockContract));

      const result = await api.getContract("test-id");

      expect(result).toEqual(mockContract);
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/contracts/test-id"),
        expect.any(Object),
      );
    });
  });
});
