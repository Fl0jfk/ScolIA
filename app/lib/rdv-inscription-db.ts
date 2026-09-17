import "server-only";

import { and, asc, desc, eq } from "drizzle-orm";
import { randomUUID } from "crypto";
import { getDb, isDatabaseConfigured } from "@/db/index";
import {
  rdvInscriptionBooking,
  rdvInscriptionConfig,
  rdvInscriptionDirection,
} from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  DEFAULT_RDV_DIRECTIONS,
  DEFAULT_RDV_INSCRIPTION_CONSENT,
  DEFAULT_RDV_INSCRIPTION_PATTERN,
  DEFAULT_RDV_INSCRIPTION_TITLE,
  isValidDirectionSlug,
  type RdvInscriptionBookingRow,
  type RdvInscriptionConfigPublic,
  type RdvInscriptionDirectionRow,
} from "@/app/lib/rdv-inscription-types";

async function requireEtabId(explicit?: string): Promise<string> {
  const id = explicit || (await resolveCurrentEtablissementId());
  if (!id) throw new Error("Établissement introuvable (tenant).");
  return id;
}

function requireDb() {
  if (!isDatabaseConfigured()) {
    throw new Error("Base de données non configurée.");
  }
  return getDb();
}

function toIso(d: Date | null | undefined): string | null {
  if (!d) return null;
  return d.toISOString();
}

function mapConfig(row: typeof rdvInscriptionConfig.$inferSelect): RdvInscriptionConfigPublic {
  return {
    enabled: row.enabled === 1,
    title: row.title,
    intro: row.intro,
    eventTitlePattern: row.eventTitlePattern,
    notifyEmail: row.notifyEmail?.trim() || null,
    location: row.location || "",
    consentLabel: row.consentLabel,
    horizonDays: row.horizonDays > 0 ? row.horizonDays : 60,
    googleLinked: row.googleLinked === 1,
    googleLinkedEmail: row.googleLinkedEmail?.trim() || null,
    googleLinkedAt: toIso(row.googleLinkedAt),
  };
}

function mapDirection(row: typeof rdvInscriptionDirection.$inferSelect): RdvInscriptionDirectionRow {
  return {
    id: row.id,
    slug: row.slug,
    label: row.label,
    googleCalendarId: row.googleCalendarId || "",
    directriceDisplayName: row.directriceDisplayName?.trim() || null,
    active: row.active === 1,
    sortOrder: row.sortOrder,
  };
}

function mapBooking(row: typeof rdvInscriptionBooking.$inferSelect): RdvInscriptionBookingRow {
  return {
    id: row.id,
    directionId: row.directionId,
    directionSlug: row.directionSlug,
    googleEventId: row.googleEventId,
    googleCalendarId: row.googleCalendarId,
    googleHtmlLink: row.googleHtmlLink,
    startAt: row.startAt.toISOString(),
    endAt: row.endAt.toISOString(),
    studentFirstName: row.studentFirstName,
    studentLastName: row.studentLastName,
    parentEmail: row.parentEmail,
    parentPhone: row.parentPhone,
    status: row.status === "cancelled" ? "cancelled" : "confirmed",
    createdAt: row.createdAt.toISOString(),
  };
}

