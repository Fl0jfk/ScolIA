import "server-only";

import { and, asc, desc, eq, ne, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import {
  sstCampagne,
  sstConsultation,
  sstEmargement,
  sstFiche,
  type SstFicheStatus,
  type SstFicheWorkflow,
  type SstFicheVisa,
} from "@/db/schema-sst-registre";
import { currentSchoolYearLabel } from "@/app/lib/ent-core-db";
import { listStaffMembersLite } from "@/app/lib/members-db";
import {
  canManageSstRegistre,
  primaryFonctionFromRoles,
} from "@/app/lib/sst-registre/access";
import {
  isSstFicheStatus,
  parseSignaturePngDataUrl,
  previewText,
  type SstCampagneDto,
  type SstConsultationDto,
  type SstEmargementDto,
  type SstFicheDetail,
  type SstFicheListItem,
  type SstMyStatus,
  type SstSuiviPayload,
  type SstSuiviPerson,
} from "@/app/lib/sst-registre/types";

function assertDb() {
  if (!isDatabaseConfigured()) {
    throw new Error("Base de données indisponible.");
  }
  return getDb();
}

function toCampagneDto(row: typeof sstCampagne.$inferSelect): SstCampagneDto {
  return {
    id: row.id,
    anneeLabel: row.anneeLabel,
    title: row.title,
    active: row.active,
  };
}

function toEmargementDto(row: typeof sstEmargement.$inferSelect): SstEmargementDto {
  return {
    id: row.id,
    userId: row.userId,
    firstName: row.firstName,
    lastName: row.lastName,
    fonction: row.fonction,
    signedAt: row.signedAt.toISOString(),
    remarques: row.remarques,
    hasSignature: Boolean(row.signaturePngBase64?.trim()),
  };
}

function toFicheListItem(row: typeof sstFiche.$inferSelect): SstFicheListItem {
  const status = isSstFicheStatus(row.status) ? row.status : "ouverte";
  return {
    id: row.id,
    numero: row.numero,
    status,
    observedDate: row.observedDate,
    lieu: row.lieu,
    observationsPreview: previewText(row.observations),
    declarantName: `${row.declarantFirstName} ${row.declarantLastName}`.trim(),
    createdAt: row.createdAt.toISOString(),
    updatedAt: row.updatedAt.toISOString(),
  };
}

function toFicheDetail(
  row: typeof sstFiche.$inferSelect,
  canAdvance: boolean,
): SstFicheDetail {
  const status = isSstFicheStatus(row.status) ? row.status : "ouverte";
  return {
    id: row.id,
    numero: row.numero,
    status,
    campagneId: row.campagneId,
    createdByUserId: row.createdByUserId,
    declarantFirstName: row.declarantFirstName,
    declarantLastName: row.declarantLastName,
    declarantFonction: row.declarantFonction,
    observedDate: row.observedDate,
    observedTime: row.observedTime,
    lieu: row.lieu,
    observations: row.observations,
    suggestions: row.suggestions,
    hasSignature: Boolean(row.signaturePngBase64?.trim()),
    workflow: row.workflow ?? {},
    createdAt: row.createdAt.toISOString(),
    updatedAt: row.updatedAt.toISOString(),
    canAdvance,
  };
}

function toConsultationDto(row: typeof sstConsultation.$inferSelect): SstConsultationDto {
  return {
    id: row.id,
    userId: row.userId,
    firstName: row.firstName,
    lastName: row.lastName,
    fonction: row.fonction,
    consultedAt: row.consultedAt.toISOString(),
    comments: row.comments,
    hasSignature: Boolean(row.signaturePngBase64?.trim()),
  };
}

/** Crée la campagne de l’année scolaire courante si absente. */
export async function ensureSstCampagne(etablissementId: string): Promise<SstCampagneDto> {
  const db = assertDb();
  const anneeLabel = currentSchoolYearLabel();
  const existing = await db
    .select()
    .from(sstCampagne)
    .where(
      and(eq(sstCampagne.etablissementId, etablissementId), eq(sstCampagne.anneeLabel, anneeLabel)),
    )
    .limit(1);
  if (existing[0]) return toCampagneDto(existing[0]);

  const title = `Registre de sécurité et santé au travail — ${anneeLabel}`;
  const inserted = await db
    .insert(sstCampagne)
    .values({
      etablissementId,
      anneeLabel,
      title,
      active: true,
    })
    .onConflictDoNothing()
    .returning();
  if (inserted[0]) return toCampagneDto(inserted[0]);

  const again = await db
    .select()
    .from(sstCampagne)
    .where(
      and(eq(sstCampagne.etablissementId, etablissementId), eq(sstCampagne.anneeLabel, anneeLabel)),
    )
    .limit(1);
  if (!again[0]) throw new Error("Impossible de créer la campagne SST.");
  return toCampagneDto(again[0]);
}

export async function getSstMyStatus(params: {
  etablissementId: string;
  userId: string;
  roles: string[];
}): Promise<SstMyStatus> {
  const db = assertDb();
  const campagne = await ensureSstCampagne(params.etablissementId);
  const rows = await db
    .select()
    .from(sstEmargement)
    .where(
      and(
        eq(sstEmargement.etablissementId, params.etablissementId),
        eq(sstEmargement.campagneId, campagne.id),
        eq(sstEmargement.userId, params.userId),
      ),
    )
    .limit(1);
  const openRows = await db
    .select({ count: sql<number>`count(*)::int` })
    .from(sstFiche)
    .where(
      and(
        eq(sstFiche.etablissementId, params.etablissementId),
        eq(sstFiche.campagneId, campagne.id),
        ne(sstFiche.status, "cloturee"),
      ),
    );
  return {
    campagne,
    signed: Boolean(rows[0]),
    emargement: rows[0] ? toEmargementDto(rows[0]) : null,
    canManage: canManageSstRegistre(params.roles),
    openFichesCount: openRows[0]?.count ?? 0,
  };
}

/** True si l’utilisateur n’a pas encore émargé pour l’année en cours. */
export async function isSstEmargementPending(params: {
  etablissementId: string;
  userId: string;
}): Promise<boolean> {
  if (!isDatabaseConfigured()) return false;
  try {
    const db = getDb();
    const anneeLabel = currentSchoolYearLabel();
    const campagnes = await db
      .select({ id: sstCampagne.id })
      .from(sstCampagne)
      .where(
        and(
          eq(sstCampagne.etablissementId, params.etablissementId),
          eq(sstCampagne.anneeLabel, anneeLabel),
        ),
      )
      .limit(1);
    // Pas encore de campagne = personne n’a ouvert le module ; on rappelle quand même.
    if (!campagnes[0]) return true;
    const signed = await db
      .select({ id: sstEmargement.id })
      .from(sstEmargement)
      .where(
        and(
          eq(sstEmargement.campagneId, campagnes[0].id),
          eq(sstEmargement.userId, params.userId),
        ),
      )
      .limit(1);
    return !signed[0];
  } catch {
    return false;
  }
}

export async function signSstEmargement(params: {
  etablissementId: string;
  userId: string;
  firstName: string;
  lastName: string;
  roles: string[];
  signatureDataUrl: string;
  remarques?: string;
}): Promise<{ ok: true; emargement: SstEmargementDto } | { ok: false; error: string }> {
  const png = parseSignaturePngDataUrl(params.signatureDataUrl);
  if (!png) return { ok: false, error: "Signature manuscrite invalide (pad requis)." };

  const db = assertDb();
  const campagne = await ensureSstCampagne(params.etablissementId);
  const existing = await db
    .select({ id: sstEmargement.id })
    .from(sstEmargement)
    .where(
      and(eq(sstEmargement.campagneId, campagne.id), eq(sstEmargement.userId, params.userId)),
    )
    .limit(1);
  if (existing[0]) {
    return { ok: false, error: "Vous avez déjà signé pour cette année scolaire." };
  }

  const inserted = await db
    .insert(sstEmargement)
    .values({
      etablissementId: params.etablissementId,
      campagneId: campagne.id,
      userId: params.userId,
      firstName: params.firstName.trim(),
      lastName: params.lastName.trim(),
      fonction: primaryFonctionFromRoles(params.roles),
      signaturePngBase64: png,
      remarques: (params.remarques ?? "").trim().slice(0, 500),
      signedAt: new Date(),
    })
    .returning();
  if (!inserted[0]) return { ok: false, error: "Échec de l’enregistrement." };
  return { ok: true, emargement: toEmargementDto(inserted[0]) };
}

export async function getSstSuivi(params: {
  etablissementId: string;
}): Promise<SstSuiviPayload> {
  const db = assertDb();
  const campagne = await ensureSstCampagne(params.etablissementId);
  const [staff, emargements, fiches, consultations] = await Promise.all([
    listStaffMembersLite(params.etablissementId),
    db
      .select()
      .from(sstEmargement)
      .where(
        and(
          eq(sstEmargement.etablissementId, params.etablissementId),
          eq(sstEmargement.campagneId, campagne.id),
        ),
      ),
    db
      .select()
      .from(sstFiche)
      .where(
        and(
          eq(sstFiche.etablissementId, params.etablissementId),
          eq(sstFiche.campagneId, campagne.id),
        ),
      )
      .orderBy(desc(sstFiche.numero)),
    db
      .select()
      .from(sstConsultation)
      .where(
        and(
          eq(sstConsultation.etablissementId, params.etablissementId),
          eq(sstConsultation.campagneId, campagne.id),
        ),
      )
      .orderBy(desc(sstConsultation.consultedAt)),
  ]);

  const signedByUser = new Map(emargements.map((e) => [e.userId, e]));
  const people: SstSuiviPerson[] = staff
    .map((m) => {
      const userId = String(m.userId || m.externalUserId || "").trim();
      if (!userId) return null;
      const hit = signedByUser.get(userId);
      return {
        userId,
        firstName: m.firstName ?? "",
        lastName: m.lastName ?? "",
        email: m.email,
        fonction: primaryFonctionFromRoles(m.roles),
        signed: Boolean(hit),
        signedAt: hit ? hit.signedAt.toISOString() : null,
      } satisfies SstSuiviPerson;
    })
    .filter((p): p is SstSuiviPerson => p !== null)
    .sort((a, b) => {
      if (a.signed !== b.signed) return a.signed ? 1 : -1;
      return `${a.lastName} ${a.firstName}`.localeCompare(`${b.lastName} ${b.firstName}`, "fr");
    });

  const signedCount = people.filter((p) => p.signed).length;
  return {
    campagne,
    total: people.length,
    signedCount,
    pendingCount: people.length - signedCount,
    people,
    fiches: fiches.map(toFicheListItem),
    consultations: consultations.map(toConsultationDto),
  };
}

export async function listSstFiches(params: {
  etablissementId: string;
  campagneId?: string;
}): Promise<SstFicheListItem[]> {
  const db = assertDb();
  const campagne = params.campagneId
    ? { id: params.campagneId }
    : await ensureSstCampagne(params.etablissementId);
  const rows = await db
    .select()
    .from(sstFiche)
    .where(
      and(
        eq(sstFiche.etablissementId, params.etablissementId),
        eq(sstFiche.campagneId, campagne.id),
      ),
    )
    .orderBy(desc(sstFiche.numero));
  return rows.map(toFicheListItem);
}

export async function getSstFiche(params: {
  etablissementId: string;
  ficheId: string;
  roles: string[];
}): Promise<SstFicheDetail | null> {
  const db = assertDb();
  const rows = await db
    .select()
    .from(sstFiche)
    .where(
      and(eq(sstFiche.etablissementId, params.etablissementId), eq(sstFiche.id, params.ficheId)),
    )
    .limit(1);
  if (!rows[0]) return null;
  return toFicheDetail(rows[0], canManageSstRegistre(params.roles));
}

export async function createSstFiche(params: {
  etablissementId: string;
  userId: string;
  firstName: string;
  lastName: string;
  roles: string[];
  observedDate: string;
  observedTime?: string;
  lieu?: string;
  observations: string;
  suggestions?: string;
  signatureDataUrl?: string;
}): Promise<{ ok: true; fiche: SstFicheDetail } | { ok: false; error: string }> {
  const observations = params.observations.trim();
  if (observations.length < 5) {
    return { ok: false, error: "Décrivez l’observation (au moins quelques mots)." };
  }
  if (!/^\d{4}-\d{2}-\d{2}$/.test(params.observedDate)) {
    return { ok: false, error: "Date d’observation invalide." };
  }
  let signature: string | null = null;
  if (params.signatureDataUrl?.trim()) {
    signature = parseSignaturePngDataUrl(params.signatureDataUrl);
    if (!signature) return { ok: false, error: "Signature manuscrite invalide." };
  }

  const db = assertDb();
  const campagne = await ensureSstCampagne(params.etablissementId);
  const maxRows = await db
    .select({ max: sql<number>`coalesce(max(${sstFiche.numero}), 0)::int` })
    .from(sstFiche)
    .where(eq(sstFiche.etablissementId, params.etablissementId));
  const numero = (maxRows[0]?.max ?? 0) + 1;

  const inserted = await db
    .insert(sstFiche)
    .values({
      etablissementId: params.etablissementId,
      campagneId: campagne.id,
      numero,
      createdByUserId: params.userId,
      declarantFirstName: params.firstName.trim(),
      declarantLastName: params.lastName.trim(),
      declarantFonction: primaryFonctionFromRoles(params.roles),
      observedDate: params.observedDate,
      observedTime: (params.observedTime ?? "").trim().slice(0, 8),
      lieu: (params.lieu ?? "").trim().slice(0, 200),
      observations: observations.slice(0, 8000),
      suggestions: (params.suggestions ?? "").trim().slice(0, 4000),
      signaturePngBase64: signature,
      status: "ouverte",
      workflow: {},
    })
    .returning();
  if (!inserted[0]) return { ok: false, error: "Échec de la création de la fiche." };
  return {
    ok: true,
    fiche: toFicheDetail(inserted[0], canManageSstRegistre(params.roles)),
  };
}

const NEXT_STATUS: Partial<Record<SstFicheStatus, SstFicheStatus>> = {
  ouverte: "visa_responsable",
  visa_responsable: "examen_cse",
  examen_cse: "decision",
  decision: "realisation",
  realisation: "cloturee",
};

const STATUS_WORKFLOW_KEY: Partial<
  Record<SstFicheStatus, keyof Pick<SstFicheWorkflow, "responsable" | "cse" | "decision" | "realisation">>
> = {
  ouverte: "responsable",
  visa_responsable: "cse",
  examen_cse: "decision",
  decision: "realisation",
};

export async function advanceSstFiche(params: {
  etablissementId: string;
  ficheId: string;
  userId: string;
  firstName: string;
  lastName: string;
  roles: string[];
  notes: string;
  signatureDataUrl?: string;
}): Promise<{ ok: true; fiche: SstFicheDetail } | { ok: false; error: string }> {
  if (!canManageSstRegistre(params.roles)) {
    return { ok: false, error: "Accès réservé au pilotage RH / direction." };
  }
  const db = assertDb();
  const rows = await db
    .select()
    .from(sstFiche)
    .where(
      and(eq(sstFiche.etablissementId, params.etablissementId), eq(sstFiche.id, params.ficheId)),
    )
    .limit(1);
  const row = rows[0];
  if (!row) return { ok: false, error: "Fiche introuvable." };
  const status = isSstFicheStatus(row.status) ? row.status : "ouverte";
  if (status === "cloturee") return { ok: false, error: "Fiche déjà clôturée." };

  const next = NEXT_STATUS[status];
  if (!next) return { ok: false, error: "Transition impossible." };

  let signature: string | null = null;
  if (params.signatureDataUrl?.trim()) {
    signature = parseSignaturePngDataUrl(params.signatureDataUrl);
    if (!signature) return { ok: false, error: "Signature manuscrite invalide." };
  }

  const visa: SstFicheVisa = {
    notes: params.notes.trim().slice(0, 4000),
    userId: params.userId,
    name: `${params.firstName} ${params.lastName}`.trim() || "Responsable",
    at: new Date().toISOString(),
    signaturePngBase64: signature,
  };

  const workflow: SstFicheWorkflow = { ...(row.workflow ?? {}) };
  const key = STATUS_WORKFLOW_KEY[status];
  if (key) workflow[key] = visa;
  if (next === "cloturee") {
    workflow.closedAt = new Date().toISOString();
  }

  const updated = await db
    .update(sstFiche)
    .set({
      status: next,
      workflow,
      updatedAt: new Date(),
    })
    .where(
      and(eq(sstFiche.etablissementId, params.etablissementId), eq(sstFiche.id, params.ficheId)),
    )
    .returning();
  if (!updated[0]) return { ok: false, error: "Mise à jour impossible." };
  return { ok: true, fiche: toFicheDetail(updated[0], true) };
}

export async function addSstConsultation(params: {
  etablissementId: string;
  userId: string;
  firstName: string;
  lastName: string;
  roles: string[];
  comments?: string;
  signatureDataUrl?: string;
}): Promise<{ ok: true; consultation: SstConsultationDto } | { ok: false; error: string }> {
  if (!canManageSstRegistre(params.roles)) {
    return { ok: false, error: "Accès réservé au pilotage RH / direction." };
  }
  let signature: string | null = null;
  if (params.signatureDataUrl?.trim()) {
    signature = parseSignaturePngDataUrl(params.signatureDataUrl);
    if (!signature) return { ok: false, error: "Signature manuscrite invalide." };
  }
  const db = assertDb();
  const campagne = await ensureSstCampagne(params.etablissementId);
  const inserted = await db
    .insert(sstConsultation)
    .values({
      etablissementId: params.etablissementId,
      campagneId: campagne.id,
      userId: params.userId,
      firstName: params.firstName.trim(),
      lastName: params.lastName.trim(),
      fonction: primaryFonctionFromRoles(params.roles),
      comments: (params.comments ?? "").trim().slice(0, 2000),
      signaturePngBase64: signature,
      consultedAt: new Date(),
    })
    .returning();
  if (!inserted[0]) return { ok: false, error: "Échec de l’enregistrement." };
  return { ok: true, consultation: toConsultationDto(inserted[0]) };
}

export async function listSstConsultations(etablissementId: string): Promise<SstConsultationDto[]> {
  const db = assertDb();
  const campagne = await ensureSstCampagne(etablissementId);
  const rows = await db
    .select()
    .from(sstConsultation)
    .where(
      and(
        eq(sstConsultation.etablissementId, etablissementId),
        eq(sstConsultation.campagneId, campagne.id),
      ),
    )
    .orderBy(desc(sstConsultation.consultedAt));
  return rows.map(toConsultationDto);
}

export async function listPendingSstUserIds(etablissementId: string): Promise<string[]> {
  const suivi = await getSstSuivi({ etablissementId });
  return suivi.people.filter((p) => !p.signed).map((p) => p.userId);
}

/** Pour tests / debug : liste des émargements d’une campagne. */
export async function listSstEmargements(
  etablissementId: string,
  campagneId: string,
): Promise<SstEmargementDto[]> {
  const db = assertDb();
  const rows = await db
    .select()
    .from(sstEmargement)
    .where(
      and(
        eq(sstEmargement.etablissementId, etablissementId),
        eq(sstEmargement.campagneId, campagneId),
      ),
    )
    .orderBy(asc(sstEmargement.lastName), asc(sstEmargement.firstName));
  return rows.map(toEmargementDto);
}
