import "server-only";

import { and, asc, desc, eq, inArray } from "drizzle-orm";
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
  type RdvInscriptionBookingStatus,
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

const DEFAULT_INTRO =
  "Choisissez un créneau pour rencontrer la direction après votre inscription / préinscription.";

function mapConfig(row: typeof rdvInscriptionConfig.$inferSelect): RdvInscriptionConfigPublic {
  return {
    enabled: row.enabled === 1,
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
    title: row.title?.trim() || DEFAULT_RDV_INSCRIPTION_TITLE,
    intro: row.intro ?? "",
    eventTitlePattern: row.eventTitlePattern?.trim() || DEFAULT_RDV_INSCRIPTION_PATTERN,
    notifyEmail: row.notifyEmail?.trim() || null,
    location: row.location || "",
    consentLabel: row.consentLabel?.trim() || DEFAULT_RDV_INSCRIPTION_CONSENT,
    horizonDays: row.horizonDays > 0 ? row.horizonDays : 60,
    active: row.active === 1,
    sortOrder: row.sortOrder,
  };
}

function mapBookingStatus(raw: string): RdvInscriptionBookingStatus {
  if (raw === "pending") return "pending";
  if (raw === "cancelled") return "cancelled";
  if (raw === "expired") return "expired";
  return "confirmed";
}

function mapBooking(row: typeof rdvInscriptionBooking.$inferSelect): RdvInscriptionBookingRow {
  const matchRaw = row.matchStatus?.trim() || null;
  const matchStatus =
    matchRaw === "confirmed" || matchRaw === "created" ? matchRaw : null;
  const reconfirmRaw = row.reconfirmStatus?.trim() || null;
  const reconfirmStatus =
    reconfirmRaw === "pending" || reconfirmRaw === "ok" || reconfirmRaw === "cancelled"
      ? reconfirmRaw
      : null;
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
    parentFirstName: row.parentFirstName?.trim() || null,
    parentLastName: row.parentLastName?.trim() || null,
    rdvAttendee:
      row.rdvAttendee === "madame" ||
      row.rdvAttendee === "monsieur" ||
      row.rdvAttendee === "les_deux"
        ? row.rdvAttendee
        : null,
    niveauId: row.niveauId,
    niveauLabel: row.niveauLabel,
    regime:
      row.regime === "DP" || row.regime === "EXT" || row.regime === "INT" ? row.regime : null,
    eleveId: row.eleveId,
    matchStatus,
    createNew: row.createNew === 1,
    hasPap: row.hasPap === "yes" || row.hasPap === "no" ? row.hasPap : null,
    papS3Key: row.papS3Key,
    papFileName: row.papFileName,
    papMimeType: row.papMimeType,
    papBringToRdv: row.papBringToRdv === 1,
    etablissementOrigineRne: row.etablissementOrigineRne,
    etablissementOrigineLabel: row.etablissementOrigineLabel,
    etablissementOrigineAdresse: row.etablissementOrigineAdresse,
    status: mapBookingStatus(row.status),
    confirmExpiresAt: toIso(row.confirmExpiresAt),
    confirmedAt: toIso(row.confirmedAt),
    reconfirmStatus,
    reconfirmMailSentAt: toIso(row.reconfirmMailSentAt),
    reconfirmedAt: toIso(row.reconfirmedAt),
    adminCancelNote: row.adminCancelNote?.trim() || null,
    createdAt: row.createdAt.toISOString(),
  };
}

