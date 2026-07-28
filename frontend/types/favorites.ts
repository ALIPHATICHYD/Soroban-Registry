/**
 * Favorites types for Soroban Registry
 */

export interface FavoriteContract {
  id: string;
  contract_id: string;
  name: string;
  network: string;
  category?: string;
  is_verified: boolean;
  added_at: string;
}

export interface FavoritesState {
  items: FavoriteContract[];
  isLoading: boolean;
  error: string | null;
}

export type FavoriteAction =
  | { type: 'ADD_FAVORITE'; payload: FavoriteContract }
  | { type: 'REMOVE_FAVORITE'; payload: string }
  | { type: 'SET_FAVORITES'; payload: FavoriteContract[] }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_ERROR'; payload: string | null };
