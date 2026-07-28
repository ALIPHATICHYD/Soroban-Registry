import { NextFunction, Router, Request, Response } from "express";
import { pool } from "./db.js";
import { logger } from "./logger.js";
import { BadRequestError, DatabaseError, NotFoundError } from "./errors.js";
import type { Tag, TagWithAliases, HierarchicalTagGroup } from "./models.js";

const router = Router();

router.get("/tags", async (req: Request, res: Response, next: NextFunction) => {
  const prefix = (req.query.prefix as string) || "";
  const limit = Math.min(
    Math.max(parseInt(req.query.limit as string) || 20, 1),
    100,
  );

  try {
    const { rows } = await pool.query<Tag & { aliases: string[] }>(
      `SELECT
        t.*,
        COALESCE(
          array_agg(ta.alias) FILTER (WHERE ta.alias IS NOT NULL),
          '{}'
        ) AS aliases
      FROM tags t
      LEFT JOIN tag_aliases ta ON ta.canonical_tag_id = t.id
      WHERE t.prefix = $1
        OR t.prefix LIKE $1 || ':%'
        OR EXISTS (
          SELECT 1 FROM tag_aliases a
          WHERE a.canonical_tag_id = t.id
            AND (a.alias LIKE $1 || '%')
        )
      GROUP BY t.id
      ORDER BY t.is_trending DESC, t.usage_count DESC
      LIMIT $2`,
      [prefix, limit],
    );

    const grouped = new Map<string, TagWithAliases[]>();
    for (const row of rows) {
      const group = grouped.get(row.prefix) || [];
      group.push(row);
      grouped.set(row.prefix, group);
    }

    const result: HierarchicalTagGroup[] = Array.from(grouped.entries()).map(
      ([pfx, tags]) => ({
        prefix: pfx,
        tags,
        total: tags.length,
      }),
    );

    res.json({ groups: result, total: rows.length });
  } catch (err) {
    next(new DatabaseError("Failed to fetch tags", err instanceof Error ? err : undefined));
  }
});

router.post("/tags", async (req: Request, res: Response, next: NextFunction) => {
  const { prefix, name, description } = req.body;

  if (!prefix || !name) {
    next(new BadRequestError("prefix and name are required"));
    return;
  }

  try {
    const { rows } = await pool.query<Tag>(
      `INSERT INTO tags (prefix, name, description)
       VALUES ($1, $2, $3)
       ON CONFLICT (prefix, name) DO UPDATE SET
         description = COALESCE(EXCLUDED.description, tags.description),
         usage_count = tags.usage_count + 1
       RETURNING *`,
      [prefix, name, description || null],
    );

    logger.info({ tagId: rows[0].id, prefix, name }, "Tag created");
    res.status(201).json(rows[0]);
  } catch (err) {
    next(new DatabaseError("Failed to create tag", err instanceof Error ? err : undefined));
  }
});

router.post("/tags/alias", async (req: Request, res: Response, next: NextFunction) => {
  const { alias, canonical_prefix, canonical_name } = req.body;

  if (!alias || !canonical_prefix || !canonical_name) {
    next(new BadRequestError("alias, canonical_prefix, and canonical_name are required"));
    return;
  }

  try {
    const tagResult = await pool.query<Tag>(
      `SELECT id FROM tags WHERE prefix = $1 AND name = $2`,
      [canonical_prefix, canonical_name],
    );

    if (tagResult.rows.length === 0) {
      next(new NotFoundError("canonical tag not found"));
      return;
    }

    const { rows } = await pool.query(
      `INSERT INTO tag_aliases (alias, canonical_tag_id)
       VALUES ($1, $2)
       ON CONFLICT (alias) DO UPDATE SET canonical_tag_id = EXCLUDED.canonical_tag_id
       RETURNING *`,
      [alias, tagResult.rows[0].id],
    );

    logger.info({ alias, canonicalPrefix: canonical_prefix, canonicalName: canonical_name }, "Tag alias created");
    res.status(201).json(rows[0]);
  } catch (err) {
    next(new DatabaseError("Failed to create tag alias", err instanceof Error ? err : undefined));
  }
});

router.patch("/tags/:id/increment", async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { rows } = await pool.query<Tag>(
      `UPDATE tags SET usage_count = usage_count + 1 WHERE id = $1 RETURNING *`,
      [req.params.id],
    );

    if (rows.length === 0) {
      next(new NotFoundError("tag not found"));
      return;
    }

    res.json(rows[0]);
  } catch (err) {
    next(new DatabaseError("Failed to increment tag usage count", err instanceof Error ? err : undefined));
  }
});

export default router;
