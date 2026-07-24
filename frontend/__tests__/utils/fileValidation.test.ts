import { validateFilesToAdd, formatBytes } from "@/utils/fileValidation";

describe("fileValidation", () => {
  describe("formatBytes", () => {
    it("should format bytes correctly", () => {
      expect(formatBytes(0)).toBe("0 B");
      expect(formatBytes(1024)).toBe("1.0 KB");
      expect(formatBytes(1048576)).toBe("1.0 MB");
      expect(formatBytes(1073741824)).toBe("1.0 GB");
    });
  });

  describe("validateFilesToAdd", () => {
    const mockFile = (name: string, size: number, type: string) => {
      const blob = new Blob([new ArrayBuffer(size)], { type });
      return new File([blob], name, { type, lastModified: 12345 });
    };

    it("should accept valid files", () => {
      const file = mockFile("test.pdf", 100, "application/pdf");
      const { accepted, errors } = validateFilesToAdd({
        existingFiles: [],
        newFiles: [file],
      });
      expect(accepted).toHaveLength(1);
      expect(errors).toHaveLength(0);
    });

    it("should reject invalid types", () => {
      const file = mockFile("test.exe", 100, "application/x-msdownload");
      const { accepted, errors } = validateFilesToAdd({
        existingFiles: [],
        newFiles: [file],
      });
      expect(accepted).toHaveLength(0);
      expect(errors[0].code).toBe("type_not_allowed");
    });

    it("should reject duplicates", () => {
      const file = mockFile("test.pdf", 100, "application/pdf");
      const { accepted, errors } = validateFilesToAdd({
        existingFiles: [file],
        newFiles: [file],
      });
      expect(accepted).toHaveLength(0);
      expect(errors[0].code).toBe("duplicate");
    });

    it("should reject when total size exceeded", () => {
      const file = mockFile("test.pdf", 200, "application/pdf");
      const { accepted, errors } = validateFilesToAdd({
        existingFiles: [],
        newFiles: [file],
        maxTotalBytes: 100,
      });
      expect(accepted).toHaveLength(0);
      expect(errors[0].code).toBe("total_size_exceeded");
    });
  });
});
