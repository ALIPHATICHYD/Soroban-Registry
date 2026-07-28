/**
 * Review and collaboration types
 */

export interface CollaborativeReview {
  id: string;
  contract_id: string;
  version: string;
  status: "pending" | "approved" | "changes_requested";
  created_at: string;
  updated_at: string;
}

export interface CollaborativeReviewer {
  id: string;
  review_id: string;
  user_id: string;
  status: "pending" | "approved" | "changes_requested";
  created_at: string;
  updated_at: string;
}

export interface CollaborativeComment {
  id: string;
  review_id: string;
  user_id: string;
  content: string;
  line_number?: number;
  file_path?: string;
  abi_path?: string;
  parent_id?: string;
  created_at: string;
  updated_at: string;
}

export interface CollaborativeReviewDetails {
  review: CollaborativeReview;
  reviewers: CollaborativeReviewer[];
  comments: CollaborativeComment[];
}
