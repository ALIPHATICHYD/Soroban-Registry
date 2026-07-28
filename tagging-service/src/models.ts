export interface Tag {
  id: string;
  prefix: string;
  name: string;
  description: string | null;
  usage_count: number;
  is_trending: boolean;
  created_at: Date;
  updated_at: Date;
}

export interface TagAlias {
  id: string;
  alias: string;
  canonical_tag_id: string;
  created_at: Date;
}

export interface TagUsageLog {
  id: string;
  tag_id: string;
  usage_count: number;
  recorded_at: Date;
}

export interface TagWithAliases extends Tag {
  aliases: string[];
}

export interface HierarchicalTagGroup {
  prefix: string;
  tags: TagWithAliases[];
  total: number;
}