function directionDefaultsFromConfig(cfg?: {
  title?: string | null;
  intro?: string | null;
  eventTitlePattern?: string | null;
  notifyEmail?: string | null;
  location?: string | null;
  consentLabel?: string | null;
  horizonDays?: number | null;
}) {
  return {
    title: cfg?.title?.trim() || DEFAULT_RDV_INSCRIPTION_TITLE,
    intro: cfg?.intro ?? DEFAULT_INTRO,
    eventTitlePattern: cfg?.eventTitlePattern?.trim() || DEFAULT_RDV_INSCRIPTION_PATTERN,
    notifyEmail: cfg?.notifyEmail?.trim() || null,
    location: cfg?.location?.trim() || "",
    consentLabel: cfg?.consentLabel?.trim() || DEFAULT_RDV_INSCRIPTION_CONSENT,
    horizonDays:
      typeof cfg?.horizonDays === "number" && cfg.horizonDays > 0 ? cfg.horizonDays : 60,
  };
}

/** Assure une ligne config + directions par défaut (école / collège / lycée). */
export async function ensureRdvInscriptionDefaults(etablissementId?: string): Promise<void> {
  const etabId = await requireEtabId(etablissementId);
  const db = requireDb();

  const existing = await db
    .select()
    .from(rdvInscriptionConfig)
    .where(eq(rdvInscriptionConfig.etablissementId, etabId))
    .limit(1);

  if (existing.length === 0) {
    await db.insert(rdvInscriptionConfig).values({
      etablissementId: etabId,
      enabled: 0,
      title: DEFAULT_RDV_INSCRIPTION_TITLE,
      intro: DEFAULT_INTRO,
      eventTitlePattern: DEFAULT_RDV_INSCRIPTION_PATTERN,
      consentLabel: DEFAULT_RDV_INSCRIPTION_CONSENT,
      horizonDays: 60,
      googleLinked: 0,
    });
  }

  const cfgRow = existing[0];
  const seed = directionDefaultsFromConfig(cfgRow);

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
      ...seed,
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
      googleLinked: false,
      googleLinkedEmail: null,
      googleLinkedAt: null,
    };
  }
  return mapConfig(rows[0]);
}

export async function updateRdvInscriptionConfig(
  patch: Partial<{ enabled: boolean }>,
  etablissementId?: string,
): Promise<RdvInscriptionConfigPublic> {
  await ensureRdvInscriptionDefaults(etablissementId);
  const etabId = await requireEtabId(etablissementId);
  const db = requireDb();

  const updates: Partial<typeof rdvInscriptionConfig.$inferInsert> = {
    updatedAt: new Date(),
  };
  if (typeof patch.enabled === "boolean") updates.enabled = patch.enabled ? 1 : 0;

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
  refreshToken?: string | null;
}): Promise<void> {
  await ensureRdvInscriptionDefaults();
  const etabId = await requireEtabId();
  const db = requireDb();
  const patch: Partial<typeof rdvInscriptionConfig.$inferInsert> = {
    googleLinked: input.linked ? 1 : 0,
    googleLinkedEmail: input.linked ? input.email?.trim() || null : null,
    googleLinkedAt: input.linked ? input.linkedAt || new Date() : null,
    updatedAt: new Date(),
  };
  if (input.linked) {
    if (typeof input.refreshToken === "string" && input.refreshToken.trim()) {
      patch.googleRefreshToken = input.refreshToken.trim();
    }
  } else {
    patch.googleRefreshToken = null;
  }
  await db
    .update(rdvInscriptionConfig)
    .set(patch)
    .where(eq(rdvInscriptionConfig.etablissementId, etabId));
}

/** Refresh token Google stocké en BDD (prioritaire pour les appels Agenda). */
export async function getRdvInscriptionGoogleRefreshToken(
  etablissementId?: string,
): Promise<string | null> {
  await ensureRdvInscriptionDefaults(etablissementId);
  const etabId = await requireEtabId(etablissementId);
  const db = requireDb();
  const rows = await db
    .select({ token: rdvInscriptionConfig.googleRefreshToken })
    .from(rdvInscriptionConfig)
    .where(eq(rdvInscriptionConfig.etablissementId, etabId))
    .limit(1);
  const token = rows[0]?.token?.trim();
  return token || null;
}

