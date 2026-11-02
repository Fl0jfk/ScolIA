import "server-only";

import { and, asc, desc, eq, ne, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import {
  sstCampagne,
  sstConsultation,
  sstEmargement,
  sstFiche,
  sstProxyDeposant,
  type SstFicheStatus,
  type SstFicheWorkflow,
  type SstFicheVisa,
} from "@/db/schema-sst-registre";
import { currentSchoolYearLabel } from "@/app/lib/ent-core-db";
import { listStaffMembersLite } from "@/app/lib/members-db";
import {
  canDepositSstForOthers,
  canManageSstRegistre,
  primaryFonctionFromRoles,
} from "@/app/lib/sst-registre/access";
import {
  isSstAttestationOnlySignature,
  SST_ATTESTATION_ONLY_DATA_URL,
  SST_MARKER_AS_DRAWN_SIGNATURE_ERROR,
} from "@/app/lib/sst-registre/emarger-body";
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
  type SstProxyPerson,
  type SstStaffOption,
  type SstSuiviPayload,
  type SstSuiviPerson,
} from "@/app/lib/sst-registre/types";

function displayName(first: string, last: string): string {
  return `${first} ${last}`.trim();
}

function isDepositedForOther(row: typeof sstFiche.$inferSelect): boolean {
  const declarantId = (row.declarantUserId || row.createdByUserId).trim();
  if (declarantId && declarantId !== row.createdByUserId.trim()) return true;
  const byName = displayName(row.createdByFirstName, row.createdByLastName);
  const declarantName = displayName(row.declarantFirstName, row.declarantLastName);
  if (byName && declarantName && byName.localeCompare(declarantName, "fr", { sensitivity: "accent" }) !== 0) {
    return true;
  }
  return false;
}

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
  const createdByName =
    displayName(row.createdByFirstName, row.createdByLastName) ||
    displayName(row.declarantFirstName, row.declarantLastName);
  return {
    id: row.id,
    numero: row.numero,
    status,
    observedDate: row.observedDate,
    lieu: row.lieu,
    observationsPreview: previewText(row.observations),
    declarantName: displayName(row.declarantFirstName, row.declarantLastName),
    createdByName,
    depositedForOther: isDepositedForOther(row),
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
    createdByFirstName: row.createdByFirstName || row.declarantFirstName,
    createdByLastName: row.createdByLastName || row.declarantLastName,
    declarantUserId: row.declarantUserId || row.createdByUserId,
    declarantFirstName: row.declarantFirstName,
    declarantLastName: row.declarantLastName,
    declarantFonction: row.declarantFonction,
    depositedForOther: isDepositedForOther(row),
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

export async function listSstProxyUserIds(etablissementId: string): Promise<string[]> {
  const db = assertDb();
  const rows = await db
    .select({ userId: sstProxyDeposant.userId })
    .from(sstProxyDeposant)
    .where(eq(sstProxyDeposant.etablissementId, etablissementId));
  return rows.map((r) => r.userId);
}

export async function listSstProxyPeople(etablissementId: string): Promise<SstProxyPerson[]> {
  const db = assertDb();
  const rows = await db
    .select()
    .from(sstProxyDeposant)
    .where(eq(sstProxyDeposant.etablissementId, etablissementId))
    .orderBy(asc(sstProxyDeposant.lastName), asc(sstProxyDeposant.firstName));
  return rows.map((r) => ({
    userId: r.userId,
    firstName: r.firstName,
    lastName: r.lastName,
    email: r.email,
  }));
}

export async function setSstProxyPeople(params: {
  etablissementId: string;
  people: SstProxyPerson[];
}): Promise<SstProxyPerson[]> {
  const db = assertDb();
  const cleaned = new Map<string, SstProxyPerson>();
  for (const p of params.people) {
    const userId = p.userId.trim();
    if (!userId) continue;
    cleaned.set(userId, {
      userId,
      firstName: p.firstName.trim().slice(0, 120),
      lastName: p.lastName.trim().slice(0, 120),
      email: p.email.trim().slice(0, 200),
    });
  }

  const existing = await db
    .select({ id: sstProxyDeposant.id, userId: sstProxyDeposant.userId })
    .from(sstProxyDeposant)
    .where(eq(sstProxyDeposant.etablissementId, params.etablissementId));

  const nextIds = new Set(cleaned.keys());
  for (const row of existing) {
    if (!nextIds.has(row.userId)) {
      await db
        .delete(sstProxyDeposant)
        .where(
          and(
            eq(sstProxyDeposant.etablissementId, params.etablissementId),
            eq(sstProxyDeposant.id, row.id),
          ),
        );
    }
  }

  for (const person of cleaned.values()) {
    await db
      .insert(sstProxyDeposant)
      .values({
        etablissementId: params.etablissementId,
        userId: person.userId,
        firstName: person.firstName,
        lastName: person.lastName,
        email: person.email,
      })
      .onConflictDoUpdate({
        target: [sstProxyDeposant.etablissementId, sstProxyDeposant.userId],
        set: {
          firstName: person.firstName,
          lastName: person.lastName,
          email: person.email,
        },
      });
  }

  return listSstProxyPeople(params.etablissementId);
}

export async function listSstStaffOptions(etablissementId: string): Promise<SstStaffOption[]> {
  const staff = await listStaffMembersLite(etablissementId);
  return staff
    .map((m) => {
      const userId = String(m.userId || m.externalUserId || "").trim();
      if (!userId) return null;
      return {
        userId,
        firstName: m.firstName ?? "",
        lastName: m.lastName ?? "",
        email: m.email,
        fonction: primaryFonctionFromRoles(m.roles),
      } satisfies SstStaffOption;
    })
    .filter((p): p is SstStaffOption => p !== null)
    .sort((a, b) =>
      `${a.lastName} ${a.firstName}`.localeCompare(`${b.lastName} ${b.firstName}`, "fr"),
    );
}

export async function getSstMyStatus(params: {
  etablissementId: string;
  userId: string;
  roles: string[];
}): Promise<SstMyStatus> {
  const db = assertDb();
  const campagne = await ensureSstCampagne(params.etablissementId);
  const [rows, openRows, proxyIds] = await Promise.all([
    db
      .select()
      .from(sstEmargement)
      .where(
        and(
          eq(sstEmargement.etablissementId, params.etablissementId),
          eq(sstEmargement.campagneId, campagne.id),
          eq(sstEmargement.userId, params.userId),
        ),
      )
      .limit(1),
    db
      .select({ count: sql<number>`count(*)::int` })
      .from(sstFiche)
      .where(
        and(
          eq(sstFiche.etablissementId, params.etablissementId),
          eq(sstFiche.campagneId, campagne.id),
          ne(sstFiche.status, "cloturee"),
        ),
      ),
    listSstProxyUserIds(params.etablissementId),
  ]);
  return {
    campagne,
    signed: Boolean(rows[0]),
    emargement: rows[0] ? toEmargementDto(rows[0]) : null,
    canManage: canManageSstRegistre(params.roles),
    canDepositForOthers: canDepositSstForOthers({
      roles: params.roles,
      userId: params.userId,
      proxyUserIds: proxyIds,
    }),
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
  attestationOnly?: boolean;
  remarques?: string;
}): Promise<{ ok: true; emargement: SstEmargementDto } | { ok: false; error: string }> {
  let png: string | null = null;
  if (params.attestationOnly) {
    if (params.signatureDataUrl.trim() !== SST_ATTESTATION_ONLY_DATA_URL) {
      return { ok: false, error: "Attestation invalide." };
    }
    png = SST_ATTESTATION_ONLY_DATA_URL;
  } else {
    if (isSstAttestationOnlySignature(params.signatureDataUrl)) {
      return { ok: false, error: SST_MARKER_AS_DRAWN_SIGNATURE_ERROR };
    }
    png = parseSignaturePngDataUrl(params.signatureDataUrl);
    if (!png) return { ok: false, error: "Signature manuscrite invalide (pad requis)." };
  }

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
      remarques: params.attestationOnly
        ? `[Attestation sans signature manuscrite] ${(params.remarques ?? "").trim()}`.trim().slice(0, 500)
        : (params.remarques ?? "").trim().slice(0, 500),
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
  /** Si renseigné : fiche déposée au nom d’un autre collaborateur. */
  onBehalfOfUserId?: string;
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

  const onBehalfId = (params.onBehalfOfUserId ?? "").trim();
  let declarantUserId = params.userId;
  let declarantFirstName = params.firstName.trim();
  let declarantLastName = params.lastName.trim();
  let declarantFonction = primaryFonctionFromRoles(params.roles);

  if (onBehalfId && onBehalfId !== params.userId) {
    const proxyIds = await listSstProxyUserIds(params.etablissementId);
    if (
      !canDepositSstForOthers({
        roles: params.roles,
        userId: params.userId,
        proxyUserIds: proxyIds,
      })
    ) {
      return {
        ok: false,
        error: "Vous n’êtes pas autorisé(e) à déposer une fiche pour une autre personne.",
      };
    }
    const staff = await listStaffMembersLite(params.etablissementId);
    const subject = staff.find((m) => {
      const uid = String(m.userId || "").trim();
      const ext = String(m.externalUserId || "").trim();
      return uid === onBehalfId || ext === onBehalfId;
    });
    if (!subject || subject.pending) {
      return { ok: false, error: "Collaborateur introuvable pour le dépôt pour autrui." };
    }
    declarantUserId = String(subject.userId || subject.externalUserId || onBehalfId).trim();
    declarantFirstName = (subject.firstName ?? "").trim();
    declarantLastName = (subject.lastName ?? "").trim();
    declarantFonction = primaryFonctionFromRoles(subject.roles);
    if (!declarantFirstName && !declarantLastName) {
      declarantLastName = subject.displayName?.trim() || subject.email;
    }
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
      createdByFirstName: params.firstName.trim(),
      createdByLastName: params.lastName.trim(),
      declarantUserId,
      declarantFirstName,
      declarantLastName,
      declarantFonction,
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