/** Assure une ligne config + directions par défaut (école / collège / lycée). */
export async function ensureRdvInscriptionDefaults(etablissementId?: string): Promise<void> {
  const etabId = await requireEtabId(etablissementId);
  const db = requireDb();

  const existing = await db
    .select({ id: rdvInscriptionConfig.id })
    .from(rdvInscriptionConfig)
    .where(eq(rdvInscriptionConfig.etablissementId, etabId))
    .limit(1);

  if (existing.length === 0) {
    await db.insert(rdvInscriptionConfig).values({
      etablissementId: etabId,
      enabled: 0,
      title: DEFAULT_RDV_INSCRIPTION_TITLE,
      intro:
        "Choisissez un créneau pour rencontrer la direction après votre inscription / préinscription.",
      eventTitlePattern: DEFAULT_RDV_INSCRIPTION_PATTERN,
      consentLabel: DEFAULT_RDV_INSCRIPTION_CONSENT,
      horizonDays: 60,
      googleLinked: 0,
    });
  }

  const dirs = await db
    .select({ slug: rdvInscriptionDirection.slug })
    .from(rdvInscriptionDirection)
    .where(eq(rdvInscriptionDirection.etablissementId, etabId));
  const have = new Set(dirs.map((d) => d.slug));
  for (const d of DEFAULT_RDV_DIRECTIONS) {
    if (have.has(d.slug)) continue;
    await db.insert(rdvInscriptionDirection).values({
      etablissementId: etabId,
      slug: d.slug,
      label: d.label,
      googleCalendarId: "",
      active: 0,
      sortOrder: d.sortOrder,
    });
  }
}

export async function getRdvInscriptionConfig(
  etablissementId?: string,
): Promise<RdvInscriptionConfigPublic> {
  await ensureRdvInscriptionDefaults(etablissementId);
  const etabId = await requireEtabId(etablissementId);
  const db = requireDb();
  const rows = await db
    .select()
    .from(rdvInscriptionConfig)
    .where(eq(rdvInscriptionConfig.etablissementId, etabId))
    .limit(1);
  if (!rows[0]) {
    return {
      enabled: false,
      title: DEFAULT_RDV_INSCRIPTION_TITLE,
      intro: "",
      eventTitlePattern: DEFAULT_RDV_INSCRIPTION_PATTERN,
      notifyEmail: null,
      location: "",
      consentLabel: DEFAULT_RDV_INSCRIPTION_CONSENT,
      horizonDays: 60,
      googleLinked: false,
      googleLinkedEmail: null,
      googleLinkedAt: null,
    };
  }
  return mapConfig(rows[0]);
}

export async function updateRdvInscriptionConfig(
  patch: Partial<{
    enabled: boolean;
    title: string;
    intro: string;
    eventTitlePattern: string;
    notifyEmail: string | null;
    location: string;
    consentLabel: string;
    horizonDays: number;
  }>,
  etablissementId?: string,
): Promise<RdvInscriptionConfigPublic> {
  await ensureRdvInscriptionDefaults(etablissementId);
  const etabId = await requireEtabId(etablissementId);
  const db = requireDb();

  const updates: Partial<typeof rdvInscriptionConfig.$inferInsert> = {
    updatedAt: new Date(),
  };
  if (typeof patch.enabled === "boolean") updates.enabled = patch.enabled ? 1 : 0;
  if (typeof patch.title === "string") updates.title = patch.title.trim() || DEFAULT_RDV_INSCRIPTION_TITLE;
  if (typeof patch.intro === "string") updates.intro = patch.intro;
  if (typeof patch.eventTitlePattern === "string") {
    updates.eventTitlePattern =
      patch.eventTitlePattern.trim() || DEFAULT_RDV_INSCRIPTION_PATTERN;
  }
  if (patch.notifyEmail !== undefined) {
    updates.notifyEmail = patch.notifyEmail?.trim() || null;
  }
  if (typeof patch.location === "string") updates.location = patch.location.trim();
  if (typeof patch.consentLabel === "string") {
    updates.consentLabel = patch.consentLabel.trim() || DEFAULT_RDV_INSCRIPTION_CONSENT;
  }
  if (typeof patch.horizonDays === "number" && Number.isFinite(patch.horizonDays)) {
    updates.horizonDays = Math.min(180, Math.max(7, Math.round(patch.horizonDays)));
  }

  await db
    .update(rdvInscriptionConfig)
    .set(updates)
    .where(eq(rdvInscriptionConfig.etablissementId, etabId));

  return getRdvInscriptionConfig(etabId);
}

