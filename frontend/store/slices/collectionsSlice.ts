import { createSlice, PayloadAction } from "@reduxjs/toolkit";

export interface Collection {
  id: string;
  name: string;
  items: string[];
  createdAt: string;
}

interface CollectionsState {
  collections: Collection[];
}

const initialState: CollectionsState = { collections: [] };

const slice = createSlice({
  name: "collections",
  initialState,
  reducers: {
    createCollection(state, action: PayloadAction<string>) {
      state.collections.push({
        id: Math.random().toString(36).slice(2) + Date.now().toString(36),
        name: action.payload.trim(),
        items: [],
        createdAt: new Date().toISOString(),
      });
    },
    deleteCollection(state, action: PayloadAction<string>) {
      state.collections = state.collections.filter(
        (c) => c.id !== action.payload,
      );
    },
    renameCollection(
      state,
      action: PayloadAction<{ id: string; name: string }>,
    ) {
      const col = state.collections.find((c) => c.id === action.payload.id);
      if (col) col.name = action.payload.name.trim();
    },
    addToCollection(
      state,
      action: PayloadAction<{ collectionId: string; contractId: string }>,
    ) {
      const col = state.collections.find(
        (c) => c.id === action.payload.collectionId,
      );
      if (col && !col.items.includes(action.payload.contractId)) {
        col.items.push(action.payload.contractId);
      }
    },
    removeFromCollection(
      state,
      action: PayloadAction<{ collectionId: string; contractId: string }>,
    ) {
      const col = state.collections.find(
        (c) => c.id === action.payload.collectionId,
      );
      if (col) {
        col.items = col.items.filter((id) => id !== action.payload.contractId);
      }
    },
  },
});

export const {
  createCollection,
  deleteCollection,
  renameCollection,
  addToCollection,
  removeFromCollection,
} = slice.actions;
export default slice.reducer;
