import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  stageApplication,
  stageApplicationAttr,
  stageConvention,
  stageConventionAttr,
  stageOffer,
  stageOfferAttr,
  stageToken,
  stageTokenAttr,
} from "@/db/schema";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import type {
  StageConvention,
  StageConventionIndexEntry,
  StageOffer,
  StageOfferApplication,
  StageOfferIndexEntry,
  StageOfferCandidatureTokenRef,
  StageSignCodeLookupRef,
  StageSignTokenRef,
  StageStudentTokenRef,
} from "@/app/lib/stage-types";

export type StageTokenKind =
  | "sign"
  | "sign_code"
  | "student"
  | "offer_candidature"
  | "periods"
  | "referents"
  | "constraints"
  | "watchers"
  | "auto_purge"
  | "identity_otp"
  | "identity_recipient_choice"
  | "identity_proof";

const SKIP_CONVENTION_ROOT = new Set(["id", "status", "updatedAt"]);
const SKIP_OFFER_ROOT = new Set(["id", "status", "updatedAt", "companyName"]);
const SKIP_APPLICATION_ROOT = new Set(["id", "offerId", "status", "updatedAt"]);

function parseTs(raw: string | undefined | null): Date {
  if (!raw) return new Date();
  const d = new Date(raw);
  return Number.isNaN(d.getTime()) ? new Date() : d;
}

export async function stagesDbReady(): Promise<string | null> {
  if (!isEntCoreDbEnabled()) return null;
  return resolveCurrentEtablissementId();
}

function asConvention(row: Record<string, unknown>, fallbackId: string): StageConvention | null {
  const id = String(row.id ?? fallbackId ?? "").trim();
  if (!id) return null;
  if (!row.student || typeof row.student !== "object") return null;
  if (!row.company || typeof row.company !== "object") return null;
  if (!row.schedule || typeof row.schedule !== "object") return null;
  return { ...row, id } as StageConvention;
}

function asOffer(row: Record<string, unknown>, fallbackId: string): StageOffer | null {
  const id = String(row.id ?? fallbackId ?? "").trim();
  if (!id) return null;
  if (!row.companyName && !row.kind) return null;
  return { ...row, id } as StageOffer;
}

function conventionToIndexEntry(c: StageConvention): StageConventionIndexEntry {
  return {
    id: c.id,
    status: c.status,
    studentName: `${c.student.firstName} ${c.student.lastName}`.trim(),
    className: c.student.className,
    level: c.student.level,
    companyName: c.company.name,
    internshipKind: c.internshipKind,
    periodStart: c.schedule.periodStart,
    periodEnd: c.schedule.periodEnd,
    schoolYear: c.schoolYear,
    updatedAt: c.updatedAt,
    stageLabel: c.stageLabel?.trim() || undefined,
    teacherReferentEmail: c.teacherReferent.email?.toLowerCase() || undefined,
  };
}

function offerToIndexEntry(o: StageOffer): StageOfferIndexEntry {
  return {
    id: o.id,
    kind: o.kind,
    status: o.status,
    companyName: o.companyName,
    targetLevels: o.targetLevels,
    schoolYear: o.schoolYear,
    createdAt: o.createdAt,
  };
}

/* -------------------------------------------------------------------------- */
/* Conventions                                                                */
/* -------------------------------------------------------------------------- */

async function hydrateConvention(
  etablissementId: string,
  m: typeof stageConvention.$inferSelect,
): Promise<StageConvention> {
  const db = getDb();
  const attrs = await db
    .select()
    .from(stageConventionAttr)
    .where(
      and(
        eq(stageConventionAttr.etablissementId, etablissementId),
        eq(stageConventionAttr.conventionId, m.id),
      ),
    );
  const inflated = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
  const convention = asConvention(
    {
      ...inflated,
      id: m.id,
      status: m.status ?? inflated.status,
      updatedAt: m.updatedAt?.toISOString?.() ?? inflated.updatedAt,
    },
    m.id,
  );
  if (!convention) {
    throw new Error(`[stage-db] Convention ${m.id} illisible (attrs incomplets).`);
  }
  return convention;
}

