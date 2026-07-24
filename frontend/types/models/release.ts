/**
 * Release and changelog types
 */

export interface FunctionChange {
  name: string;
  change_type: "added" | "removed" | "modified";
  old_signature?: string;
  new_signature?: string;
  is_breaking: boolean;
}

export interface DiffSummary {
  files_changed: number;
  lines_added: number;
  lines_removed: number;
  function_changes: FunctionChange[];
  has_breaking_changes: boolean;
  features_count: number;
  fixes_count: number;
  breaking_count: number;
}

export type ReleaseNotesStatus = "draft" | "published";

export interface ReleaseNotesResponse {
  id: string;
  contract_id: string;
  version: string;
  previous_version?: string;
  diff_summary: DiffSummary;
  changelog_entry?: string;
  notes_text: string;
  status: ReleaseNotesStatus;
  generated_by: string;
  created_at: string;
  updated_at: string;
  published_at?: string;
}

export interface GenerateReleaseNotesRequest {
  version: string;
  previous_version?: string;
  source_url?: string;
  changelog_content?: string;
  contract_address?: string;
}

export interface UpdateReleaseNotesRequest {
  notes_text: string;
}

export interface PublishReleaseNotesRequest {
  update_version_record?: boolean;
}