export async function markRdvInscriptionGoogleLinked(input: {
  linked: boolean;
  email?: string | null;
  linkedAt?: Date | null;
}): Promise<void> {
  await ensureRdvInscriptionDefaults();
  const etabId = await requireEtabId();
  const db = requireDb();
  await db
    .update(rdvInscriptionConfig)
    .set({
      googleLinked: input.linked ? 1 : 0,
      googleLinkedEmail: input.linked ? input.email?.trim() || null : null,
      googleLinkedAt: input.linked ? input.linkedAt || new Date() : null,
      updatedAt: new Date(),
    })
    .where(eq(rdvInscriptionConfig.etablissementId, etabId));
}

export async function listRdvInscriptionDirections(opts?: {
  activeOnly?: boolean;
  etablissementId?: string;
}): Promise<RdvInscriptionDirectionRow[]> {
  await ensureRdvInscriptionDefaults(opts?.etablissementId);
  const etabId = await requireEtabId(opts?.etablissementId);
  const db = requireDb();
  const rows = await db
    .select()
    .from(rdvInscriptionDirection)
    .where(
      opts?.activeOnly
        ? and(
            eq(rdvInscriptionDirection.etablissementId, etabId),
            eq(rdvInscriptionDirection.active, 1),
          )
        : eq(rdvInscriptionDirection.etablissementId, etabId),
    )
    .orderBy(asc(rdvInscriptionDirection.sortOrder), asc(rdvInscriptionDirection.label));
  return rows.map(mapDirection);
}

export async function getRdvInscriptionDirectionBySlug(
  slug: string,
  opts?: { activeOnly?: boolean; etablissementId?: string },
): Promise<RdvInscriptionDirectionRow | null> {
  await ensureRdvInscriptionDefaults(opts?.etablissementId);
  const etabId = await requireEtabId(opts?.etablissementId);
  const db = requireDb();
  const normalized = slug.trim().toLowerCase();
  const rows = await db
    .select()
    .from(rdvInscriptionDirection)
    .where(
      and(
        eq(rdvInscriptionDirection.etablissementId, etabId),
        eq(rdvInscriptionDirection.slug, normalized),
      ),
    )
    .limit(1);
  const row = rows[0];
  if (!row) return null;
  if (opts?.activeOnly && row.active !== 1) return null;
  return mapDirection(row);
}

export async function upsertRdvInscriptionDirection(
  input: {
    id?: string;
    slug: string;
    label: string;
    googleCalendarId: string;
    directriceDisplayName?: string | null;
    active: boolean;
    sortOrder?: number;
  },
  etablissementId?: string,
): Promise<RdvInscriptionDirectionRow> {
  await ensureRdvInscriptionDefaults(etablissementId);
  const etabId = await requireEtabId(etablissementId);
  const db = requireDb();
  const slug = input.slug.trim().toLowerCase();
  if (!isValidDirectionSlug(slug)) {
    throw new Error("Slug invalide (lettres minuscules, chiffres, tirets).");
  }
  const label = input.label.trim();
  if (!label) throw new Error("Libellé requis.");

  if (input.id) {
    const updates: Partial<typeof rdvInscriptionDirection.$inferInsert> = {
      slug,
      label,
      googleCalendarId: input.googleCalendarId.trim(),
      directriceDisplayName: input.directriceDisplayName?.trim() || null,
      active: input.active ? 1 : 0,
      updatedAt: new Date(),
    };
    if (typeof input.sortOrder === "number" && Number.isFinite(input.sortOrder)) {
      updates.sortOrder = Math.round(input.sortOrder);
    }
    await db
      .update(rdvInscriptionDirection)
      .set(updates)
      .where(
        and(
          eq(rdvInscriptionDirection.etablissementId, etabId),
          eq(rdvInscriptionDirection.id, input.id),
        ),
      );
    const rows = await db
      .select()
      .from(rdvInscriptionDirection)
      .where(
        and(
          eq(rdvInscriptionDirection.etablissementId, etabId),
          eq(rdvInscriptionDirection.id, input.id),
        ),
      )
      .limit(1);
    if (!rows[0]) throw new Error("Direction introuvable.");
    return mapDirection(rows[0]);
  }

  const id = randomUUID();
  await db.insert(rdvInscriptionDirection).values({
    id,
    etablissementId: etabId,
    slug,
    label,
    googleCalendarId: input.googleCalendarId.trim(),
    directriceDisplayName: input.directriceDisplayName?.trim() || null,
    active: input.active ? 1 : 0,
    sortOrder:
      typeof input.sortOrder === "number" && Number.isFinite(input.sortOrder)
        ? Math.round(input.sortOrder)
        : 99,
  });
  const rows = await db
    .select()
    .from(rdvInscriptionDirection)
    .where(
      and(eq(rdvInscriptionDirection.etablissementId, etabId), eq(rdvInscriptionDirection.id, id)),
    )
    .limit(1);
  if (!rows[0]) throw new Error("Échec création direction.");
  return mapDirection(rows[0]);
}