export async function persistRdvInscriptionGoogleRefreshToken(
  refreshToken: string,
  etablissementId?: string,
): Promise<void> {
  await ensureRdvInscriptionDefaults(etablissementId);
  const etabId = await requireEtabId(etablissementId);
  const db = requireDb();
  await db
    .update(rdvInscriptionConfig)
    .set({
      googleRefreshToken: refreshToken.trim(),
      googleLinked: 1,
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

export async function getRdvInscriptionDirectionById(
  id: string,
  etablissementId?: string,
): Promise<RdvInscriptionDirectionRow | null> {
  await ensureRdvInscriptionDefaults(etablissementId);
  const etabId = await requireEtabId(etablissementId);
  const db = requireDb();
  const rows = await db
    .select()
    .from(rdvInscriptionDirection)
    .where(
      and(eq(rdvInscriptionDirection.etablissementId, etabId), eq(rdvInscriptionDirection.id, id)),
    )
    .limit(1);
  return rows[0] ? mapDirection(rows[0]) : null;
}

export async function upsertRdvInscriptionDirection(
  input: {
    id?: string;
    slug: string;
    label: string;
    googleCalendarId: string;
    directriceDisplayName?: string | null;
    title?: string;
    intro?: string;
    eventTitlePattern?: string;
    notifyEmail?: string | null;
    location?: string;
    consentLabel?: string;
    horizonDays?: number;
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

  const pageFields: Partial<typeof rdvInscriptionDirection.$inferInsert> = {};
  if (typeof input.title === "string") {
    pageFields.title = input.title.trim() || DEFAULT_RDV_INSCRIPTION_TITLE;
  }
  if (typeof input.intro === "string") pageFields.intro = input.intro;
  if (typeof input.eventTitlePattern === "string") {
    pageFields.eventTitlePattern =
      input.eventTitlePattern.trim() || DEFAULT_RDV_INSCRIPTION_PATTERN;
  }
  if (input.notifyEmail !== undefined) {
    pageFields.notifyEmail = input.notifyEmail?.trim() || null;
  }
  if (typeof input.location === "string") pageFields.location = input.location.trim();
  if (typeof input.consentLabel === "string") {
    pageFields.consentLabel = input.consentLabel.trim() || DEFAULT_RDV_INSCRIPTION_CONSENT;
  }
  if (typeof input.horizonDays === "number" && Number.isFinite(input.horizonDays)) {
    pageFields.horizonDays = Math.min(180, Math.max(7, Math.round(input.horizonDays)));
  }

  if (input.id) {
    const updates: Partial<typeof rdvInscriptionDirection.$inferInsert> = {
      slug,
      label,
      googleCalendarId: input.googleCalendarId.trim(),
      directriceDisplayName: input.directriceDisplayName?.trim() || null,
      active: input.active ? 1 : 0,
      updatedAt: new Date(),
      ...pageFields,
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

    // Activer une direction ⇒ ouvrir le module public automatiquement.
    if (input.active) {
      await db
        .update(rdvInscriptionConfig)
        .set({ enabled: 1, updatedAt: new Date() })
        .where(eq(rdvInscriptionConfig.etablissementId, etabId));
    }

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
  const seed = directionDefaultsFromConfig();
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
    title: pageFields.title ?? seed.title,
    intro: pageFields.intro ?? seed.intro,
    eventTitlePattern: pageFields.eventTitlePattern ?? seed.eventTitlePattern,
    notifyEmail: pageFields.notifyEmail !== undefined ? pageFields.notifyEmail : seed.notifyEmail,
    location: pageFields.location ?? seed.location,
    consentLabel: pageFields.consentLabel ?? seed.consentLabel,
    horizonDays: pageFields.horizonDays ?? seed.horizonDays,
  });

  if (input.active) {
    await db
      .update(rdvInscriptionConfig)
      .set({ enabled: 1, updatedAt: new Date() })
      .where(eq(rdvInscriptionConfig.etablissementId, etabId));
  }

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
  parentFirstName?: string | null;
  parentLastName?: string | null;
  rdvAttendee?: "madame" | "monsieur" | "les_deux" | null;
  niveauId?: string | null;
  niveauLabel?: string | null;
  regime?: "DP" | "EXT" | "INT" | null;
  eleveId?: string | null;
  createNew?: boolean;
  hasPap?: "yes" | "no" | null;
  papS3Key?: string | null;
  papFileName?: string | null;
  papMimeType?: string | null;
  papBringToRdv?: boolean;
  etablissementOrigineRne?: string | null;
  etablissementOrigineLabel?: string | null;
  etablissementOrigineAdresse?: string | null;
  status?: RdvInscriptionBookingStatus;
  confirmToken?: string | null;
  confirmExpiresAt?: Date | null;
  bookingId?: string;
  etablissementId?: string;
}): Promise<RdvInscriptionBookingRow> {
  const etabId = await requireEtabId(input.etablissementId);
  const db = requireDb();
  const id = input.bookingId || randomUUID();
  const status = input.status || "pending";

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
    parentFirstName: input.parentFirstName?.trim() || null,
    parentLastName: input.parentLastName?.trim() || null,
    rdvAttendee:
      input.rdvAttendee === "madame" ||
      input.rdvAttendee === "monsieur" ||
      input.rdvAttendee === "les_deux"
        ? input.rdvAttendee
        : null,
    niveauId: input.niveauId?.trim() || null,
    niveauLabel: input.niveauLabel?.trim() || null,
    regime:
      input.regime === "DP" || input.regime === "EXT" || input.regime === "INT"
        ? input.regime
        : null,
    eleveId: input.eleveId?.trim() || null,
    createNew: input.createNew ? 1 : 0,
    hasPap: input.hasPap === "yes" || input.hasPap === "no" ? input.hasPap : null,
    papS3Key: input.papS3Key?.trim() || null,
    papFileName: input.papFileName?.trim() || null,
    papMimeType: input.papMimeType?.trim() || null,
    papBringToRdv: input.papBringToRdv ? 1 : 0,
    etablissementOrigineRne: input.etablissementOrigineRne?.trim() || null,
    etablissementOrigineLabel: input.etablissementOrigineLabel?.trim() || null,
    etablissementOrigineAdresse: input.etablissementOrigineAdresse?.trim() || null,
    status,
    confirmToken: input.confirmToken || null,
    confirmExpiresAt: input.confirmExpiresAt || null,
    confirmedAt: status === "confirmed" ? new Date() : null,
  });

  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(and(eq(rdvInscriptionBooking.etablissementId, etabId), eq(rdvInscriptionBooking.id, id)))
    .limit(1);
  if (!rows[0]) throw new Error("Échec enregistrement réservation.");
  return mapBooking(rows[0]);
}

/** Réservation active (confirmée ou pending non expirée) pour un événement Google. */
export async function findActiveBookingByGoogleEvent(opts: {
  calendarId: string;
  eventId: string;
  etablissementId?: string;
}): Promise<RdvInscriptionBookingRow | null> {
  const etabId = await requireEtabId(opts.etablissementId);
  const db = requireDb();
  const now = new Date();
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, etabId),
        eq(rdvInscriptionBooking.googleCalendarId, opts.calendarId),
        eq(rdvInscriptionBooking.googleEventId, opts.eventId),
      ),
    )
    .orderBy(desc(rdvInscriptionBooking.createdAt))
    .limit(5);

  for (const row of rows) {
    if (row.status === "confirmed") return mapBooking(row);
    if (row.status === "pending") {
      if (!row.confirmExpiresAt || row.confirmExpiresAt.getTime() > now.getTime()) {
        return mapBooking(row);
      }
    }
  }
  return null;
}