export async function listConventionsFromDb(etablissementId: string): Promise<StageConvention[]> {
  const db = getDb();
  const mains = await db
    .select()
    .from(stageConvention)
    .where(eq(stageConvention.etablissementId, etablissementId));
  const out: StageConvention[] = [];
  for (const m of mains) {
    try {
      out.push(await hydrateConvention(etablissementId, m));
    } catch (e) {
      console.error("[stage-db] hydrate convention", m.id, e);
    }
  }
  return out.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
}

export async function getConventionFromDb(
  etablissementId: string,
  id: string,
): Promise<StageConvention | null> {
  const db = getDb();
  const [m] = await db
    .select()
    .from(stageConvention)
    .where(and(eq(stageConvention.etablissementId, etablissementId), eq(stageConvention.id, id)))
    .limit(1);
  if (!m) return null;
  return hydrateConvention(etablissementId, m);
}

export async function upsertConventionInDb(
  etablissementId: string,
  convention: StageConvention,
): Promise<void> {
  const db = getDb();
  const id = String(convention.id).trim();
  if (!id) throw new Error("[stage-db] convention.id requis");
  const status = String(convention.status ?? "");
  const updatedAt = parseTs(convention.updatedAt);

  await db
    .insert(stageConvention)
    .values({
      id,
      etablissementId,
      status,
      updatedAt,
    })
    .onConflictDoUpdate({
      target: stageConvention.id,
      set: {
        etablissementId,
        status,
        updatedAt,
      },
    });

  const rest: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(convention as unknown as Record<string, unknown>)) {
    if (SKIP_CONVENTION_ROOT.has(k)) continue;
    rest[k] = v;
  }

  await db
    .delete(stageConventionAttr)
    .where(
      and(
        eq(stageConventionAttr.etablissementId, etablissementId),
        eq(stageConventionAttr.conventionId, id),
      ),
    );
  const attrs = flattenToAttrs(rest);
  if (attrs.length > 0) {
    const chunk = 80;
    for (let i = 0; i < attrs.length; i += chunk) {
      await db.insert(stageConventionAttr).values(
        attrs.slice(i, i + chunk).map((a) => ({
          etablissementId,
          conventionId: id,
          path: a.path,
          value: a.value,
        })),
      );
    }
  }
}

export async function listConventionIndexFromDb(
  etablissementId: string,
): Promise<StageConventionIndexEntry[]> {
  const list = await ensureConventionsMigratedFromCollection(etablissementId);
  return list.map(conventionToIndexEntry);
}

/**
 * Copie les conventions encore en `ent_collection` vers `stage_convention`
 * (upsert unitaire, jamais de wipe). Les ids déjà typés ne sont pas écrasés.
 */
export async function ensureConventionsMigratedFromCollection(
  etablissementId: string,
): Promise<StageConvention[]> {
  const existing = await listConventionsFromDb(etablissementId);
  const existingIds = new Set(existing.map((c) => c.id));

  const { listCollectionRecords, getCollectionRecord } = await import(
    "@/app/lib/ent-collection-db"
  );
  const legacyRows = await listCollectionRecords<Record<string, unknown>>(
    etablissementId,
    "stages__conventions",
  );

  for (const row of legacyRows) {
    const id = String(row.id ?? "").trim();
    if (!id || existingIds.has(id)) continue;
    const convention = asConvention(row, id);
    if (!convention) continue;
    await upsertConventionInDb(etablissementId, convention);
    existingIds.add(id);
  }

  const indexRec = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages",
    "conventions-index",
  );
  if (indexRec) {
    const raw =
      "__root" in indexRec && Array.isArray(indexRec.__root)
        ? indexRec.__root
        : Array.isArray(indexRec)
          ? indexRec
          : null;
    if (Array.isArray(raw)) {
      for (const entry of raw) {
        const id =
          entry && typeof entry === "object"
            ? String((entry as { id?: string }).id ?? "").trim()
            : "";
        if (!id || existingIds.has(id)) continue;
        const one = await getCollectionRecord<Record<string, unknown>>(
          etablissementId,
          "stages__conventions",
          id,
        );
        if (!one) continue;
        const convention = asConvention(one, id);
        if (!convention) continue;
        await upsertConventionInDb(etablissementId, convention);
        existingIds.add(id);
      }
    }
  }

  return listConventionsFromDb(etablissementId);
}

