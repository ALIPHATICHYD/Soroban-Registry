import {
  addCollaborativeComment,
  applyCollaborativeEdit,
  createCollaborativeDocument,
} from "../../lib/collaboration";

describe("collaboration helpers", () => {
  test("applies real-time edits and records version history", () => {
    const doc = createCollaborativeDocument("pub fn a() {}");
    const result = applyCollaborativeEdit(
      doc,
      {
        userId: "u1",
        baseRevision: 1,
        from: 0,
        to: 0,
        text: "// live\n",
      },
      "2026-04-23T00:00:00.000Z",
    );

    expect(result.conflictResolved).toBe(false);
    expect(result.document.revision).toBe(2);
    expect(result.document.source.startsWith("// live")).toBe(true);
  });

  test("marks stale edits as automatically resolved conflicts", () => {
    const doc = createCollaborativeDocument("pub fn a() {}");
    const result = applyCollaborativeEdit(
      doc,
      {
        userId: "u2",
        baseRevision: 0,
        from: 4,
        to: 6,
        text: "function",
      },
      "2026-04-23T00:00:00.000Z",
    );

    expect(result.conflictResolved).toBe(true);
    expect(result.resolution).toContain("transformed");
  });

  test("adds live review comments", () => {
    const doc = addCollaborativeComment(
      createCollaborativeDocument("source"),
      "u3",
      4,
      "Looks good",
    );

    expect(doc.comments.at(-1)?.body).toBe("Looks good");
  });
});