export async function findBookingByConfirmToken(
  token: string,
): Promise<(RdvInscriptionBookingRow & { etablissementId: string }) | null> {
  const trimmed = token.trim();
  if (!trimmed) return null;
  if (!isDatabaseConfigured()) return null;
  const db = requireDb();
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(eq(rdvInscriptionBooking.confirmToken, trimmed))
    .limit(1);
  const row = rows[0];
  if (!row) return null;
  return { ...mapBooking(row), etablissementId: row.etablissementId };
}

export async function findRdvInscriptionBookingById(opts: {
  bookingId: string;
  etablissementId?: string;
}): Promise<(RdvInscriptionBookingRow & { etablissementId: string }) | null> {
  const bookingId = opts.bookingId.trim();
  if (!bookingId) return null;
  if (!isDatabaseConfigured()) return null;
  const etabId = await requireEtabId(opts.etablissementId);
  const db = requireDb();
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, etabId),
        eq(rdvInscriptionBooking.id, bookingId),
      ),
    )
    .limit(1);
  const row = rows[0];
  if (!row) return null;
  return { ...mapBooking(row), etablissementId: row.etablissementId };
}

export async function markRdvInscriptionBookingConfirmed(opts: {
  bookingId: string;
  etablissementId: string;
  googleHtmlLink?: string | null;
  eleveId?: string | null;
  matchStatus?: "confirmed" | "created" | null;
  reconfirmToken?: string | null;
}): Promise<RdvInscriptionBookingRow | null> {
  const db = requireDb();
  await db
    .update(rdvInscriptionBooking)
    .set({
      status: "confirmed",
      confirmedAt: new Date(),
      confirmToken: null,
      confirmExpiresAt: null,
      googleHtmlLink: opts.googleHtmlLink ?? undefined,
      eleveId: opts.eleveId !== undefined ? opts.eleveId : undefined,
      matchStatus: opts.matchStatus !== undefined ? opts.matchStatus : undefined,
      reconfirmToken: opts.reconfirmToken ?? undefined,
      reconfirmStatus: opts.reconfirmToken ? "pending" : undefined,
      updatedAt: new Date(),
    })
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
      ),
    );
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
      ),
    )
    .limit(1);
  return rows[0] ? mapBooking(rows[0]) : null;
}