export async function getConventionFromDbOrMigrate(
  etablissementId: string,
  id: string,
): Promise<StageConvention | null> {
  const hit = await getConventionFromDb(etablissementId, id);
  if (hit) return hit;
  const { getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  const legacy = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages__conventions",
    id,
  );
  if (!legacy) return null;
  const convention = asConvention(legacy, id);
  if (!convention) return null;
  await upsertConventionInDb(etablissementId, convention);
  return getConventionFromDb(etablissementId, id);
}

/* -------------------------------------------------------------------------- */
/* Offers                                                                     */
/* -------------------------------------------------------------------------- */

async function hydrateOffer(
  etablissementId: string,
  m: typeof stageOffer.$inferSelect,
): Promise<StageOffer> {
  const db = getDb();
  const attrs = await db
    .select()
    .from(stageOfferAttr)
    .where(
      and(eq(stageOfferAttr.etablissementId, etablissementId), eq(stageOfferAttr.offerId, m.id)),
    );
  const inflated = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
  const offer = asOffer(
    {
      ...inflated,
      id: m.id,
      status: m.status ?? inflated.status,
      companyName: inflated.companyName ?? m.title ?? "",
      updatedAt: m.updatedAt?.toISOString?.() ?? inflated.updatedAt,
    },
    m.id,
  );
  if (!offer) throw new Error(`[stage-db] Offre ${m.id} illisible.`);
  return offer;
}

export async function listOffersFromDb(etablissementId: string): Promise<StageOffer[]> {
  const db = getDb();
  const mains = await db
    .select()
    .from(stageOffer)
    .where(eq(stageOffer.etablissementId, etablissementId));
  const out: StageOffer[] = [];
  for (const m of mains) {
    try {
      out.push(await hydrateOffer(etablissementId, m));
    } catch (e) {
      console.error("[stage-db] hydrate offer", m.id, e);
    }
  }
  return out;
}

export async function getOfferFromDb(
  etablissementId: string,
  id: string,
): Promise<StageOffer | null> {
  const db = getDb();
  const [m] = await db
    .select()
    .from(stageOffer)
    .where(and(eq(stageOffer.etablissementId, etablissementId), eq(stageOffer.id, id)))
    .limit(1);
  if (!m) return null;
  return hydrateOffer(etablissementId, m);
}

export async function upsertOfferInDb(etablissementId: string, offer: StageOffer): Promise<void> {
  const db = getDb();
  const id = String(offer.id).trim();
  if (!id) throw new Error("[stage-db] offer.id requis");
  const status = String(offer.status ?? "");
  const title = String(offer.companyName ?? "").trim();
  const updatedAt = parseTs(offer.updatedAt ?? offer.createdAt);

  await db
    .insert(stageOffer)
    .values({ id, etablissementId, title, status, updatedAt })
    .onConflictDoUpdate({
      target: stageOffer.id,
      set: { etablissementId, title, status, updatedAt },
    });

  const rest: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(offer as unknown as Record<string, unknown>)) {
    if (SKIP_OFFER_ROOT.has(k)) continue;
    rest[k] = v;
  }
  // companyName utile en attrs aussi (title = projection)
  rest.companyName = offer.companyName;

  await db
    .delete(stageOfferAttr)
    .where(
      and(eq(stageOfferAttr.etablissementId, etablissementId), eq(stageOfferAttr.offerId, id)),
    );
  const attrs = flattenToAttrs(rest);
  if (attrs.length > 0) {
    const chunk = 80;
    for (let i = 0; i < attrs.length; i += chunk) {
      await db.insert(stageOfferAttr).values(
        attrs.slice(i, i + chunk).map((a) => ({
          etablissementId,
          offerId: id,
          path: a.path,
          value: a.value,
        })),
      );
    }
  }
}

