/**
 * Type utilities for common patterns
 */

/** Make all properties in T nullable */
export type Nullable<T> = { [P in keyof T]: T[P] | null };

/** Make all properties in T optional */
export type Optional<T> = { [P in keyof T]?: T[P] };

/** Standard API response wrapper */
export interface ApiResponse<T> {
  data: T | null;
  error: string | null;
  loading: boolean;
}

/** Helper for pick and omit combinations */
export type Subset<T, K extends keyof T> = Pick<T, K>;

/** Extract type of array elements */
export type ElementType<T extends ReadonlyArray<unknown>> =
  T extends ReadonlyArray<infer Element> ? Element : never;
