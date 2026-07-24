/**
 * Common shared types
 */

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface MaintenanceWindow {
  message: string;
  scheduled_end_at?: string;
}