export async function listOfferIndexFromDb(
  etablissementId: string,
): Promise<StageOfferIndexEntry[]> {
  const list = await ensureOffersMigratedFromCollection(etablissementId);
  return list.map(offerToIndexEntry);
}

export async function ensureOffersMigratedFromCollection(
  etablissementId: string,
): Promise<StageOffer[]> {
  const existing = await listOffersFromDb(etablissementId);
  const existingIds = new Set(existing.map((o) => o.id));
  const { listCollectionRecords, getCollectionRecord } = await import(
    "@/app/lib/ent-collection-db"
  );
  const legacyRows = await listCollectionRecords<Record<string, unknown>>(
    etablissementId,
    "stages__offers",
  );
  for (const row of legacyRows) {
    const id = String(row.id ?? "").trim();
    if (!id || existingIds.has(id)) continue;
    const offer = asOffer(row, id);
    if (!offer) continue;
    await upsertOfferInDb(etablissementId, offer);
    existingIds.add(id);
  }

  const indexRec = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages",
    "offers-index",
  );
  if (indexRec) {
    const raw =
      "__root" in indexRec && Array.isArray(indexRec.__root)
        ? indexRec.__root
        : Array.isArray(indexRec)
          ? indexRec
          : null;
    if (Array.isArray(raw)) {
      for (const entry of raw) {
        const id =
          entry && typeof entry === "object"
            ? String((entry as { id?: string }).id ?? "").trim()
            : "";
        if (!id || existingIds.has(id)) continue;
        const one = await getCollectionRecord<Record<string, unknown>>(
          etablissementId,
          "stages__offers",
          id,
        );
        if (!one) continue;
        const offer = asOffer(one, id);
        if (!offer) continue;
        await upsertOfferInDb(etablissementId, offer);
        existingIds.add(id);
      }
    }
  }
  return listOffersFromDb(etablissementId);
}

export async function getOfferFromDbOrMigrate(
  etablissementId: string,
  id: string,
): Promise<StageOffer | null> {
  const hit = await getOfferFromDb(etablissementId, id);
  if (hit) return hit;
  const { getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  const legacy = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages__offers",
    id,
  );
  if (!legacy) return null;
  const offer = asOffer(legacy, id);
  if (!offer) return null;
  await upsertOfferInDb(etablissementId, offer);
  return getOfferFromDb(etablissementId, id);
}

/* -------------------------------------------------------------------------- */
/* Applications (1 ligne / candidature)                                       */
/* -------------------------------------------------------------------------- */

async function hydrateApplication(
  etablissementId: string,
  m: typeof stageApplication.$inferSelect,
): Promise<StageOfferApplication> {
  const db = getDb();
  const attrs = await db
    .select()
    .from(stageApplicationAttr)
    .where(
      and(
        eq(stageApplicationAttr.etablissementId, etablissementId),
        eq(stageApplicationAttr.applicationId, m.id),
      ),
    );
  const inflated = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
  return {
    id: m.id,
    offerId: String(m.offerId ?? inflated.offerId ?? ""),
    conventionId: String(inflated.conventionId ?? ""),
    studentFirstName: String(inflated.studentFirstName ?? ""),
    studentLastName: String(inflated.studentLastName ?? ""),
    studentClassName: String(inflated.studentClassName ?? ""),
    studentLevel: String(inflated.studentLevel ?? ""),
    createdAt: String(inflated.createdAt ?? m.updatedAt?.toISOString?.() ?? ""),
  };
}

export async function listApplicationsForOfferFromDb(
  etablissementId: string,
  offerId: string,
): Promise<StageOfferApplication[]> {
  await ensureApplicationsMigratedForOffer(etablissementId, offerId);
  const db = getDb();
  const mains = await db
    .select()
    .from(stageApplication)
    .where(
      and(
        eq(stageApplication.etablissementId, etablissementId),
        eq(stageApplication.offerId, offerId),
      ),
    );
  const out: StageOfferApplication[] = [];
  for (const m of mains) {
    out.push(await hydrateApplication(etablissementId, m));
  }
  return out.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}