export async function findBookingByReconfirmToken(
  token: string,
): Promise<(RdvInscriptionBookingRow & { etablissementId: string; reconfirmToken: string | null }) | null> {
  const trimmed = token.trim();
  if (!trimmed) return null;
  if (!isDatabaseConfigured()) return null;
  const db = requireDb();
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(eq(rdvInscriptionBooking.reconfirmToken, trimmed))
    .limit(1);
  const row = rows[0];
  if (!row) return null;
  return {
    ...mapBooking(row),
    etablissementId: row.etablissementId,
    reconfirmToken: row.reconfirmToken,
  };
}

export async function markRdvInscriptionReconfirm(opts: {
  bookingId: string;
  etablissementId: string;
  status: "ok" | "cancelled";
}): Promise<RdvInscriptionBookingRow | null> {
  const db = requireDb();
  const now = new Date();
  const patch: {
    reconfirmStatus: string;
    reconfirmedAt: Date;
    updatedAt: Date;
    status?: string;
  } = {
    reconfirmStatus: opts.status,
    reconfirmedAt: now,
    updatedAt: now,
  };
  if (opts.status === "cancelled") {
    patch.status = "cancelled";
  }
  await db
    .update(rdvInscriptionBooking)
    .set(patch)
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
      ),
    );
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
      ),
    )
    .limit(1);
  return rows[0] ? mapBooking(rows[0]) : null;
}

export async function markRdvInscriptionReconfirmMailSent(opts: {
  bookingId: string;
  etablissementId: string;
}): Promise<void> {
  const db = requireDb();
  await db
    .update(rdvInscriptionBooking)
    .set({
      reconfirmMailSentAt: new Date(),
      updatedAt: new Date(),
    })
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
      ),
    );
}

