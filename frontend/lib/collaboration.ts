export interface CollaboratorCursor {
  userId: string;
  name: string;
  color: string;
  line: number;
  column: number;
}

export interface CollaborativeComment {
  id: string;
  userId: string;
  line: number;
  body: string;
  createdAt: string;
  resolved: boolean;
}

export interface CollaborativeEdit {
  userId: string;
  baseRevision: number;
  from: number;
  to: number;
  text: string;
}

export interface CollaborativeDocument {
  source: string;
  revision: number;
  cursors: CollaboratorCursor[];
  comments: CollaborativeComment[];
  history: Array<{
    revision: number;
    userId: string;
    summary: string;
    createdAt: string;
  }>;
}

export interface EditResult {
  document: CollaborativeDocument;
  conflictResolved: boolean;
  resolution: string;
}

export function createCollaborativeDocument(
  source: string,
): CollaborativeDocument {
  return {
    source,
    revision: 1,
    cursors: [
      { userId: "u1", name: "Maya", color: "#06b6d4", line: 8, column: 12 },
      { userId: "u2", name: "Ken", color: "#22c55e", line: 18, column: 5 },
      { userId: "u3", name: "Rin", color: "#f59e0b", line: 27, column: 18 },
    ],
    comments: [
      {
        id: "c-auth",
        userId: "u1",
        line: 21,
        body: "Check whether this state change has the correct signer.",
        createdAt: "2026-04-23T00:00:00.000Z",
        resolved: false,
      },
    ],
    history: [
      {
        revision: 1,
        userId: "system",
        summary: "Workspace opened",
        createdAt: "2026-04-23T00:00:00.000Z",
      },
    ],
  };
}

export function applyCollaborativeEdit(
  document: CollaborativeDocument,
  edit: CollaborativeEdit,
  createdAt = new Date().toISOString(),
): EditResult {
  const safeFrom = Math.max(0, Math.min(edit.from, document.source.length));
  const safeTo = Math.max(safeFrom, Math.min(edit.to, document.source.length));
  const conflictResolved = edit.baseRevision !== document.revision;
  const source = `${document.source.slice(0, safeFrom)}${edit.text}${document.source.slice(safeTo)}`;
  const revision = document.revision + 1;
  const delta = edit.text.length - (safeTo - safeFrom);

  const documentNext: CollaborativeDocument = {
    ...document,
    source,
    revision,
    cursors: document.cursors.map((cursor) =>
      cursor.userId === edit.userId
        ? { ...cursor, column: cursor.column + edit.text.length }
        : cursor,
    ),
    comments: document.comments.map((comment) =>
      comment.line * 80 > safeFrom
        ? { ...comment, line: Math.max(1, comment.line + Math.sign(delta)) }
        : comment,
    ),
    history: [
      ...document.history,
      {
        revision,
        userId: edit.userId,
        summary: conflictResolved
          ? "Merged stale edit with latest revision using last-writer text range transform"
          : "Applied live edit",
        createdAt,
      },
    ],
  };

  return {
    document: documentNext,
    conflictResolved,
    resolution: conflictResolved
      ? `Edit based on r${edit.baseRevision} was transformed onto r${document.revision}.`
      : "Edit applied without conflict.",
  };
}

export function addCollaborativeComment(
  document: CollaborativeDocument,
  userId: string,
  line: number,
  body: string,
  createdAt = new Date().toISOString(),
): CollaborativeDocument {
  return {
    ...document,
    comments: [
      ...document.comments,
      {
        id: `c-${document.revision}-${document.comments.length + 1}`,
        userId,
        line: Math.max(1, line),
        body,
        createdAt,
        resolved: false,
      },
    ],
  };
}
