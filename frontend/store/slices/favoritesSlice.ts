import { createSlice, PayloadAction } from "@reduxjs/toolkit";

interface FavoritesState {
  items: string[];
}

const initialState: FavoritesState = { items: [] };

const slice = createSlice({
  name: "favorites",
  initialState,
  reducers: {
    addFavorite(state, action: PayloadAction<string>) {
      if (!state.items.includes(action.payload))
        state.items.push(action.payload);
    },
    removeFavorite(state, action: PayloadAction<string>) {
      state.items = state.items.filter((i) => i !== action.payload);
    },
    setFavorites(state, action: PayloadAction<string[]>) {
      state.items = action.payload;
    },
  },
});

export const { addFavorite, removeFavorite, setFavorites } = slice.actions;
export default slice.reducer;