export async function upsertApplicationInDb(
  etablissementId: string,
  app: StageOfferApplication,
): Promise<void> {
  const db = getDb();
  const id = String(app.id).trim();
  if (!id) throw new Error("[stage-db] application.id requis");
  const offerId = String(app.offerId ?? "").trim() || null;
  const updatedAt = parseTs(app.createdAt);

  await db
    .insert(stageApplication)
    .values({ id, etablissementId, offerId, status: null, updatedAt })
    .onConflictDoUpdate({
      target: stageApplication.id,
      set: { etablissementId, offerId, updatedAt },
    });

  const rest: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(app as unknown as Record<string, unknown>)) {
    if (SKIP_APPLICATION_ROOT.has(k)) continue;
    rest[k] = v;
  }

  await db
    .delete(stageApplicationAttr)
    .where(
      and(
        eq(stageApplicationAttr.etablissementId, etablissementId),
        eq(stageApplicationAttr.applicationId, id),
      ),
    );
  const attrs = flattenToAttrs(rest);
  if (attrs.length > 0) {
    await db.insert(stageApplicationAttr).values(
      attrs.map((a) => ({
        etablissementId,
        applicationId: id,
        path: a.path,
        value: a.value,
      })),
    );
  }
}

/**
 * Remplace les candidatures d’une offre (scoped offerId uniquement — pas de wipe tenant).
 */
export async function replaceOfferApplicationsInDb(
  etablissementId: string,
  offerId: string,
  apps: StageOfferApplication[],
): Promise<void> {
  const db = getDb();
  const existing = await db
    .select({ id: stageApplication.id })
    .from(stageApplication)
    .where(
      and(
        eq(stageApplication.etablissementId, etablissementId),
        eq(stageApplication.offerId, offerId),
      ),
    );
  for (const row of existing) {
    await db
      .delete(stageApplication)
      .where(
        and(
          eq(stageApplication.etablissementId, etablissementId),
          eq(stageApplication.id, row.id),
        ),
      );
  }
  for (const app of apps) {
    await upsertApplicationInDb(etablissementId, { ...app, offerId });
  }
}

async function ensureApplicationsMigratedForOffer(
  etablissementId: string,
  offerId: string,
): Promise<void> {
  const db = getDb();
  const [any] = await db
    .select({ id: stageApplication.id })
    .from(stageApplication)
    .where(
      and(
        eq(stageApplication.etablissementId, etablissementId),
        eq(stageApplication.offerId, offerId),
      ),
    )
    .limit(1);
  if (any) return;

  const { getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  const legacy = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages__offer-applications",
    offerId,
  );
  if (!legacy) return;
  const list =
    "__root" in legacy && Array.isArray(legacy.__root)
      ? legacy.__root
      : Array.isArray(legacy)
        ? legacy
        : null;
  if (!Array.isArray(list) || list.length === 0) return;
  for (const raw of list) {
    if (!raw || typeof raw !== "object") continue;
    const app = raw as StageOfferApplication;
    if (!app.id) continue;
    await upsertApplicationInDb(etablissementId, { ...app, offerId });
  }
}

/* -------------------------------------------------------------------------- */
/* Tokens + docs config / OTP (stage_token)                                   */
/* -------------------------------------------------------------------------- */

export function stageTokenStorageKey(kind: StageTokenKind, key: string): string {
  return `${kind}:${key}`;
}

async function hydrateTokenPayload<T extends Record<string, unknown>>(
  etablissementId: string,
  tokenPk: string,
): Promise<T | null> {
  const db = getDb();
  const [m] = await db
    .select()
    .from(stageToken)
    .where(and(eq(stageToken.etablissementId, etablissementId), eq(stageToken.token, tokenPk)))
    .limit(1);
  if (!m) return null;
  const attrs = await db
    .select()
    .from(stageTokenAttr)
    .where(
      and(eq(stageTokenAttr.etablissementId, etablissementId), eq(stageTokenAttr.token, tokenPk)),
    );
  return inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value }))) as T;
}