/** Bookings confirmés à J-7 (fenêtre 6–8 jours) sans mail de reconfirm encore envoyé. */
export async function listBookingsDueForReconfirmMail(opts?: {
  etablissementId?: string;
  limit?: number;
  now?: Date;
}): Promise<Array<RdvInscriptionBookingRow & { etablissementId: string; reconfirmToken: string }>> {
  const db = requireDb();
  const now = opts?.now ?? new Date();
  const minStart = new Date(now.getTime() + 6 * 24 * 60 * 60 * 1000);
  const maxStart = new Date(now.getTime() + 8 * 24 * 60 * 60 * 1000);
  const limit = Math.min(100, Math.max(1, opts?.limit ?? 50));
  const conditions = [
    eq(rdvInscriptionBooking.status, "confirmed"),
    eq(rdvInscriptionBooking.reconfirmStatus, "pending"),
  ];
  if (opts?.etablissementId) {
    conditions.push(eq(rdvInscriptionBooking.etablissementId, opts.etablissementId));
  }
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(and(...conditions))
    .orderBy(asc(rdvInscriptionBooking.startAt))
    .limit(limit * 3);

  return rows
    .filter((r) => {
      if (r.reconfirmMailSentAt) return false;
      if (!r.reconfirmToken) return false;
      const t = r.startAt.getTime();
      return t >= minStart.getTime() && t <= maxStart.getTime();
    })
    .slice(0, limit)
    .map((r) => ({
      ...mapBooking(r),
      etablissementId: r.etablissementId,
      reconfirmToken: r.reconfirmToken!,
    }));
}

export async function markRdvInscriptionBookingExpired(opts: {
  bookingId: string;
  etablissementId: string;
}): Promise<void> {
  const db = requireDb();
  await db
    .update(rdvInscriptionBooking)
    .set({
      status: "expired",
      confirmToken: null,
      updatedAt: new Date(),
    })
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
        eq(rdvInscriptionBooking.status, "pending"),
      ),
    );
}

export async function listExpiredPendingBookings(opts?: {
  etablissementId?: string;
  limit?: number;
}): Promise<Array<RdvInscriptionBookingRow & { etablissementId: string }>> {
  const db = requireDb();
  const now = new Date();
  const limit = Math.min(100, Math.max(1, opts?.limit ?? 50));
  const conditions = [
    eq(rdvInscriptionBooking.status, "pending"),
  ];
  if (opts?.etablissementId) {
    conditions.push(eq(rdvInscriptionBooking.etablissementId, opts.etablissementId));
  }
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(and(...conditions))
    .orderBy(asc(rdvInscriptionBooking.confirmExpiresAt))
    .limit(limit);

  return rows
    .filter((r) => r.confirmExpiresAt && r.confirmExpiresAt.getTime() <= now.getTime())
    .map((r) => ({ ...mapBooking(r), etablissementId: r.etablissementId }));
}

export async function findBookingByGoogleEvent(opts: {
  calendarId: string;
  eventId: string;
  etablissementId?: string;
}): Promise<RdvInscriptionBookingRow | null> {
  return findActiveBookingByGoogleEvent(opts);
}

export async function listRdvInscriptionBookings(opts?: {
  directionSlug?: string;
  limit?: number;
  etablissementId?: string;
}): Promise<RdvInscriptionBookingRow[]> {
  const etabId = await requireEtabId(opts?.etablissementId);
  const db = requireDb();
  const limit = Math.min(300, Math.max(1, opts?.limit ?? 200));
  const conditions = [eq(rdvInscriptionBooking.etablissementId, etabId)];
  if (opts?.directionSlug) {
    conditions.push(eq(rdvInscriptionBooking.directionSlug, opts.directionSlug.trim().toLowerCase()));
  }
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(and(...conditions))
    // Plus récentes réservations en premier (pas la date du créneau).
    .orderBy(desc(rdvInscriptionBooking.createdAt))
    .limit(limit);
  return rows.map(mapBooking);
}

