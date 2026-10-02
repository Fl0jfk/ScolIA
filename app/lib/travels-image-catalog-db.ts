import "server-only";
import { and, eq, ne, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { travelImageCatalog } from "@/db/schema-travel-image-catalog";
import IMAGE_CATALOG from "@/app/api/travels/update/image-catalog.json";
import { normalizePublicImageUrl } from "@/app/lib/scola-image";
import {
  normalizeTravelImageKey,
  type TravelCatalogImage,
} from "@/app/lib/travels-image-catalog-helpers";

export type { TravelCatalogImage };
export {
  buildTravelPlaceSearchQuery,
  buildTravelWebSearchQueries,
  formatTravelImageAttribution,
  normalizeTravelImageKey,
  rankTravelCatalogCandidates,
  scoreTravelCatalogMatch,
  strongTravelThemeTokens,
  tokenizeTravelPlaceQuery,
} from "@/app/lib/travels-image-catalog-helpers";

type JsonCatalogEntry = {
  id: string;
  label: string;
  url: string;
  keywords?: string;
};

function withNormalizedUrl(img: TravelCatalogImage): TravelCatalogImage {
  return {
    ...img,
    url: normalizePublicImageUrl(img.url),
  };
}

function fromJsonEntry(entry: JsonCatalogEntry): TravelCatalogImage {
  return withNormalizedUrl({
    id: entry.id,
    label: entry.label,
    url: entry.url,
    keywords: entry.keywords || "",
    source: "manual",
    normalizeKey: normalizeTravelImageKey(entry.id || entry.label),
  });
}

function rowToImage(row: typeof travelImageCatalog.$inferSelect): TravelCatalogImage {
  return withNormalizedUrl({
    id: row.id,
    label: row.label,
    url: row.url,
    keywords: row.keywords || "",
    source: row.source,
    author: row.author,
    license: row.license,
    attributionUrl: row.attributionUrl,
    sourcePageUrl: row.sourcePageUrl,
    normalizeKey: row.normalizeKey,
  });
}

let seedPromise: Promise<void> | null = null;

/** Importe le JSON historique une fois (idempotent). */
export async function ensureTravelImageCatalogSeeded(): Promise<void> {
  if (!isDatabaseConfigured()) return;
  if (!seedPromise) {
    seedPromise = (async () => {
      const db = getDb();
      const existing = await db
        .select({ id: travelImageCatalog.id })
        .from(travelImageCatalog)
        .limit(1);
      if (existing.length > 0) return;

      const entries = IMAGE_CATALOG as JsonCatalogEntry[];
      const now = new Date();
      const usedKeys = new Set<string>();
      const values = entries.map((entry) => {
        let normalizeKey = normalizeTravelImageKey(entry.id || entry.label) || entry.id;
        if (usedKeys.has(normalizeKey)) {
          normalizeKey = `${normalizeKey}${normalizeTravelImageKey(entry.id).slice(-8) || "x"}`;
        }
        usedKeys.add(normalizeKey);
        return {
          id: entry.id,
          label: entry.label,
          url: normalizePublicImageUrl(entry.url),
          keywords: entry.keywords || "",
          normalizeKey,
          source: "manual" as const,
          author: null as string | null,
          license: null as string | null,
          attributionUrl: null as string | null,
          sourcePageUrl: null as string | null,
          createdAt: now,
          updatedAt: now,
        };
      });

      const chunkSize = 40;
      for (let i = 0; i < values.length; i += chunkSize) {
        const chunk = values.slice(i, i + chunkSize);
        await db
          .insert(travelImageCatalog)
          .values(chunk)
          .onConflictDoNothing({ target: travelImageCatalog.id });
      }
    })().catch((err) => {
      seedPromise = null;
      console.error("[travel-image-catalog] seed failed", err);
    });
  }
  await seedPromise;
}

/** Catalogue complet (BDD si dispo, sinon JSON embarqué). */
export async function listTravelCatalogImages(): Promise<TravelCatalogImage[]> {
  await ensureTravelImageCatalogSeeded();
  if (!isDatabaseConfigured()) {
    return (IMAGE_CATALOG as JsonCatalogEntry[]).map(fromJsonEntry);
  }
  try {
    const db = getDb();
    const rows = await db.select().from(travelImageCatalog);
    if (rows.length === 0) {
      return (IMAGE_CATALOG as JsonCatalogEntry[]).map(fromJsonEntry);
    }
    return rows.map(rowToImage);
  } catch (err) {
    console.error("[travel-image-catalog] list failed", err);
    return (IMAGE_CATALOG as JsonCatalogEntry[]).map(fromJsonEntry);
  }
}

export async function findTravelCatalogByNormalizeKey(
  normalizeKey: string,
): Promise<TravelCatalogImage | null> {
  const key = normalizeTravelImageKey(normalizeKey);
  if (!key) return null;
  await ensureTravelImageCatalogSeeded();
  if (!isDatabaseConfigured()) {
    const fromJson = (IMAGE_CATALOG as JsonCatalogEntry[])
      .map(fromJsonEntry)
      .find((img) => img.normalizeKey === key);
    return fromJson || null;
  }
  try {
    const db = getDb();
    const rows = await db
      .select()
      .from(travelImageCatalog)
      .where(eq(travelImageCatalog.normalizeKey, key))
      .limit(1);
    return rows[0] ? rowToImage(rows[0]) : null;
  } catch (err) {
    console.error("[travel-image-catalog] findByKey failed", err);
    return null;
  }
}

export async function findTravelCatalogById(
  id: string,
): Promise<TravelCatalogImage | null> {
  const wanted = String(id || "").trim();
  if (!wanted) return null;
  await ensureTravelImageCatalogSeeded();
  if (!isDatabaseConfigured()) {
    const hit = (IMAGE_CATALOG as JsonCatalogEntry[]).find((e) => e.id === wanted);
    return hit ? fromJsonEntry(hit) : null;
  }
  try {
    const db = getDb();
    const rows = await db
      .select()
      .from(travelImageCatalog)
      .where(eq(travelImageCatalog.id, wanted))
      .limit(1);
    return rows[0] ? rowToImage(rows[0]) : null;
  } catch {
    return null;
  }
}

export type UpsertTravelCatalogInput = {
  id: string;
  label: string;
  url: string;
  keywords?: string;
  normalizeKey?: string;
  source: NonNullable<TravelCatalogImage["source"]>;
  author?: string | null;
  license?: string | null;
  attributionUrl?: string | null;
  sourcePageUrl?: string | null;
};

/** Ajoute / met à jour une entrée (enrichissement web). Ne touche pas les manuelles existantes. */
export async function upsertTravelCatalogImage(
  input: UpsertTravelCatalogInput,
): Promise<TravelCatalogImage | null> {
  const normalizeKey = normalizeTravelImageKey(
    input.normalizeKey || input.label || input.id,
  );
  if (!normalizeKey || !input.url) return null;

  const payload: TravelCatalogImage = {
    id: input.id,
    label: input.label.trim() || input.id,
    url: normalizePublicImageUrl(input.url),
    keywords: (input.keywords || "").trim(),
    normalizeKey,
    source: input.source,
    author: input.author?.trim() || null,
    license: input.license?.trim() || null,
    attributionUrl: input.attributionUrl?.trim() || null,
    sourcePageUrl: input.sourcePageUrl?.trim() || null,
  };

  if (!isDatabaseConfigured()) {
    return withNormalizedUrl(payload);
  }

  try {
    await ensureTravelImageCatalogSeeded();
    const db = getDb();
    const updatedAt = new Date();

    const existing = await db
      .select()
      .from(travelImageCatalog)
      .where(eq(travelImageCatalog.normalizeKey, normalizeKey))
      .limit(1);
    if (existing[0]?.source === "manual") {
      return rowToImage(existing[0]);
    }

    if (existing[0]) {
      await db
        .update(travelImageCatalog)
        .set({
          label: payload.label,
          url: payload.url!,
          keywords: payload.keywords || "",
          source: payload.source!,
          author: payload.author ?? null,
          license: payload.license ?? null,
          attributionUrl: payload.attributionUrl ?? null,
          sourcePageUrl: payload.sourcePageUrl ?? null,
          updatedAt,
        })
        .where(
          and(
            eq(travelImageCatalog.normalizeKey, normalizeKey),
            ne(travelImageCatalog.source, "manual"),
          ),
        );
      const refreshed = await db
        .select()
        .from(travelImageCatalog)
        .where(eq(travelImageCatalog.normalizeKey, normalizeKey))
        .limit(1);
      return refreshed[0] ? rowToImage(refreshed[0]) : null;
    }

    await db
      .insert(travelImageCatalog)
      .values({
        id: payload.id,
        label: payload.label,
        url: payload.url!,
        keywords: payload.keywords || "",
        normalizeKey,
        source: payload.source!,
        author: payload.author ?? null,
        license: payload.license ?? null,
        attributionUrl: payload.attributionUrl ?? null,
        sourcePageUrl: payload.sourcePageUrl ?? null,
        createdAt: new Date(),
        updatedAt,
      })
      .onConflictDoNothing({ target: travelImageCatalog.id });

    const afterInsert = await db
      .select()
      .from(travelImageCatalog)
      .where(eq(travelImageCatalog.normalizeKey, normalizeKey))
      .limit(1);
    if (afterInsert[0]) return rowToImage(afterInsert[0]);

    const altId = `${payload.id}-${Date.now().toString(36)}`;
    await db.insert(travelImageCatalog).values({
      id: altId,
      label: payload.label,
      url: payload.url!,
      keywords: payload.keywords || "",
      normalizeKey,
      source: payload.source!,
      author: payload.author ?? null,
      license: payload.license ?? null,
      attributionUrl: payload.attributionUrl ?? null,
      sourcePageUrl: payload.sourcePageUrl ?? null,
      createdAt: new Date(),
      updatedAt,
    });
    return withNormalizedUrl({ ...payload, id: altId });
  } catch (err) {
    console.error("[travel-image-catalog] upsert failed", err);
    return withNormalizedUrl(payload);
  }
}

/** Recherche floue SQL complémentaire (si tokens présents). */
export async function searchTravelCatalogByTokens(
  tokens: string[],
  excludeId?: string | null,
  limit = 12,
): Promise<TravelCatalogImage[]> {
  if (tokens.length === 0 || !isDatabaseConfigured()) return [];
  await ensureTravelImageCatalogSeeded();
  try {
    const db = getDb();
    const likeParts = tokens.slice(0, 6).map((t) => `%${t}%`);
    const rows = await db
      .select()
      .from(travelImageCatalog)
      .where(
        sql`(${sql.join(
          likeParts.map(
            (p) =>
              sql`(lower(${travelImageCatalog.label}) LIKE lower(${p})
                OR lower(coalesce(${travelImageCatalog.keywords}, '')) LIKE lower(${p})
                OR lower(${travelImageCatalog.id}) LIKE lower(${p}))`,
          ),
          sql` OR `,
        )})`,
      )
      .limit(limit * 2);

    const excluded = String(excludeId || "")
      .toLowerCase()
      .replace(/[^a-z0-9]/g, "");
    return rows
      .map(rowToImage)
      .filter((img) => {
        if (!excluded) return true;
        const idNorm = String(img.id || "")
          .toLowerCase()
          .replace(/[^a-z0-9]/g, "");
        return idNorm !== excluded;
      })
      .slice(0, limit);
  } catch (err) {
    console.error("[travel-image-catalog] token search failed", err);
    return [];
  }
}