export async function upsertStageTokenInDb(
  etablissementId: string,
  kind: StageTokenKind,
  key: string,
  payload: Record<string, unknown>,
): Promise<void> {
  const db = getDb();
  const tokenPk = stageTokenStorageKey(kind, key);
  const updatedAt = new Date();

  await db
    .insert(stageToken)
    .values({ token: tokenPk, etablissementId, kind, updatedAt })
    .onConflictDoUpdate({
      target: stageToken.token,
      set: { etablissementId, kind, updatedAt },
    });

  await db
    .delete(stageTokenAttr)
    .where(
      and(eq(stageTokenAttr.etablissementId, etablissementId), eq(stageTokenAttr.token, tokenPk)),
    );
  const attrs = flattenToAttrs(payload);
  if (attrs.length > 0) {
    await db.insert(stageTokenAttr).values(
      attrs.map((a) => ({
        etablissementId,
        token: tokenPk,
        path: a.path,
        value: a.value,
      })),
    );
  }
}

export async function getStageTokenFromDb<T extends Record<string, unknown>>(
  etablissementId: string,
  kind: StageTokenKind,
  key: string,
): Promise<T | null> {
  const tokenPk = stageTokenStorageKey(kind, key);
  const hit = await hydrateTokenPayload<T>(etablissementId, tokenPk);
  if (hit) return hit;
  return migrateOneTokenFromCollection(etablissementId, kind, key);
}

export async function deleteStageTokenFromDb(
  etablissementId: string,
  kind: StageTokenKind,
  key: string,
): Promise<void> {
  const db = getDb();
  const tokenPk = stageTokenStorageKey(kind, key);
  await db
    .delete(stageToken)
    .where(and(eq(stageToken.etablissementId, etablissementId), eq(stageToken.token, tokenPk)));
}

function collectionForTokenKind(kind: StageTokenKind): string | null {
  switch (kind) {
    case "sign":
      return "stages__sign-tokens";
    case "sign_code":
      return "stages__sign-code-lookup";
    case "student":
      return "stages__student-tokens";
    case "offer_candidature":
      return "stages__offer-candidature-tokens";
    case "periods":
      return "stages__periods";
    case "referents":
      return "stages__referents";
    case "constraints":
      return "stages__constraints";
    case "watchers":
      return "stages__watchers";
    case "identity_otp":
      return "stages__identity-otp";
    case "identity_recipient_choice":
      return "stages__identity-recipient-choice";
    case "identity_proof":
      return "stages__identity-proof";
    case "auto_purge":
      return "stages";
    default:
      return null;
  }
}

function payloadFromCollectionRow(row: Record<string, unknown>): Record<string, unknown> {
  if ("__root" in row && Object.keys(row).every((k) => k === "__root" || k === "id")) {
    const root = row.__root;
    if (root && typeof root === "object" && !Array.isArray(root)) {
      return { ...(root as Record<string, unknown>) };
    }
    return { __root: root as unknown };
  }
  const copy = { ...row };
  delete copy.id;
  return copy;
}

async function migrateOneTokenFromCollection<T extends Record<string, unknown>>(
  etablissementId: string,
  kind: StageTokenKind,
  key: string,
): Promise<T | null> {
  const collection = collectionForTokenKind(kind);
  if (!collection) return null;
  const { getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  const recordId = kind === "auto_purge" ? "auto-purge-state" : key;
  const legacy = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    collection,
    recordId,
  );
  if (!legacy) return null;
  const obj = payloadFromCollectionRow(legacy);
  await upsertStageTokenInDb(etablissementId, kind, key, obj);
  return hydrateTokenPayload<T>(etablissementId, stageTokenStorageKey(kind, key));
}