/** RDV actifs (pending non expiré / confirmed) pour un élève (optionnellement une direction). */
export async function listActiveBookingsForEleve(opts: {
  eleveId: string;
  directionSlug?: string;
  etablissementId?: string;
}): Promise<Array<RdvInscriptionBookingRow & { etablissementId: string }>> {
  const eleveId = opts.eleveId.trim();
  if (!eleveId) return [];
  const etabId = await requireEtabId(opts.etablissementId);
  const db = requireDb();
  const now = new Date();
  const conditions = [
    eq(rdvInscriptionBooking.etablissementId, etabId),
    eq(rdvInscriptionBooking.eleveId, eleveId),
    inArray(rdvInscriptionBooking.status, ["pending", "confirmed"]),
  ];
  if (opts.directionSlug?.trim()) {
    conditions.push(
      eq(rdvInscriptionBooking.directionSlug, opts.directionSlug.trim().toLowerCase()),
    );
  }
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(and(...conditions))
    .orderBy(desc(rdvInscriptionBooking.createdAt));

  return rows
    .filter((r) => {
      if (r.status === "confirmed") return true;
      if (r.status === "pending") {
        if (!r.confirmExpiresAt) return true;
        return r.confirmExpiresAt.getTime() > now.getTime();
      }
      return false;
    })
    .map((r) => ({ ...mapBooking(r), etablissementId: r.etablissementId }));
}

export async function markRdvInscriptionBookingCancelled(opts: {
  bookingId: string;
  etablissementId: string;
}): Promise<RdvInscriptionBookingRow | null> {
  const db = requireDb();
  await db
    .update(rdvInscriptionBooking)
    .set({
      status: "cancelled",
      confirmToken: null,
      confirmExpiresAt: null,
      updatedAt: new Date(),
    })
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
        inArray(rdvInscriptionBooking.status, ["pending", "confirmed"]),
      ),
    );
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
      ),
    )
    .limit(1);
  return rows[0] ? mapBooking(rows[0]) : null;
}

/** Annulation admin avec demande de rechoix (token + note). */
export async function markRdvInscriptionBookingCancelledForReschedule(opts: {
  bookingId: string;
  etablissementId: string;
  adminCancelNote: string | null;
  rescheduleToken: string;
  rescheduleTokenExpiresAt: Date;
}): Promise<RdvInscriptionBookingRow | null> {
  const db = requireDb();
  const note = opts.adminCancelNote?.trim() || null;
  await db
    .update(rdvInscriptionBooking)
    .set({
      status: "cancelled",
      confirmToken: null,
      confirmExpiresAt: null,
      adminCancelNote: note,
      rescheduleToken: opts.rescheduleToken,
      rescheduleTokenExpiresAt: opts.rescheduleTokenExpiresAt,
      updatedAt: new Date(),
    })
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
        inArray(rdvInscriptionBooking.status, ["pending", "confirmed"]),
      ),
    );
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(
      and(
        eq(rdvInscriptionBooking.etablissementId, opts.etablissementId),
        eq(rdvInscriptionBooking.id, opts.bookingId),
      ),
    )
    .limit(1);
  return rows[0] ? mapBooking(rows[0]) : null;
}

export async function findBookingByRescheduleToken(
  token: string,
): Promise<
  | (RdvInscriptionBookingRow & {
      etablissementId: string;
      rescheduleToken: string;
      rescheduleTokenExpiresAt: Date | null;
    })
  | null
> {
  const trimmed = String(token || "").trim();
  if (!trimmed) return null;
  const db = requireDb();
  const rows = await db
    .select()
    .from(rdvInscriptionBooking)
    .where(eq(rdvInscriptionBooking.rescheduleToken, trimmed))
    .limit(1);
  const row = rows[0];
  if (!row?.rescheduleToken) return null;
  return {
    ...mapBooking(row),
    etablissementId: row.etablissementId,
    rescheduleToken: row.rescheduleToken,
    rescheduleTokenExpiresAt: row.rescheduleTokenExpiresAt,
  };
}