export async function insertRdvInscriptionBooking(input: {
  directionId: string;
  directionSlug: string;
  googleEventId: string;
  googleCalendarId: string;
  googleHtmlLink?: string | null;
  startAt: Date;
  endAt: Date;
  studentFirstName: string;
  studentLastName: string;
  parentEmail: string;
  parentPhone: string;
  bookingId?: string;
  etablissementId?: string;
}): Promise<RdvInscriptionBookingRow> {
  const etabId = await requireEtabId(input.etablissementId);
  const db = requireDb();
  const id = input.bookingId || randomUUID();

  await db.insert(rdvInscriptionBooking).values({
    id,
    etablissementId: etabId,
    directionId: input.directionId,
    directionSlug: input.directionSlug,
    googleEventId: input.googleEventId,
    googleCalendarId: input.googleCalendarId,
    googleHtmlLink: input.googleHtmlLink || null,
    startAt: input.startAt,
    endAt: input.endAt,
    studentFirstName: input.studentFirstName.trim(),
    studentLastName: input.studentLastName.trim(),
    parentEmail: input.parentEmail.trim().toLowerCase(),
    parentPhone: input.parentPhone.trim(),
    status: "confirmed",
  });

  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(and(eq(rdvInscriptionBooking.etablissementId, etabId), eq(rdvInscriptionBooking.id, id)))
    .limit(1);
  if (!rows[0]) throw new Error("Échec enregistrement réservation.");
  return mapBooking(rows[0]);
}

export async function findBookingByGoogleEvent(opts: {
  calendarId: string;
  eventId: string;
  etablissementId?: string;
}): Promise<RdvInscriptionBookingRow | null> {
  const etabId = await requireEtabId(opts.etablissementId);
  const db = requireDb();
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, etabId),
        eq(rdvInscriptionBooking.googleCalendarId, opts.calendarId),
        eq(rdvInscriptionBooking.googleEventId, opts.eventId),
        eq(rdvInscriptionBooking.status, "confirmed"),
      ),
    )
    .limit(1);
  return rows[0] ? mapBooking(rows[0]) : null;
}

export async function listRdvInscriptionBookings(opts?: {
  directionSlug?: string;
  limit?: number;
  etablissementId?: string;
}): Promise<RdvInscriptionBookingRow[]> {
  const etabId = await requireEtabId(opts?.etablissementId);
  const db = requireDb();
  const limit = Math.min(200, Math.max(1, opts?.limit ?? 100));
  const conditions = [eq(rdvInscriptionBooking.etablissementId, etabId)];
  if (opts?.directionSlug) {
    conditions.push(eq(rdvInscriptionBooking.directionSlug, opts.directionSlug.trim().toLowerCase()));
  }
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(and(...conditions))
    .orderBy(desc(rdvInscriptionBooking.startAt))
    .limit(limit);
  return rows.map(mapBooking);
}
