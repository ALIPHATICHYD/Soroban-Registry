import { createSlice, PayloadAction } from "@reduxjs/toolkit";

export type Theme = "light" | "dark" | "system";

interface ThemeState {
  value: Theme;
}

const getInitial = (): Theme => {
  try {
    if (typeof window !== "undefined") {
      const v = window.localStorage.getItem(
        "soroban-registry-theme",
      ) as Theme | null;
      if (v) return v;
    }
  } catch (e) {
    // ignore
  }
  return "system";
};

const initialState: ThemeState = { value: getInitial() };

const slice = createSlice({
  name: "theme",
  initialState,
  reducers: {
    setTheme(state, action: PayloadAction<Theme>) {
      state.value = action.payload;
      try {
        if (typeof window !== "undefined") {
          window.localStorage.setItem("soroban-registry-theme", action.payload);
        }
      } catch (e) {}
    },
  },
});

export const { setTheme } = slice.actions;
export default slice.reducer;