/** Migre en masse les tokens d’un kind depuis ent_collection (ids manquants seulement). */
export async function ensureTokensMigratedFromCollection(
  etablissementId: string,
  kind: StageTokenKind,
): Promise<number> {
  const collection = collectionForTokenKind(kind);
  if (!collection) return 0;

  // `stages` contient aussi offers-index / conventions-index — ne prendre que auto-purge-state.
  if (kind === "auto_purge") {
    const hit = await migrateOneTokenFromCollection(etablissementId, "auto_purge", "state");
    return hit ? 1 : 0;
  }

  const { listCollectionRecords } = await import("@/app/lib/ent-collection-db");
  const rows = await listCollectionRecords<Record<string, unknown>>(etablissementId, collection);
  let n = 0;
  for (const row of rows) {
    const storageKey = String(row.id ?? "").trim();
    if (!storageKey) continue;
    const existing = await hydrateTokenPayload(
      etablissementId,
      stageTokenStorageKey(kind, storageKey),
    );
    if (existing) continue;
    await upsertStageTokenInDb(etablissementId, kind, storageKey, payloadFromCollectionRow(row));
    n += 1;
  }
  return n;
}

/* Typed helpers for callers / bridge */

export async function saveSignTokenTyped(
  etablissementId: string,
  token: string,
  ref: StageSignTokenRef,
): Promise<void> {
  await upsertStageTokenInDb(etablissementId, "sign", token, { ...ref });
}

export async function getSignTokenTyped(
  etablissementId: string,
  token: string,
): Promise<StageSignTokenRef | null> {
  return getStageTokenFromDb<StageSignTokenRef>(etablissementId, "sign", token);
}

export async function saveStudentTokenTyped(
  etablissementId: string,
  token: string,
  ref: StageStudentTokenRef,
): Promise<void> {
  await upsertStageTokenInDb(etablissementId, "student", token, { ...ref });
}

export async function getStudentTokenTyped(
  etablissementId: string,
  token: string,
): Promise<StageStudentTokenRef | null> {
  return getStageTokenFromDb<StageStudentTokenRef>(etablissementId, "student", token);
}

export async function saveOfferCandidatureTokenTyped(
  etablissementId: string,
  token: string,
  ref: StageOfferCandidatureTokenRef,
): Promise<void> {
  await upsertStageTokenInDb(etablissementId, "offer_candidature", token, { ...ref });
}

export async function getOfferCandidatureTokenTyped(
  etablissementId: string,
  token: string,
): Promise<StageOfferCandidatureTokenRef | null> {
  return getStageTokenFromDb<StageOfferCandidatureTokenRef>(
    etablissementId,
    "offer_candidature",
    token,
  );
}

export async function saveSignCodeLookupTyped(
  etablissementId: string,
  lookupKey: string,
  ref: StageSignCodeLookupRef,
): Promise<void> {
  await upsertStageTokenInDb(etablissementId, "sign_code", lookupKey, { ...ref });
}

export async function getSignCodeLookupTyped(
  etablissementId: string,
  lookupKey: string,
): Promise<StageSignCodeLookupRef | null> {
  return getStageTokenFromDb<StageSignCodeLookupRef>(etablissementId, "sign_code", lookupKey);
}

/** Backfill one-shot : toutes collections stages → tables typées (idempotent). */
export async function migrateAllStagesFromCollection(etablissementId: string): Promise<{
  conventions: number;
  offers: number;
  tokens: number;
}> {
  const conventions = await ensureConventionsMigratedFromCollection(etablissementId);
  const offers = await ensureOffersMigratedFromCollection(etablissementId);
  const kinds: StageTokenKind[] = [
    "sign",
    "sign_code",
    "student",
    "offer_candidature",
    "periods",
    "referents",
    "constraints",
    "watchers",
    "auto_purge",
    "identity_otp",
    "identity_recipient_choice",
    "identity_proof",
  ];
  let tokens = 0;
  for (const kind of kinds) {
    tokens += await ensureTokensMigratedFromCollection(etablissementId, kind);
  }
  // applications: per offer from offers list + legacy collection
  for (const o of offers) {
    await ensureApplicationsMigratedForOffer(etablissementId, o.id);
  }
  return { conventions: conventions.length, offers: offers.length, tokens };
}
