import "server-only";

import { and, asc, desc, eq, inArray, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  fdCampagne,
  fdEtape,
  fdFiche,
  fdPdfArchive,
  fdReponse,
  fdSignature,
  fdToken,
  type FdAcceptationPayload,
  type FdAppelConfig,
  type FdAppelPayload,
  type FdCalendrierMode,
  type FdCampagneRow,
  type FdCatalogueChoix,
  type FdConseilDecisionPayload,
  type FdEtapeKind,
  type FdEtapeRow,
  type FdFicheRow,
  type FdFicheStatut,
  type FdParentAccordPayload,
  type FdReponsePayload,
  type FdReponseRow,
} from "@/db/schema";
import {
  collectEleveParentEmails,
  isValidParentEmail,
  normalizeParentEmail,
} from "@/app/lib/eleves-parent-emails";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { drizzleEleveActifPourListes } from "@/app/lib/eleve-actif-scope";
import { fileFicheDialoguePdfToDossier } from "@/app/lib/fiches-dialogue-filing";
import {
  notifyFdAcceptationRequest,
  notifyFdAppelProcedure,
  notifyFdAvenantConseil,
  notifyFdConflictParents,
  notifyFdConflictStaff,
  notifyFdDecisionPdf,
  notifyFdFamilleSaisie,
  notifyFdParent2Accord,
  notifyFdRefusStaff,
  notifyFdTacitAccord,
} from "@/app/lib/fiches-dialogue-notify";
import {
  resolveStagesAdminEmails,
  resolveStagesDirectionEmail,
} from "@/app/lib/stage-config";
import {
  findPrincipalAssignments,
  getStageReferentsConfig,
} from "@/app/lib/stage-referents-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";
import type { FdStarterMode } from "@/db/schema-fiches-dialogue";
import {
  buildFicheDialoguePdf,
  identityFromFiche,
  sectionsFromAcceptation,
  sectionsFromConseil,
  sectionsFromFamilleReponse,
  type FdPdfSection,
} from "@/app/lib/fiches-dialogue-pdf";
import { getFdTemplate, presetForNiveau, isFdNiveau, type FdNiveau } from "@/app/lib/fiches-dialogue-templates";
import { niveauFromClasse } from "@/app/lib/internat-level";
import {
  createFdAccessToken,
  revokeFdTokensForFiche,
} from "@/app/lib/fiches-dialogue-tokens";

function now() {
  return new Date();
}

/**
 * Destinataires staff d’une fiche : PP, direction et secrétariat de stages.
 * Best-effort : une configuration incomplète ne doit jamais bloquer le workflow.
 */
async function resolveFdStaffEmails(fiche: FdFicheRow): Promise<string[]> {
  const staffTo = new Set<string>();
  try {
    const year = currentStageSchoolYear();
    const refs = await getStageReferentsConfig(year);
    for (const a of findPrincipalAssignments(refs, fiche.classeActuelle)) {
      if (a.email?.trim()) staffTo.add(a.email.trim().toLowerCase());
    }
    const directionEmail = await resolveStagesDirectionEmail(
      fiche.classeActuelle,
      fiche.classeActuelle,
    );
    if (directionEmail) staffTo.add(directionEmail.trim().toLowerCase());
    for (const e of await resolveStagesAdminEmails(fiche.classeActuelle, fiche.classeActuelle)) {
      staffTo.add(e.trim().toLowerCase());
    }
  } catch {
    /* best-effort */
  }
  return [...staffTo];
}

export async function listFdCampagnes(etablissementId: string): Promise<FdCampagneRow[]> {
  const db = getDb();
  return db
    .select()
    .from(fdCampagne)
    .where(eq(fdCampagne.etablissementId, etablissementId))
    .orderBy(asc(fdCampagne.createdAt));
}

export async function getFdCampagne(
  etablissementId: string,
  campagneId: string,
): Promise<FdCampagneRow | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(fdCampagne)
    .where(and(eq(fdCampagne.etablissementId, etablissementId), eq(fdCampagne.id, campagneId)))
    .limit(1);
  return row ?? null;
}

export async function listFdEtapes(
  etablissementId: string,
  campagneId: string,
): Promise<FdEtapeRow[]> {
  const db = getDb();
  return db
    .select()
    .from(fdEtape)
    .where(and(eq(fdEtape.etablissementId, etablissementId), eq(fdEtape.campagneId, campagneId)))
    .orderBy(asc(fdEtape.ordre));
}

export async function createFdCampagneFromTemplate(params: {
  etablissementId: string;
  /** @deprecated préférer niveauActuel + starterMode */
  templateKey?: string;
  niveauActuel?: string;
  label: string;
  anneeLabel: string;
  anneeScolaireId?: string | null;
  siteKey?: string | null;
  classesCibles?: string[];
  eleveIdsCibles?: string[];
  delaiFamilleJours?: number;
  appelConfig?: FdAppelConfig;
  catalogueOverride?: FdCatalogueChoix;
  starterMode?: FdStarterMode;
  contactPpLabel?: string | null;
  createdByUserId?: string | null;
  etapesDates?: Array<{
    ordre: number;
    opensAt?: string | Date | null;
    closesAt?: string | Date | null;
    conseilDate?: string | null;
  }>;
}): Promise<{ campagne: FdCampagneRow; etapes: FdEtapeRow[] }> {
  const niveau =
    params.niveauActuel && isFdNiveau(params.niveauActuel)
      ? (params.niveauActuel as FdNiveau)
      : null;
  /** La Terminale commence toujours par la famille — on normalise pour ne pas promettre un « conseil d’abord » inopérant. */
  const starterMode: FdStarterMode =
    niveau === "Tle" ? "famille_dabord" : (params.starterMode ?? "conseil_dabord");

  const preset = niveau ? presetForNiveau(niveau, starterMode, params.templateKey) : null;
  const template = getFdTemplate(params.templateKey || preset?.templateKey || "");
  if (!preset && !template) throw new Error("TEMPLATE_UNKNOWN");

  const calendrierMode = (preset?.calendrierMode ??
    template!.calendrierMode) as FdCalendrierMode;
  const catalogue = params.catalogueOverride ?? preset?.catalogue ?? template!.catalogue;
  const etapeDefs = preset?.etapes ?? template!.etapes;
  const templateKey = preset?.templateKey ?? template!.key;

  const db = getDb();
  const [campagne] = await db
    .insert(fdCampagne)
    .values({
      etablissementId: params.etablissementId,
      anneeScolaireId: params.anneeScolaireId ?? null,
      label: params.label.trim(),
      anneeLabel: params.anneeLabel.trim(),
      siteKey: params.siteKey ?? null,
      calendrierMode,
      templateKey,
      starterMode,
      niveauActuel: niveau ?? params.niveauActuel ?? null,
      statut: "brouillon",
      catalogue,
      appelConfig: params.appelConfig
        ? { ...params.appelConfig, enabled: true }
        : { enabled: true },
      contactPpLabel: params.contactPpLabel ?? null,
      delaiFamilleJours: params.delaiFamilleJours ?? 7,
      classesCibles: params.classesCibles ?? [],
      eleveIdsCibles: params.eleveIdsCibles ?? [],
      createdByUserId: params.createdByUserId ?? null,
    })
    .returning();

  const etapes: FdEtapeRow[] = [];
  for (let i = 0; i < etapeDefs.length; i++) {
    const def = etapeDefs[i];
    const datePatch = params.etapesDates?.find((d) => d.ordre === i + 1);
    const [etape] = await db
      .insert(fdEtape)
      .values({
        etablissementId: params.etablissementId,
        campagneId: campagne.id,
        ordre: i + 1,
        kind: def.kind,
        label: def.label,
        description: def.description ?? null,
        optionnelle: Boolean(def.optionnelle),
        opensAt: datePatch?.opensAt
          ? datePatch.opensAt instanceof Date
            ? datePatch.opensAt
            : new Date(datePatch.opensAt)
          : null,
        closesAt: datePatch?.closesAt
          ? datePatch.closesAt instanceof Date
            ? datePatch.closesAt
            : new Date(datePatch.closesAt)
          : null,
        conseilDate: datePatch?.conseilDate || null,
      })
      .returning();
    etapes.push(etape);
  }

  return { campagne, etapes };
}

export async function deleteFdCampagne(
  etablissementId: string,
  campagneId: string,
): Promise<{ ok: true }> {
  const db = getDb();
  const deleted = await db
    .delete(fdCampagne)
    .where(and(eq(fdCampagne.etablissementId, etablissementId), eq(fdCampagne.id, campagneId)))
    .returning({ id: fdCampagne.id });
  if (!deleted.length) throw new Error("CAMPAGNE_NOT_FOUND");
  return { ok: true };
}

export async function updateFdCampagne(
  etablissementId: string,
  campagneId: string,
  patch: Partial<{
    label: string;
    statut: FdCampagneRow["statut"];
    catalogue: FdCatalogueChoix;
    appelConfig: FdAppelConfig;
    delaiFamilleJours: number;
    classesCibles: string[];
    eleveIdsCibles: string[];
    niveauActuel: string | null;
    calendrierMode: FdCalendrierMode;
    starterMode: FdStarterMode;
    contactPpLabel: string | null;
  }>,
): Promise<FdCampagneRow> {
  const db = getDb();
  const safePatch: Partial<typeof fdCampagne.$inferInsert> = { ...patch, updatedAt: now() };
  if (patch.statut === "active") {
    safePatch.openedAt = now();
    safePatch.closedAt = null;
  } else if (patch.statut === "cloturee" || patch.statut === "archivee") {
    safePatch.closedAt = now();
  } else if (patch.statut === "brouillon") {
    safePatch.closedAt = null;
  }
  if (safePatch.appelConfig) {
    safePatch.appelConfig = { ...safePatch.appelConfig, enabled: true };
  }
  const [row] = await db
    .update(fdCampagne)
    .set(safePatch)
    .where(and(eq(fdCampagne.etablissementId, etablissementId), eq(fdCampagne.id, campagneId)))
    .returning();
  if (!row) throw new Error("CAMPAGNE_NOT_FOUND");
  return row;
}

export async function updateFdEtapeDates(
  etablissementId: string,
  etapeId: string,
  patch: {
    opensAt?: Date | null;
    closesAt?: Date | null;
    conseilDate?: string | null;
    label?: string;
    description?: string | null;
  },
): Promise<FdEtapeRow> {
  const db = getDb();
  const [row] = await db
    .update(fdEtape)
    .set({ ...patch, updatedAt: now() })
    .where(and(eq(fdEtape.etablissementId, etablissementId), eq(fdEtape.id, etapeId)))
    .returning();
  if (!row) throw new Error("ETAPE_NOT_FOUND");
  return row;
}

function matchClasse(classe: string, cibles: string[]): boolean {
  if (!cibles.length) return true;
  const c = classe.trim().toLowerCase();
  return cibles.some((t) => {
    const target = t.trim().toLowerCase();
    if (!target) return false;
    return c === target || c.startsWith(target);
  });
}

export async function generateFdFichesForCampagne(
  etablissementId: string,
  campagneId: string,
): Promise<{
  created: number;
  skipped: number;
  errors: Array<{ eleve?: string; reason: string }>;
}> {
  const campagne = await getFdCampagne(etablissementId, campagneId);
  if (!campagne) throw new Error("CAMPAGNE_NOT_FOUND");
  const etapes = await listFdEtapes(etablissementId, campagneId);
  const firstEtape = etapes.find((e) => !e.optionnelle) ?? etapes[0];
  if (!firstEtape) throw new Error("NO_ETAPES");

  const registry = await loadElevesActifsRegistry();
  const db = getDb();
  const elevesDb = await db
    .select()
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), drizzleEleveActifPourListes()!));

  const byIne = new Map(elevesDb.map((e) => [e.ine?.toUpperCase() ?? "", e]));
  const byId = new Map(elevesDb.map((e) => [e.id, e]));

  const cibleIds = new Set((campagne.eleveIdsCibles ?? []).filter(Boolean));
  const hasEleveCibles = cibleIds.size > 0;
  const niveauCampagne = campagne.niveauActuel?.trim() || null;

  let created = 0;
  let skipped = 0;
  const errors: Array<{ eleve?: string; reason: string }> = [];

  for (const reg of registry) {
    const classe = String(reg.classe || "");
    const row =
      (reg.id && byId.get(reg.id)) ||
      (reg.ine ? byIne.get(reg.ine.toUpperCase()) : undefined);
    if (!row) {
      skipped += 1;
      continue;
    }

    if (hasEleveCibles) {
      if (!cibleIds.has(row.id)) {
        skipped += 1;
        continue;
      }
    } else {
      if (niveauCampagne) {
        const niv = niveauFromClasse(classe || row.classe);
        if (niv !== niveauCampagne) {
          skipped += 1;
          continue;
        }
      }
      if (!matchClasse(classe || row.classe || "", campagne.classesCibles ?? [])) {
        skipped += 1;
        continue;
      }
    }

    const emails = collectEleveParentEmails(reg).filter(isValidParentEmail);
    const optionsActuelles = [
      ...new Set([
        ...(reg.lv1 ? [`LV1 ${reg.lv1}`] : row.lv1 ? [`LV1 ${row.lv1}`] : []),
        ...(reg.lv2 ? [`LV2 ${reg.lv2}`] : row.lv2 ? [`LV2 ${row.lv2}`] : []),
        ...(reg.options ?? []),
        ...((row.options as string[] | null) ?? []),
      ]),
    ];
    try {
      await db.insert(fdFiche).values({
        etablissementId,
        campagneId,
        eleveId: row.id,
        eleveNom: row.nom || reg.nom,
        elevePrenom: row.prenom || reg.prenom,
        classeActuelle: classe || row.classe || "",
        eleveDateNaissance: reg.dateNaissance || row.dateNaissance || null,
        eleveIne: row.ine || reg.ine || null,
        eleveMef: row.mef || null,
        elevePhotoKey: reg.photoKey || row.photoKey || null,
        optionsActuelles,
        parentEmails: emails,
        statut: "a_envoyer",
        etapeCouranteId: firstEtape.id,
      });
      created += 1;
    } catch (err) {
      skipped += 1;
      errors.push({
        eleve: `${row.prenom || reg.prenom} ${row.nom || reg.nom}`.trim(),
        reason: err instanceof Error ? err.message : "échec insertion fiche",
      });
    }
  }

  return { created, skipped, errors };
}

export async function listFdFiches(
  etablissementId: string,
  campagneId: string,
  opts?: { classe?: string; statut?: string },
): Promise<FdFicheRow[]> {
  const db = getDb();
  const rows = await db
    .select()
    .from(fdFiche)
    .where(and(eq(fdFiche.etablissementId, etablissementId), eq(fdFiche.campagneId, campagneId)))
    .orderBy(asc(fdFiche.classeActuelle), asc(fdFiche.eleveNom));
  return rows.filter((r) => {
    if (opts?.classe && r.classeActuelle !== opts.classe) return false;
    if (opts?.statut && r.statut !== opts.statut) return false;
    return true;
  });
}

export async function getFdFiche(
  etablissementId: string,
  ficheId: string,
): Promise<FdFicheRow | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(fdFiche)
    .where(and(eq(fdFiche.etablissementId, etablissementId), eq(fdFiche.id, ficheId)))
    .limit(1);
  return row ?? null;
}

async function getEtape(etablissementId: string, etapeId: string): Promise<FdEtapeRow | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(fdEtape)
    .where(and(eq(fdEtape.etablissementId, etablissementId), eq(fdEtape.id, etapeId)))
    .limit(1);
  return row ?? null;
}

function isFamilleEtape(kind: FdEtapeKind): boolean {
  return kind === "saisie_famille" || kind === "choix_definitifs" || kind === "acceptation_famille";
}

function isConseilEtape(kind: FdEtapeKind): boolean {
  return kind === "conseil" || kind === "decision_finale_conseil";
}

/** true si la famille peut encore modifier (date limite non dépassée, pas gelée admin). */
export function isFdEtapeOpenForFamille(etape: FdEtapeRow, at = new Date()): boolean {
  if (etape.gelee) return false;
  if (etape.opensAt && etape.opensAt.getTime() > at.getTime()) return false;
  if (etape.closesAt && etape.closesAt.getTime() < at.getTime()) return false;
  return true;
}

export async function sendFdFicheToFamille(params: {
  etablissementId: string;
  ficheId: string;
  reminder?: boolean;
}): Promise<{ ok: true; sent: boolean; reason?: string } | { ok: false; error: string }> {
  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  if (!fiche.etapeCouranteId) return { ok: false, error: "Aucune étape courante." };

  const etape = await getEtape(params.etablissementId, fiche.etapeCouranteId);
  if (!etape) return { ok: false, error: "Étape introuvable." };
  if (etape.gelee) return { ok: false, error: "Cette étape est figée." };
  if (!isFamilleEtape(etape.kind)) {
    return { ok: false, error: "L’étape courante n’est pas une saisie famille." };
  }

  const campagne = await getFdCampagne(params.etablissementId, fiche.campagneId);
  if (!campagne) return { ok: false, error: "Campagne introuvable." };
  if (campagne.statut === "cloturee" || campagne.statut === "archivee") {
    return { ok: false, error: "La campagne est clôturée." };
  }

  const emails = (fiche.parentEmails ?? []).filter(isValidParentEmail);
  if (!emails.length) return { ok: false, error: "Aucun e-mail parent renseigné." };

  const purpose = etape.kind === "acceptation_famille" ? "acceptation" : "saisie";
  await revokeFdTokensForFiche(fiche.id, purpose);

  let notify: { sent: boolean; reason?: string };
  if (etape.kind === "acceptation_famille") {
    /** L’acceptation reste un acte unique : un seul lien, envoyé aux deux responsables. */
    const tokenRow = await createFdAccessToken({
      etablissementId: params.etablissementId,
      ficheId: fiche.id,
      etapeId: etape.id,
      email: emails[0],
      purpose,
      expiresInDays: Math.max(campagne.delaiFamilleJours, 14),
    });
    notify = await notifyFdAcceptationRequest({
      to: emails,
      elevePrenom: fiche.elevePrenom,
      eleveNom: fiche.eleveNom,
      token: tokenRow.token,
      secureCode: tokenRow.secureCode ?? "",
    });
  } else {
    /**
     * Un jeton personnel par responsable : le dépôt est ainsi attribué à la bonne
     * adresse, et le parent qui n’a pas déposé reçoit le lien de confirmation.
     * Sans cela, les deux parents partageaient le même lien et le même code.
     */
    const tokenRows: Awaited<ReturnType<typeof createFdAccessToken>>[] = [];
    for (const email of emails) {
      tokenRows.push(
        await createFdAccessToken({
          etablissementId: params.etablissementId,
          ficheId: fiche.id,
          etapeId: etape.id,
          email,
          purpose,
          expiresInDays: Math.max(campagne.delaiFamilleJours, 14),
        }),
      );
    }
    let sent = false;
    const reasons: string[] = [];
    for (const tokenRow of tokenRows) {
      const recipient = tokenRow.email;
      if (!recipient) continue;
      const res = await notifyFdFamilleSaisie({
        to: [recipient],
        elevePrenom: fiche.elevePrenom,
        eleveNom: fiche.eleveNom,
        classe: fiche.classeActuelle,
        campagneLabel: campagne.label,
        etapeLabel: etape.label,
        token: tokenRow.token,
        secureCode: tokenRow.secureCode ?? "",
        delaiJours: campagne.delaiFamilleJours,
        reminder: params.reminder,
      });
      if (res.sent) sent = true;
      else if (res.reason) reasons.push(`${recipient}: ${res.reason}`);
    }
    notify = { sent, reason: reasons.join(" | ") || undefined };
  }

  const db = getDb();
  /**
   * Ne jamais faire croire à un envoi réussi quand SMTP est absent : le statut
   * ne bascule qu’après un envoi réellement accepté, et la date d’envoi reste
   * celle du dernier succès. L’admin peut alors relancer sans perdre l’information.
   */
  await db
    .update(fdFiche)
    .set({
      statut: notify.sent
        ? etape.kind === "acceptation_famille"
          ? "en_attente_acceptation"
          : "en_attente_famille"
        : fiche.statut,
      lastSentAt: notify.sent ? now() : fiche.lastSentAt,
      lastReminderAt: notify.sent && params.reminder ? now() : fiche.lastReminderAt,
      reminderCount:
        notify.sent && params.reminder ? fiche.reminderCount + 1 : fiche.reminderCount,
      updatedAt: now(),
    })
    .where(eq(fdFiche.id, fiche.id));

  if (notify.sent && campagne.statut === "brouillon") {
    await updateFdCampagne(params.etablissementId, campagne.id, { statut: "active" });
  }

  return { ok: true, sent: notify.sent, reason: notify.reason };
}

/**
 * Relance du 2e parent : recrée un jeton d’accord si besoin puis renvoie le lien.
 * Corrige le trou où « Relancer les non-réponses » ignorait les fiches
 * en attente d’accord parental.
 */
export async function resendFdParent2Accord(params: {
  etablissementId: string;
  ficheId: string;
}): Promise<{ ok: true; sent: boolean } | { ok: false; error: string }> {
  await applyFdTacitParentAccordIfDue(params.etablissementId, params.ficheId);

  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  if (fiche.statut !== "en_attente_accord_parent2") {
    return { ok: false, error: "Aucun accord parental en attente." };
  }
  const accord = fiche.parentAccord;
  const autreEmail = accord?.autreEmail ? normalizeParentEmail(accord.autreEmail) : "";
  if (!autreEmail) return { ok: false, error: "Aucun 2e parent renseigné." };

  const etapeId = fiche.etapeCouranteId ?? null;
  const deadline = accord?.accordDeadlineAt ? new Date(accord.accordDeadlineAt) : null;
  const expiresInDays =
    (deadline && deadline.getTime() > Date.now()
      ? Math.max(1, Math.ceil((deadline.getTime() - Date.now()) / (24 * 60 * 60 * 1000)))
      : 10) + 30;

  await revokeFdTokensForFiche(fiche.id, "accord_parent2");
  const tokenRow = await createFdAccessToken({
    etablissementId: params.etablissementId,
    ficheId: fiche.id,
    etapeId,
    email: autreEmail,
    purpose: "accord_parent2",
    expiresInDays,
  });

  const notify = await notifyFdParent2Accord({
    to: autreEmail,
    elevePrenom: fiche.elevePrenom,
    eleveNom: fiche.eleveNom,
    deposantLabel: accord?.deposantLabel || "l’autre responsable",
    resume: "Votre accord est toujours attendu sur les vœux déposés (détail dans le lien).",
    deadlineLabel: deadline ? deadline.toLocaleString("fr-FR") : "la date limite indiquée",
    token: tokenRow.token,
  });

  const db = getDb();
  await db
    .update(fdFiche)
    .set({
      lastReminderAt: notify.sent ? now() : fiche.lastReminderAt,
      reminderCount: notify.sent ? fiche.reminderCount + 1 : fiche.reminderCount,
      updatedAt: now(),
    })
    .where(eq(fdFiche.id, fiche.id));

  return { ok: true, sent: notify.sent };
}

export async function sendFdCampagneEtapeToFamilles(params: {
  etablissementId: string;
  campagneId: string;
  onlyMissing?: boolean;
  reminder?: boolean;
}): Promise<{ sent: number; failed: number; errors: string[] }> {
  const fiches = await listFdFiches(params.etablissementId, params.campagneId);
  const etapes = await listFdEtapes(params.etablissementId, params.campagneId);
  const etapeById = new Map(etapes.map((e) => [e.id, e]));

  let sent = 0;
  let failed = 0;
  const errors: string[] = [];
  for (const fiche of fiches) {
    const etape = fiche.etapeCouranteId ? etapeById.get(fiche.etapeCouranteId) : undefined;

    if (fiche.statut === "en_attente_accord_parent2") {
      if (params.reminder) {
        const res = await resendFdParent2Accord({
          etablissementId: params.etablissementId,
          ficheId: fiche.id,
        });
        if (res.ok && res.sent) sent += 1;
        else {
          failed += 1;
          errors.push(
            `${fiche.elevePrenom} ${fiche.eleveNom} (2e parent): ${
              res.ok ? "envoi non délivré (SMTP indisponible)" : res.error
            }`,
          );
        }
      }
      continue;
    }

    const etapeFamille =
      etape?.kind === "saisie_famille" ||
      etape?.kind === "choix_definitifs" ||
      etape?.kind === "acceptation_famille";
    /**
     * `decision_envoyee` = décision conseil publiée, étape courante désormais côté famille
     * (ex. choix définitifs). Il faut renvoyer le lien : c’était le blocage post-conseil.
     * On ne renvoie jamais aux fiches déjà reçues / au conseil / terminales.
     */
    const waiting =
      Boolean(etapeFamille) &&
      (fiche.statut === "a_envoyer" ||
        fiche.statut === "en_attente_famille" ||
        fiche.statut === "en_attente_acceptation" ||
        fiche.statut === "decision_envoyee");
    if (!waiting) continue;

    const res = await sendFdFicheToFamille({
      etablissementId: params.etablissementId,
      ficheId: fiche.id,
      reminder: params.reminder,
    });
    if (res.ok && res.sent) sent += 1;
    else {
      failed += 1;
      errors.push(
        `${fiche.elevePrenom} ${fiche.eleveNom}: ${
          res.ok ? `envoi non délivré (${res.reason ?? "SMTP indisponible"})` : res.error
        }`,
      );
    }
  }
  const campagne = await getFdCampagne(params.etablissementId, params.campagneId);
  if (sent > 0 && campagne && campagne.statut === "brouillon") {
    await updateFdCampagne(params.etablissementId, params.campagneId, { statut: "active" });
  }
  return { sent, failed, errors };
}

export async function submitFdFamilleReponse(params: {
  etablissementId: string;
  ficheId: string;
  etapeId: string;
  payload: FdReponsePayload;
  auteurLabel?: string;
  signature?: { name: string; pngBase64?: string; method?: string; email?: string };
}): Promise<{ ok: true; needsParent2?: boolean } | { ok: false; error: string }> {
  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  const etape = await getEtape(params.etablissementId, params.etapeId);
  if (!etape) return { ok: false, error: "Étape introuvable." };
  if (!isFdEtapeOpenForFamille(etape)) {
    return {
      ok: false,
      error: etape.gelee
        ? "Étape figée : modification impossible."
        : "La date limite de cette étape est dépassée (ou pas encore ouverte).",
    };
  }
  if (etape.kind !== "saisie_famille" && etape.kind !== "choix_definitifs") {
    return { ok: false, error: "Cette étape n’accepte pas une saisie de vœux." };
  }
  if (fiche.etapeCouranteId !== etape.id) {
    return { ok: false, error: "Cette étape n’est plus la étape courante." };
  }

  const campagne = await getFdCampagne(params.etablissementId, fiche.campagneId);
  if (!campagne) return { ok: false, error: "Campagne introuvable." };
  if (campagne.statut === "cloturee" || campagne.statut === "archivee") {
    return { ok: false, error: "La campagne est clôturée : la saisie n’est plus possible." };
  }

  const db = getDb();
  await db
    .insert(fdReponse)
    .values({
      etablissementId: params.etablissementId,
      ficheId: fiche.id,
      etapeId: etape.id,
      auteurRole: "famille",
      auteurLabel: params.auteurLabel ?? "Famille",
      payload: params.payload,
    })
    .onConflictDoUpdate({
      target: [fdReponse.ficheId, fdReponse.etapeId, fdReponse.auteurRole],
      set: {
        payload: params.payload,
        auteurLabel: params.auteurLabel ?? "Famille",
        submittedAt: now(),
      },
    });

  if (params.signature?.name) {
    await db
      .insert(fdSignature)
      .values({
        etablissementId: params.etablissementId,
        ficheId: fiche.id,
        etapeId: etape.id,
        role: "famille",
        signerName: params.signature.name,
        signerEmail: params.signature.email ?? null,
        method: params.signature.method ?? "pad",
        signaturePngBase64: params.signature.pngBase64 ?? null,
      })
      .onConflictDoUpdate({
        target: [fdSignature.ficheId, fdSignature.etapeId, fdSignature.role],
        set: {
          signerName: params.signature.name,
          signaturePngBase64: params.signature.pngBase64 ?? null,
          method: params.signature.method ?? "pad",
          signedAt: now(),
        },
      });
  }

  const sections = sectionsFromFamilleReponse(campagne.catalogue, params.payload);
  const pdfBytes = await buildFicheDialoguePdf({
    title: "Vœux de la famille",
    campagneLabel: campagne.label,
    anneeLabel: campagne.anneeLabel,
    eleveNom: fiche.eleveNom,
    elevePrenom: fiche.elevePrenom,
    classeActuelle: fiche.classeActuelle,
    etapeLabel: etape.label,
    identity: identityFromFiche(fiche),
    sections,
    signatures: params.signature?.name
      ? [{ role: "Famille", name: params.signature.name }]
      : undefined,
  });

  await fileFicheDialoguePdfToDossier({
    etablissementId: params.etablissementId,
    ficheId: fiche.id,
    eleveId: fiche.eleveId,
    etapeId: etape.id,
    kind: etape.kind,
    title: `Fiche de dialogue — ${etape.label}`,
    pdfBytes,
    anneeLabel: campagne.anneeLabel,
  });

  const deposantEmail = params.signature?.email
    ? normalizeParentEmail(params.signature.email)
    : "";
  const allEmails = (fiche.parentEmails ?? [])
    .map(normalizeParentEmail)
    .filter(isValidParentEmail);
  const autres = allEmails.filter((e) => e && e !== deposantEmail);
  const autreEmail = autres[0] ?? null;

  const TACITE_DAYS = 10;
  let statut: FdFicheRow["statut"] = "saisie_recue";
  let parentAccord: FdParentAccordPayload | null = null;

  if (autreEmail && deposantEmail) {
    const deadlineFromDeposit = new Date(Date.now() + TACITE_DAYS * 24 * 60 * 60 * 1000);
    const closesAt = etape.closesAt ? new Date(etape.closesAt) : null;
    const accordDeadlineAt =
      closesAt && closesAt.getTime() < deadlineFromDeposit.getTime()
        ? closesAt
        : deadlineFromDeposit;

    parentAccord = {
      deposantEmail,
      deposantLabel: params.auteurLabel ?? "Famille",
      depositedAt: now().toISOString(),
      autreEmail,
      statutAutre: "en_attente",
      accordDeadlineAt: accordDeadlineAt.toISOString(),
    };
    statut = "en_attente_accord_parent2";

    await revokeFdTokensForFiche(fiche.id, "accord_parent2");
    const tokenRow = await createFdAccessToken({
      etablissementId: params.etablissementId,
      ficheId: fiche.id,
      etapeId: etape.id,
      email: autreEmail,
      purpose: "accord_parent2",
      /** + 30 jours de grâce pour consulter l’accord tacite après l’échéance. */
      expiresInDays:
        Math.max(
          1,
          Math.ceil((accordDeadlineAt.getTime() - Date.now()) / (24 * 60 * 60 * 1000)),
        ) + 30,
    });

    const resumeLines = sections
      .flatMap((s) => s.lines)
      .slice(0, 12)
      .join("\n");
    await notifyFdParent2Accord({
      to: autreEmail,
      elevePrenom: fiche.elevePrenom,
      eleveNom: fiche.eleveNom,
      deposantLabel: parentAccord.deposantLabel || "l’autre responsable",
      resume: resumeLines || "Vœux déposés (détail dans le lien).",
      deadlineLabel: accordDeadlineAt.toLocaleString("fr-FR"),
      token: tokenRow.token,
    });
  }

  await db
    .update(fdFiche)
    .set({
      statut,
      parentAccord,
      conflictPayload: null,
      updatedAt: now(),
    })
    .where(eq(fdFiche.id, fiche.id));

  /**
   * Le déposant garde son lien de saisie jusqu’à la clôture de l’étape : il peut
   * corriger ses vœux. Les liens des autres responsables sont révoqués pour éviter
   * qu’un second dépôt écrase silencieusement le premier ; ils reçoivent le lien
   * de confirmation (accord parent2).
   */
  await revokeFdTokensForFiche(fiche.id, "saisie", {
    exceptEmail: deposantEmail || null,
  });
  return { ok: true, needsParent2: statut === "en_attente_accord_parent2" };
}

/** Confirmation ou contradiction du 2e parent (ou application tacite). */
export async function submitFdParentAccord(params: {
  etablissementId: string;
  ficheId: string;
  email: string;
  decision: "confirme" | "contredit";
  motif?: string;
  conflictValues?: Record<string, string | string[] | boolean | null>;
}): Promise<{ ok: true } | { ok: false; error: string }> {
  await applyFdTacitParentAccordIfDue(params.etablissementId, params.ficheId);

  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  const campagne = await getFdCampagne(params.etablissementId, fiche.campagneId);
  if (!campagne) return { ok: false, error: "Campagne introuvable." };
  if (campagne.statut === "cloturee" || campagne.statut === "archivee") {
    return { ok: false, error: "La campagne est clôturée." };
  }

  const email = normalizeParentEmail(params.email);
  const accord = fiche.parentAccord;
  if (!accord?.autreEmail || normalizeParentEmail(accord.autreEmail) !== email) {
    return { ok: false, error: "Vous n’êtes pas le destinataire de cette confirmation." };
  }
  /** Le lien reste consultable 30 jours après l’échéance : ne pas afficher une erreur
   *  à un parent dont l’accord tacite a déjà été enregistré. */
  if (fiche.statut === "saisie_recue" && accord.statutAutre === "tacite") {
    return { ok: true };
  }
  if (fiche.statut !== "en_attente_accord_parent2") {
    return { ok: false, error: "Aucun accord parental en attente." };
  }

  const db = getDb();
  if (params.decision === "confirme") {
    await db
      .update(fdFiche)
      .set({
        statut: "saisie_recue",
        parentAccord: {
          ...accord,
          statutAutre: "confirme",
          confirmeAt: now().toISOString(),
        },
        updatedAt: now(),
      })
      .where(eq(fdFiche.id, fiche.id));
    /** On ne révoque que le lien de confirmation : le déposant garde son lien de
     *  saisie pour corriger ses vœux jusqu’à la clôture de l’étape. */
    await revokeFdTokensForFiche(fiche.id, "accord_parent2");
    return { ok: true };
  }

  const conflict: FdParentAccordPayload = {
    ...accord,
    statutAutre: "contredit",
    conflictMotif: params.motif?.trim() || undefined,
    conflictValues: params.conflictValues,
  };
  await db
    .update(fdFiche)
    .set({
      statut: "en_conflit",
      parentAccord: conflict,
      conflictPayload: conflict,
      updatedAt: now(),
    })
    .where(eq(fdFiche.id, fiche.id));

  /**
   * Un conflit ne doit rester silencieux ni pour l’équipe (PP/direction) ni pour
   * les parents : on notifie les deux côtés, sans bloquer l’enregistrement si SMTP
   * est indisponible. L’horodatage garde la trace de l’alerte réellement partie.
   */
  try {
    const staffTo = await resolveFdStaffEmails(fiche);
    const parentsTo = (fiche.parentEmails ?? [])
      .map(normalizeParentEmail)
      .filter(isValidParentEmail);
    const results = await Promise.allSettled([
      notifyFdConflictStaff({
        to: staffTo,
        elevePrenom: fiche.elevePrenom,
        eleveNom: fiche.eleveNom,
        classe: fiche.classeActuelle,
        campagneLabel: campagne.label,
        campagneId: campagne.id,
        motif: conflict.conflictMotif,
      }),
      notifyFdConflictParents({
        to: parentsTo,
        elevePrenom: fiche.elevePrenom,
        eleveNom: fiche.eleveNom,
        campagneLabel: campagne.label,
        motif: conflict.conflictMotif,
      }),
    ]);
    const notified = results.some(
      (r) => r.status === "fulfilled" && r.value.sent === true,
    );
    if (notified) {
      const notifiedAt = now().toISOString();
      const withNotice: FdParentAccordPayload = { ...conflict, conflictNotifiedAt: notifiedAt };
      await db
        .update(fdFiche)
        .set({ parentAccord: withNotice, conflictPayload: withNotice, updatedAt: now() })
        .where(eq(fdFiche.id, fiche.id));
    }
  } catch {
    /* notification best-effort : le conflit reste enregistré */
  }

  await revokeFdTokensForFiche(fiche.id);
  return { ok: true };
}

/** Si la date limite d’accord est dépassée → acceptation tacite. */
export async function applyFdTacitParentAccordIfDue(
  etablissementId: string,
  ficheId: string,
): Promise<boolean> {
  const fiche = await getFdFiche(etablissementId, ficheId);
  if (!fiche || fiche.statut !== "en_attente_accord_parent2") return false;
  const accord = fiche.parentAccord;
  if (!accord?.accordDeadlineAt || accord.statutAutre !== "en_attente") return false;
  if (new Date(accord.accordDeadlineAt).getTime() > Date.now()) return false;

  const db = getDb();
  const taciteAccord: FdParentAccordPayload = {
    ...accord,
    statutAutre: "tacite",
    taciteAt: now().toISOString(),
  };
  await db
    .update(fdFiche)
    .set({
      statut: "saisie_recue",
      parentAccord: taciteAccord,
      updatedAt: now(),
    })
    .where(eq(fdFiche.id, fiche.id));

  /** Information des deux responsables, best-effort : l’accord tacite reste acquis. */
  try {
    const campagne = await getFdCampagne(etablissementId, fiche.campagneId);
    const etape = fiche.etapeCouranteId
      ? await getEtape(etablissementId, fiche.etapeCouranteId)
      : null;
    const parentsTo = (fiche.parentEmails ?? [])
      .map(normalizeParentEmail)
      .filter(isValidParentEmail);
    if (campagne && etape) {
      const res = await notifyFdTacitAccord({
        to: parentsTo,
        elevePrenom: fiche.elevePrenom,
        eleveNom: fiche.eleveNom,
        campagneLabel: campagne.label,
        etapeLabel: etape.label,
        accordDeadlineAt: accord.accordDeadlineAt,
      });
      if (res.sent) {
        const withNotice: FdParentAccordPayload = {
          ...taciteAccord,
          taciteNotifiedAt: now().toISOString(),
        };
        await db
          .update(fdFiche)
          .set({ parentAccord: withNotice, updatedAt: now() })
          .where(eq(fdFiche.id, fiche.id));
      }
    }
  } catch {
    /* notification best-effort */
  }
  /**
   * On conserve le jeton de confirmation pendant sa période de grâce : le parent
   * qui clique après l’échéance doit lire « accord tacite enregistré », pas
   * « lien invalide ». Le workflow reste idempotent si l’action est rejouée.
   */
  return true;
}

/**
 * Résolution direction d’un conflit parents.
 * - maintien_famille : on conserve la réponse déposée (le conseil en tient compte).
 * - avis_conseil : le conseil tranche ; la réponse famille reste tracée mais résolue.
 * Dans les deux cas la fiche redevient `saisie_recue`, donc avançable vers le conseil.
 */
export async function resolveFdConflit(params: {
  etablissementId: string;
  ficheId: string;
  resolution: "maintien_famille" | "avis_conseil";
  note?: string;
  userId?: string | null;
  auteurLabel?: string;
}): Promise<{ ok: true } | { ok: false; error: string }> {
  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  if (fiche.statut !== "en_conflit") {
    return { ok: false, error: "Cette fiche n’est pas en conflit." };
  }

  const accord = fiche.parentAccord ?? fiche.conflictPayload ?? {};
  const db = getDb();
  const resolved: FdParentAccordPayload = {
    ...accord,
    resolution: params.resolution,
    resolutionNote: params.note?.trim() || undefined,
    resolvedAt: now().toISOString(),
    resolvedBy: params.userId ?? undefined,
  };
  await db
    .update(fdFiche)
    .set({
      statut: "saisie_recue",
      parentAccord: resolved,
      conflictPayload: resolved,
      updatedAt: now(),
    })
    .where(eq(fdFiche.id, fiche.id));
  return { ok: true };
}

export async function freezeFdEtape(params: {
  etablissementId: string;
  etapeId: string;
}): Promise<FdEtapeRow> {
  const db = getDb();
  const [row] = await db
    .update(fdEtape)
    .set({ gelee: true, frozenAt: now(), updatedAt: now() })
    .where(and(eq(fdEtape.etablissementId, params.etablissementId), eq(fdEtape.id, params.etapeId)))
    .returning();
  if (!row) throw new Error("ETAPE_NOT_FOUND");
  return row;
}

/**
 * Dégèle une étape famille pour traiter les fiches restées en arrière
 * (non-répondants, conflits résolus après le gel). Les fiches déjà avancées
 * ne sont pas concernées : leur étape courante est la suivante.
 */
export async function unfreezeFdEtape(params: {
  etablissementId: string;
  campagneId?: string;
  etapeId: string;
}): Promise<FdEtapeRow> {
  const db = getDb();
  const [row] = await db
    .update(fdEtape)
    .set({ gelee: false, frozenAt: null, updatedAt: now() })
    .where(
      and(
        eq(fdEtape.etablissementId, params.etablissementId),
        eq(fdEtape.id, params.etapeId),
        params.campagneId ? eq(fdEtape.campagneId, params.campagneId) : undefined,
      ),
    )
    .returning();
  if (!row) throw new Error("ETAPE_NOT_FOUND");
  return row;
}

export async function submitFdConseilDecision(params: {
  etablissementId: string;
  ficheId: string;
  etapeId: string;
  payload: FdConseilDecisionPayload;
  auteurUserId?: string | null;
  auteurLabel?: string;
  signatures: Array<{
    role: "professeur_principal" | "direction";
    name: string;
    pngBase64?: string;
    method?: string;
  }>;
  /** false = brouillon PP (en_attente_direction), true = publication famille. */
  publish?: boolean;
  /** Rectification explicite d’une décision déjà publiée — autorise une étape passée. */
  avenant?: boolean;
}): Promise<{ ok: true; pdfBytes?: Uint8Array; published: boolean } | { ok: false; error: string }> {
  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  const etape = await getEtape(params.etablissementId, params.etapeId);
  if (!etape) return { ok: false, error: "Étape introuvable." };
  if (!isConseilEtape(etape.kind)) {
    return { ok: false, error: "Cette étape n’est pas une étape de conseil." };
  }
  const campagne = await getFdCampagne(params.etablissementId, fiche.campagneId);
  if (!campagne) return { ok: false, error: "Campagne introuvable." };
  if (campagne.statut === "cloturee" || campagne.statut === "archivee") {
    return { ok: false, error: "La campagne est clôturée." };
  }

  const db = getDb();
  const publish = params.publish !== false;
  const existingConseil = await db
    .select()
    .from(fdReponse)
    .where(
      and(
        eq(fdReponse.ficheId, fiche.id),
        eq(fdReponse.etapeId, etape.id),
        eq(fdReponse.auteurRole, "conseil"),
      ),
    )
    .limit(1);
  const prevPayload = existingConseil[0]?.payload as FdConseilDecisionPayload | undefined;
  /** Les payloads legacy n’ont pas tous `publiee` : absence de drapeau = décision publiée. */
  const previouslyPublished = Boolean(
    existingConseil.length > 0 && prevPayload?.publiee !== false,
  );
  const isAvenant = Boolean(publish && params.avenant && previouslyPublished);

  if (previouslyPublished && !params.avenant) {
    return {
      ok: false,
      error:
        "Cette décision est déjà publiée. Publiez un avenant explicite pour la rectifier.",
    };
  }
  if (params.avenant && !publish) {
    return {
      ok: false,
      error: "Un avenant doit être publié (pas d’enregistrement en brouillon).",
    };
  }

  /**
   * Verrou d’étape : une publication normale ne peut viser que l’étape courante de la fiche.
   * Une rectification d’une étape passée doit être un avenant explicite.
   */
  if (fiche.etapeCouranteId !== etape.id && !(params.avenant && isAvenant)) {
    return {
      ok: false,
      error:
        "Cette étape n’est plus l’étape courante de la fiche. Publiez un avenant explicite pour la rectifier.",
    };
  }
  /** Défense en profondeur : la publication exige une signature direction (aussi contrôlée côté route). */
  if (publish && !params.signatures.some((s) => s.role === "direction")) {
    return { ok: false, error: "La publication nécessite une signature direction." };
  }

  const etapes = await listFdEtapes(params.etablissementId, fiche.campagneId);
  const next = findNextEtape(etapes, etape, { activateAppel: false });
  /** Dernier avis : décision finale explicite, suite = acceptation famille, ou plus aucune étape. */
  const isFinalConseil =
    etape.kind === "decision_finale_conseil" ||
    next?.kind === "acceptation_famille" ||
    next === null;

  const payload: FdConseilDecisionPayload = {
    ...params.payload,
    publiee: publish,
    publishedAt: publish ? now().toISOString() : undefined,
    avenant: isAvenant || undefined,
    avenantMotif: isAvenant
      ? params.payload.motif?.trim() || params.payload.commentaire?.trim() || undefined
      : undefined,
    avenantAt: isAvenant ? now().toISOString() : undefined,
  };

  await db
    .insert(fdReponse)
    .values({
      etablissementId: params.etablissementId,
      ficheId: fiche.id,
      etapeId: etape.id,
      auteurRole: "conseil",
      auteurUserId: params.auteurUserId ?? null,
      auteurLabel: params.auteurLabel ?? "Conseil de classe",
      payload,
    })
    .onConflictDoUpdate({
      target: [fdReponse.ficheId, fdReponse.etapeId, fdReponse.auteurRole],
      set: { payload, submittedAt: now(), auteurLabel: params.auteurLabel },
    });

  for (const sig of params.signatures) {
    await db
      .insert(fdSignature)
      .values({
        etablissementId: params.etablissementId,
        ficheId: fiche.id,
        etapeId: etape.id,
        role: sig.role,
        signerName: sig.name,
        method: sig.method ?? "pad",
        signaturePngBase64: sig.pngBase64 ?? null,
      })
      .onConflictDoUpdate({
        target: [fdSignature.ficheId, fdSignature.etapeId, fdSignature.role],
        set: {
          signerName: sig.name,
          signaturePngBase64: sig.pngBase64 ?? null,
          method: sig.method ?? "pad",
          signedAt: now(),
        },
      });
  }

  if (!publish) {
    await db
      .update(fdFiche)
      .set({
        statut: "en_attente_direction",
        etapeCouranteId: etape.id,
        updatedAt: now(),
      })
      .where(eq(fdFiche.id, fiche.id));
    return { ok: true, published: false };
  }

  const history = await loadFicheHistorySections(params.etablissementId, fiche, campagne.catalogue);
  const sections = [
    ...history,
    ...sectionsFromConseil(campagne.catalogue, payload),
  ];
  const pdfBytes = await buildFicheDialoguePdf({
    title: isAvenant
      ? "AVENANT — Rectification du conseil de classe"
      : isFinalConseil
        ? "Décision définitive du conseil de classe"
        : "Avis du conseil de classe",
    subtitle: isAvenant
      ? "Document de rectification — remplace la version précédemment communiquée"
      : undefined,
    campagneLabel: campagne.label,
    anneeLabel: campagne.anneeLabel,
    eleveNom: fiche.eleveNom,
    elevePrenom: fiche.elevePrenom,
    classeActuelle: fiche.classeActuelle,
    etapeLabel: etape.label,
    identity: identityFromFiche(fiche),
    sections,
    signatures: params.signatures.map((s) => ({
      role: s.role === "direction" ? "Direction" : "Professeur principal",
      name: s.name,
    })),
  });

  await fileFicheDialoguePdfToDossier({
    etablissementId: params.etablissementId,
    ficheId: fiche.id,
    eleveId: fiche.eleveId,
    etapeId: etape.id,
    kind: isAvenant ? `${etape.kind}_avenant` : etape.kind,
    title: isAvenant
      ? `Avenant — fiche de dialogue — ${etape.label}`
      : `Fiche de dialogue — ${etape.label}`,
    pdfBytes,
    anneeLabel: campagne.anneeLabel,
    filedByUserId: params.auteurUserId,
  });

  const emails = (fiche.parentEmails ?? []).filter(isValidParentEmail);
  if (isAvenant) {
    await notifyFdAvenantConseil({
      to: emails,
      elevePrenom: fiche.elevePrenom,
      eleveNom: fiche.eleveNom,
      etapeLabel: etape.label,
      motif: params.payload.motif || params.payload.commentaire || undefined,
      pdfBytes,
      fileName: `avenant-fiche-dialogue-${etape.kind}.pdf`,
    });
  } else {
    await notifyFdDecisionPdf({
      to: emails,
      elevePrenom: fiche.elevePrenom,
      eleveNom: fiche.eleveNom,
      etapeLabel: etape.label,
      pdfBytes,
      fileName: `fiche-dialogue-${etape.kind}.pdf`,
      intro:
        isFinalConseil
          ? "Voici la décision définitive du conseil de classe concernant votre enfant."
          : "Voici l’avis du conseil de classe concernant la fiche de dialogue de votre enfant.",
    });
  }

  if (isFinalConseil) {
    const acceptEtape =
      next?.kind === "acceptation_famille"
        ? next
        : etapes.find((e) => e.kind === "acceptation_famille") ?? null;
    const target = acceptEtape ?? next ?? etape;
    await db
      .update(fdFiche)
      .set({
        /**
         * Campagnes Terminale créées avant l’ajout de l’étape d’acceptation :
         * sans étape famille suivante, on clôture au lieu de laisser la fiche
         * bloquée en `en_attente_acceptation` sans formulaire.
         */
        statut: acceptEtape ? "en_attente_acceptation" : "cloturee",
        etapeCouranteId: target.id,
        closedAt: acceptEtape ? fiche.closedAt : now(),
        updatedAt: now(),
      })
      .where(eq(fdFiche.id, fiche.id));
    if (acceptEtape) {
      // Envoi automatique du lien d’acceptation (meilleur effort : l’admin peut relancer).
      await sendFdFicheToFamille({
        etablissementId: params.etablissementId,
        ficheId: fiche.id,
      });
    }
  } else {
    await db
      .update(fdFiche)
      .set({
        statut: "decision_envoyee",
        etapeCouranteId: next?.id ?? etape.id,
        updatedAt: now(),
      })
      .where(eq(fdFiche.id, fiche.id));
    if (next && isFamilleEtape(next.kind)) {
      // Envoi automatique de l’étape famille suivante : plus besoin d’un envoi manuel post-conseil.
      await sendFdFicheToFamille({
        etablissementId: params.etablissementId,
        ficheId: fiche.id,
      });
    }
  }

  return { ok: true, pdfBytes, published: true };
}

async function loadFicheHistorySections(
  etablissementId: string,
  fiche: FdFicheRow,
  catalogue: FdCatalogueChoix,
) {
  const db = getDb();
  const etapes = await listFdEtapes(etablissementId, fiche.campagneId);
  const reponses = await db
    .select()
    .from(fdReponse)
    .where(eq(fdReponse.ficheId, fiche.id))
    .orderBy(asc(fdReponse.submittedAt));

  const sections: Array<{ title: string; lines: string[] }> = [];
  for (const etape of etapes) {
    const reps = reponses.filter((r) => r.etapeId === etape.id);
    for (const rep of reps) {
      if (rep.auteurRole === "famille" && "values" in (rep.payload as object)) {
        sections.push(
          ...sectionsFromFamilleReponse(catalogue, rep.payload as FdReponsePayload).map((s) => ({
            ...s,
            title: `${etape.label} — ${s.title}`,
          })),
        );
      } else if (rep.auteurRole === "conseil") {
        sections.push(
          ...sectionsFromConseil(catalogue, rep.payload as FdConseilDecisionPayload).map((s) => ({
            ...s,
            title: `${etape.label} — ${s.title}`,
          })),
        );
      }
    }
  }
  return sections;
}

function findNextEtape(
  etapes: FdEtapeRow[],
  current: FdEtapeRow,
  opts: { activateAppel: boolean },
): FdEtapeRow | null {
  const sorted = [...etapes].sort((a, b) => a.ordre - b.ordre);
  const idx = sorted.findIndex((e) => e.id === current.id);
  for (let i = idx + 1; i < sorted.length; i++) {
    const e = sorted[i];
    if (e.optionnelle && e.kind === "appel" && !opts.activateAppel) continue;
    if (e.optionnelle && e.kind === "appel" && opts.activateAppel) return e;
    if (!e.optionnelle) return e;
  }
  return null;
}

export async function submitFdAcceptation(params: {
  etablissementId: string;
  ficheId: string;
  etapeId: string;
  payload: FdAcceptationPayload;
  auteurLabel?: string;
  signature?: { name: string; pngBase64?: string; method?: string; email?: string };
}): Promise<{ ok: true } | { ok: false; error: string }> {
  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  const etape = await getEtape(params.etablissementId, params.etapeId);
  if (!etape || etape.kind !== "acceptation_famille") {
    return { ok: false, error: "Étape d’acceptation invalide." };
  }
  if (!isFdEtapeOpenForFamille(etape)) {
    return {
      ok: false,
      error: etape.gelee
        ? "Étape figée."
        : "La date limite de cette étape est dépassée (ou pas encore ouverte).",
    };
  }

  const campagne = await getFdCampagne(params.etablissementId, fiche.campagneId);
  if (!campagne) return { ok: false, error: "Campagne introuvable." };
  if (campagne.statut === "cloturee" || campagne.statut === "archivee") {
    return { ok: false, error: "La campagne est clôturée : l’acceptation n’est plus possible." };
  }
  if (fiche.etapeCouranteId !== etape.id) {
    return { ok: false, error: "Cette étape d’acceptation n’est plus l’étape courante de la fiche." };
  }

  const db = getDb();
  await db
    .insert(fdReponse)
    .values({
      etablissementId: params.etablissementId,
      ficheId: fiche.id,
      etapeId: etape.id,
      auteurRole: "famille",
      auteurLabel: params.auteurLabel ?? "Famille",
      payload: params.payload,
    })
    .onConflictDoUpdate({
      target: [fdReponse.ficheId, fdReponse.etapeId, fdReponse.auteurRole],
      set: { payload: params.payload, submittedAt: now() },
    });

  if (params.signature?.name) {
    await db
      .insert(fdSignature)
      .values({
        etablissementId: params.etablissementId,
        ficheId: fiche.id,
        etapeId: etape.id,
        role: "famille",
        signerName: params.signature.name,
        signerEmail: params.signature.email ?? null,
        method: params.signature.method ?? "pad",
        signaturePngBase64: params.signature.pngBase64 ?? null,
      })
      .onConflictDoUpdate({
        target: [fdSignature.ficheId, fdSignature.etapeId, fdSignature.role],
        set: {
          signerName: params.signature.name,
          signaturePngBase64: params.signature.pngBase64 ?? null,
          signedAt: now(),
        },
      });
  }

  const history = await loadFicheHistorySections(params.etablissementId, fiche, campagne.catalogue);
  const sections = [
    ...history,
    ...sectionsFromAcceptation(params.payload, campagne.appelConfig),
  ];
  const pdfBytes = await buildFicheDialoguePdf({
    title: "Fiche de dialogue — document final",
    subtitle: params.payload.accepte
      ? "Décision acceptée par la famille"
      : "Décision refusée — procédure d’appel possible",
    campagneLabel: campagne.label,
    anneeLabel: campagne.anneeLabel,
    eleveNom: fiche.eleveNom,
    elevePrenom: fiche.elevePrenom,
    classeActuelle: fiche.classeActuelle,
    etapeLabel: etape.label,
    identity: identityFromFiche(fiche),
    sections,
    signatures: params.signature?.name
      ? [{ role: "Famille", name: params.signature.name }]
      : undefined,
    footerNote: params.payload.accepte
      ? "Ce document clôture la fiche de dialogue pour l’année en cours."
      : "Ce document constate le désaccord et ouvre la possibilité d’un appel.",
  });

  await fileFicheDialoguePdfToDossier({
    etablissementId: params.etablissementId,
    ficheId: fiche.id,
    eleveId: fiche.eleveId,
    etapeId: etape.id,
    kind: "document_final",
    title: `Fiche de dialogue — document final — ${fiche.elevePrenom} ${fiche.eleveNom}`,
    pdfBytes,
    anneeLabel: campagne.anneeLabel,
  });

  const emails = (fiche.parentEmails ?? []).filter(isValidParentEmail);

  if (params.payload.accepte) {
    await notifyFdDecisionPdf({
      to: emails,
      elevePrenom: fiche.elevePrenom,
      eleveNom: fiche.eleveNom,
      etapeLabel: "Document final",
      pdfBytes,
      fileName: `fiche-dialogue-finale-${fiche.eleveNom}.pdf`,
      intro:
        "Vous avez accepté la décision définitive du conseil de classe. Voici le document final pour vos archives.",
    });
    await db
      .update(fdFiche)
      .set({
        statut: "acceptee",
        acceptation: params.payload,
        acceptedAt: now(),
        closedAt: now(),
        updatedAt: now(),
      })
      .where(eq(fdFiche.id, fiche.id));
  } else {
    await notifyFdDecisionPdf({
      to: emails,
      elevePrenom: fiche.elevePrenom,
      eleveNom: fiche.eleveNom,
      etapeLabel: "Document final",
      pdfBytes,
      fileName: `fiche-dialogue-finale-${fiche.eleveNom}.pdf`,
      intro:
        "Voici le document final constatant la décision du conseil de classe et votre position.",
    });
    if (campagne.appelConfig?.enabled !== false) {
      // Appel actif par défaut (obligation légale) ; `refusee` reste atteignable si un
      // paramétrage explicite désactive l’appel (données héritées / cas particuliers).
      await notifyFdAppelProcedure({
        to: emails,
        elevePrenom: fiche.elevePrenom,
        eleveNom: fiche.eleveNom,
        appel: {
          ...campagne.appelConfig,
          contactPpLabel:
            campagne.appelConfig.contactPpLabel ||
            campagne.contactPpLabel ||
            undefined,
        },
        pdfBytes,
      });

      const staffTo = new Set<string>();
      try {
        const year = currentStageSchoolYear();
        const refs = await getStageReferentsConfig(year);
        for (const a of findPrincipalAssignments(refs, fiche.classeActuelle)) {
          if (a.email?.trim()) staffTo.add(a.email.trim().toLowerCase());
        }
        const directionEmail = await resolveStagesDirectionEmail(
          fiche.classeActuelle,
          fiche.classeActuelle,
        );
        if (directionEmail) staffTo.add(directionEmail.trim().toLowerCase());
        for (const e of await resolveStagesAdminEmails(
          fiche.classeActuelle,
          fiche.classeActuelle,
        )) {
          staffTo.add(e.trim().toLowerCase());
        }
      } catch {
        /* best-effort */
      }
      if (staffTo.size) {
        await notifyFdRefusStaff({
          to: [...staffTo],
          elevePrenom: fiche.elevePrenom,
          eleveNom: fiche.eleveNom,
          classe: fiche.classeActuelle,
          campagneLabel: campagne.label,
          dateLimiteAppel: campagne.appelConfig.dateLimite,
          contactPpLabel:
            campagne.appelConfig.contactPpLabel ||
            campagne.contactPpLabel ||
            undefined,
        });
      }

      const etapes = await listFdEtapes(params.etablissementId, fiche.campagneId);
      const appelEtape = etapes.find((e) => e.kind === "appel");
      await db
        .update(fdFiche)
        .set({
          statut: "en_appel",
          acceptation: params.payload,
          refusedAt: now(),
          etapeCouranteId: appelEtape?.id ?? fiche.etapeCouranteId,
          updatedAt: now(),
        })
        .where(eq(fdFiche.id, fiche.id));
    } else {
      await db
        .update(fdFiche)
        .set({
          statut: "refusee",
          acceptation: params.payload,
          refusedAt: now(),
          closedAt: now(),
          updatedAt: now(),
        })
        .where(eq(fdFiche.id, fiche.id));
    }
  }

  await revokeFdTokensForFiche(fiche.id);
  return { ok: true };
}

/**
 * Enregistre ou complète le dossier d’appel. La commission est extérieure :
 * l’établissement peut saisir les informations au fur et à mesure (dates,
 * documents attendus/reçus, observations) sans clôturer la fiche.
 */
export async function saveFdAppelDossier(params: {
  etablissementId: string;
  ficheId: string;
  patch: Partial<FdAppelPayload>;
  updatedBy?: string;
}): Promise<{ ok: true; appel: FdAppelPayload } | { ok: false; error: string }> {
  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  if (fiche.appel?.closedAt) {
    return { ok: false, error: "Dossier d’appel clôturé : lecture seule." };
  }
  if (fiche.statut !== "en_appel") {
    return { ok: false, error: "Cette fiche n'est pas en appel." };
  }

  const appel = mergeFdAppelPayload(fiche.appel, params.patch, params.updatedBy);
  const db = getDb();
  await db
    .update(fdFiche)
    .set({ appel, updatedAt: now() })
    .where(eq(fdFiche.id, fiche.id));
  return { ok: true, appel };
}

function mergeFdAppelPayload(
  current: FdAppelPayload | null | undefined,
  patch: Partial<FdAppelPayload>,
  updatedBy?: string,
): FdAppelPayload {
  const clean = (value: string | undefined | null) => {
    const s = value?.trim();
    return s ? s : undefined;
  };
  /**
   * `undefined` = champ absent du patch (on conserve l’existant) ;
   * chaîne vide = effacement explicite (correction d’une saisie erronée).
   */
  const text = (value: string | undefined | null, existing?: string) => {
    if (value === undefined) return existing;
    if (value === null || value.trim() === "") return undefined;
    return clean(value);
  };

  const next: FdAppelPayload = {
    ...(current ?? {}),
    commissionLabel: text(patch.commissionLabel, current?.commissionLabel),
    saisineDate: text(patch.saisineDate, current?.saisineDate),
    commissionDate: text(patch.commissionDate, current?.commissionDate),
    receivedAt: text(patch.receivedAt, current?.receivedAt),
    motif: text(patch.motif, current?.motif),
    destinationProposee: text(patch.destinationProposee, current?.destinationProposee),
    informations: text(patch.informations, current?.informations),
    updatedAt: now().toISOString(),
    updatedBy: updatedBy || current?.updatedBy,
  };

  if (patch.decision !== undefined) {
    next.decision = patch.decision ?? undefined;
  }
  if (patch.optionsProposees !== undefined) {
    next.optionsProposees = patch.optionsProposees;
  }
  if (patch.documents !== undefined) {
    next.documents = patch.documents
      .map((d) => ({
        label: clean(d.label) ?? "",
        reference: clean(d.reference),
        statut: d.statut === "recu" ? ("recu" as const) : ("attendu" as const),
        receivedAt: clean(d.receivedAt),
        notes: clean(d.notes),
      }))
      .filter((d) => d.label);
  }

  return next;
}

/**
 * Clôture l’appel après réception de la décision de la commission extérieure.
 * La fiche passe `cloturee`, un PDF de décision est archivé et les parents sont
 * notifiés. La décision et le motif sont obligatoires ; les dates restent
 * optionnelles car elles peuvent manquer au moment de la saisie.
 */
export async function closeFdAppel(params: {
  etablissementId: string;
  ficheId: string;
  patch: Partial<FdAppelPayload>;
  updatedBy?: string;
}): Promise<{ ok: true; sent: boolean; appel: FdAppelPayload } | { ok: false; error: string }> {
  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  if (fiche.appel?.closedAt) {
    return { ok: false, error: "Dossier d’appel déjà clôturé." };
  }
  if (fiche.statut !== "en_appel") {
    return { ok: false, error: "Cette fiche n'est pas en appel." };
  }

  const merged = mergeFdAppelPayload(fiche.appel, params.patch, params.updatedBy);
  if (merged.decision !== "maintien" && merged.decision !== "reformation") {
    return { ok: false, error: "La décision de la commission est obligatoire." };
  }
  if (!merged.motif?.trim()) {
    return { ok: false, error: "Le motif de la décision est obligatoire." };
  }

  const campagne = await getFdCampagne(params.etablissementId, fiche.campagneId);
  if (!campagne) return { ok: false, error: "Campagne introuvable." };

  const decisionLabel =
    merged.decision === "reformation" ? "Réformation de la décision" : "Maintien de la décision";
  const documentsLines = (merged.documents ?? []).map((d) => {
    const statut = d.statut === "recu" ? "reçu" : "attendu";
    return `${d.label} — ${statut}${d.reference ? ` — ${d.reference}` : ""}`;
  });
  const pdfBytes = await buildFicheDialoguePdf({
    title: "Décision d’appel",
    campagneLabel: campagne.label,
    anneeLabel: campagne.anneeLabel,
    eleveNom: fiche.eleveNom,
    elevePrenom: fiche.elevePrenom,
    classeActuelle: fiche.classeActuelle,
    etapeLabel: "Commission d’appel",
    identity: identityFromFiche(fiche),
    sections: [
      {
        title: "Dossier d’appel",
        lines: [
          `Commission : ${merged.commissionLabel || "Commission extérieure"}`,
          `Date de saisine : ${merged.saisineDate || "—"}`,
          `Date de la commission : ${merged.commissionDate || "—"}`,
          `Décision reçue le : ${merged.receivedAt || "—"}`,
          ...(merged.informations ? [`Informations : ${merged.informations}`] : []),
        ],
      },
      {
        title: "Décision",
        lines: [
          `Décision : ${decisionLabel}`,
          `Motif : ${merged.motif}`,
          ...(merged.destinationProposee
            ? [`Destination retenue : ${merged.destinationProposee}`]
            : []),
        ],
      },
      {
        title: "Documents",
        lines: documentsLines.length ? documentsLines : ["Aucun document renseigné."],
      },
    ],
    footerNote:
      "Décision rendue par une commission extérieure, enregistrée dans ScolIA pour le suivi de la fiche de dialogue.",
  });

  const etapeId = fiche.etapeCouranteId;
  if (etapeId) {
    await fileFicheDialoguePdfToDossier({
      etablissementId: params.etablissementId,
      ficheId: fiche.id,
      eleveId: fiche.eleveId,
      etapeId,
      kind: "appel_decision",
      title: "Fiche de dialogue — décision d’appel",
      pdfBytes,
      anneeLabel: campagne.anneeLabel,
    });
  }

  const emails = (fiche.parentEmails ?? [])
    .map(normalizeParentEmail)
    .filter(isValidParentEmail);
  const notify = await notifyFdDecisionPdf({
    to: emails,
    elevePrenom: fiche.elevePrenom,
    eleveNom: fiche.eleveNom,
    etapeLabel: "Décision d’appel",
    pdfBytes,
    fileName: `decision-appel-${fiche.eleveNom}.pdf`,
    intro: `La commission d’appel a rendu sa décision concernant ${fiche.elevePrenom} ${fiche.eleveNom} : ${decisionLabel.toLowerCase()}.`,
  });

  const closedAt = now().toISOString();
  const appel: FdAppelPayload = {
    ...merged,
    decidedAt: merged.decidedAt || closedAt,
    closedAt,
    ...(notify.sent ? { notifiedAt: closedAt } : {}),
    updatedAt: closedAt,
  };
  const db = getDb();
  await db
    .update(fdFiche)
    .set({
      statut: "cloturee",
      appel,
      closedAt: new Date(),
      updatedAt: now(),
    })
    .where(eq(fdFiche.id, fiche.id));
  await revokeFdTokensForFiche(fiche.id);
  return { ok: true, sent: notify.sent, appel };
}

export function isFdCampagneOpenForFamille(campagne: FdCampagneRow): boolean {
  return campagne.statut !== "cloturee" && campagne.statut !== "archivee";
}

export async function getFdPublicContext(token: string) {
  const { resolveFdToken } = await import("@/app/lib/fiches-dialogue-tokens");
  const tokenRow = await resolveFdToken(token);
  if (!tokenRow) return null;

  await applyFdTacitParentAccordIfDue(tokenRow.etablissementId, tokenRow.ficheId);

  const fiche = await getFdFiche(tokenRow.etablissementId, tokenRow.ficheId);
  if (!fiche) return null;
  const campagne = await getFdCampagne(tokenRow.etablissementId, fiche.campagneId);
  if (!campagne) return null;

  /**
   * Jeton d’étape obsolète : un lien de saisie/acceptation ne doit pas rouvrir une étape
   * qui n’est plus la step courante. Le jeton d’accord 2e parent est volontairement exclu
   * (il porte l’étape du dépôt et reste valable jusqu’à la décision).
   */
  if (
    (tokenRow.purpose === "saisie" || tokenRow.purpose === "acceptation") &&
    tokenRow.etapeId &&
    fiche.etapeCouranteId &&
    tokenRow.etapeId !== fiche.etapeCouranteId
  ) {
    return null;
  }

  const etapeId = tokenRow.etapeId ?? fiche.etapeCouranteId;
  if (!etapeId) return null;
  const etape = await getEtape(tokenRow.etablissementId, etapeId);
  if (!etape) return null;

  const db = getDb();
  const reponses = await db
    .select()
    .from(fdReponse)
    .where(eq(fdReponse.ficheId, fiche.id))
    .orderBy(asc(fdReponse.submittedAt));

  return {
    token: tokenRow,
    fiche,
    campagne,
    etape,
    reponses,
    campagneOpen: isFdCampagneOpenForFamille(campagne),
  };
}

/**
 * Gèle l’étape famille et avance **uniquement** les fiches réellement prêtes
 * (`saisie_recue`, accord 2e parent échu devenu tacite).
 * Les non-répondants, conflits et appels restent à l’étape et sont signalés.
 */
export async function advanceCampagneToConseil(params: {
  etablissementId: string;
  campagneId: string;
  fromEtapeId: string;
  /** Ciblage fin optionnel ; par défaut toutes les fiches de l’étape. */
  ficheIds?: string[];
}): Promise<{
  advanced: number;
  skipped: Array<{ ficheId: string; eleve: string; reason: string }>;
}> {
  const etapes = await listFdEtapes(params.etablissementId, params.campagneId);
  const from = etapes.find((e) => e.id === params.fromEtapeId);
  if (!from) throw new Error("ETAPE_NOT_FOUND");
  await freezeFdEtape({ etablissementId: params.etablissementId, etapeId: from.id });

  const nextConseil = etapes.find((e) => e.ordre > from.ordre && isConseilEtape(e.kind));
  if (!nextConseil) return { advanced: 0, skipped: [] };

  const db = getDb();
  const fiches = await listFdFiches(params.etablissementId, params.campagneId);
  const selected = params.ficheIds?.length ? new Set(params.ficheIds) : null;

  let advanced = 0;
  const skipped: Array<{ ficheId: string; eleve: string; reason: string }> = [];

  for (const fiche of fiches) {
    if (selected && !selected.has(fiche.id)) continue;
    if (fiche.etapeCouranteId !== from.id) continue;

    if (fiche.statut === "en_attente_accord_parent2") {
      await applyFdTacitParentAccordIfDue(params.etablissementId, fiche.id);
    }
    const refreshed = await getFdFiche(params.etablissementId, fiche.id);
    if (!refreshed) continue;

    if (refreshed.statut === "saisie_recue") {
      await db
        .update(fdFiche)
        .set({
          etapeCouranteId: nextConseil.id,
          statut: "en_conseil",
          updatedAt: now(),
        })
        .where(eq(fdFiche.id, refreshed.id));
      advanced += 1;
      continue;
    }

    if (
      refreshed.statut === "acceptee" ||
      refreshed.statut === "refusee" ||
      refreshed.statut === "cloturee" ||
      refreshed.statut === "figee"
    ) {
      continue;
    }

    const reason =
      refreshed.statut === "en_conflit"
        ? "Conflit parents à résoudre avant l’avancement"
        : refreshed.statut === "en_attente_accord_parent2"
          ? "Accord du 2e parent encore en attente"
          : refreshed.statut === "en_appel"
            ? "Fiche en appel"
            : refreshed.statut === "en_attente_direction"
              ? "Avis déjà transmis à la direction"
              : refreshed.statut === "en_conseil"
                ? "Fiche déjà au conseil"
                : "Aucune réponse famille";
    skipped.push({
      ficheId: refreshed.id,
      eleve: `${refreshed.elevePrenom} ${refreshed.eleveNom}`.trim(),
      reason,
    });
  }
  return { advanced, skipped };
}

export async function getFdCampagneStats(etablissementId: string, campagneId: string) {
  const fiches = await listFdFiches(etablissementId, campagneId);
  const byStatut: Record<string, number> = {};
  const byClasse: Record<string, number> = {};
  for (const f of fiches) {
    byStatut[f.statut] = (byStatut[f.statut] ?? 0) + 1;
    byClasse[f.classeActuelle || "—"] = (byClasse[f.classeActuelle || "—"] ?? 0) + 1;
  }
  return { total: fiches.length, byStatut, byClasse };
}

export async function listFdFichesByIds(
  etablissementId: string,
  ids: string[],
): Promise<FdFicheRow[]> {
  if (!ids.length) return [];
  const db = getDb();
  return db
    .select()
    .from(fdFiche)
    .where(and(eq(fdFiche.etablissementId, etablissementId), inArray(fdFiche.id, ids)));
}

export type FdDossierReponse = {
  id: string;
  etapeId: string;
  etapeLabel: string;
  etapeKind: FdEtapeKind;
  auteurRole: string;
  auteurLabel: string | null;
  submittedAt: string;
  payload: FdReponseRow["payload"];
  sections: FdPdfSection[];
};

export type FdFicheDossier = {
  fiche: FdFicheRow;
  campagne: FdCampagneRow;
  etapes: Array<{
    id: string;
    ordre: number;
    kind: FdEtapeKind;
    label: string;
    closesAt: string | null;
    gelee: boolean;
  }>;
  reponses: FdDossierReponse[];
  signatures: Array<{
    id: string;
    etapeId: string;
    role: string;
    signerName: string;
    signerEmail: string | null;
    method: string;
    signedAt: string;
  }>;
  archives: Array<{
    id: string;
    ficheId: string;
    etapeId: string | null;
    kind: string;
    title: string;
    createdAt: string;
    storage: "s3" | "db" | "none";
  }>;
  tokens: Array<{
    id: string;
    etapeId: string | null;
    email: string | null;
    purpose: string;
    expiresAt: string | null;
    usedAt: string | null;
    revokedAt: string | null;
    createdAt: string;
  }>;
};

function dossierSections(
  campagne: FdCampagneRow,
  etapeKind: FdEtapeKind,
  auteurRole: string,
  payload: FdReponseRow["payload"],
): FdPdfSection[] {
  if (auteurRole === "conseil") {
    return sectionsFromConseil(campagne.catalogue, payload as FdConseilDecisionPayload);
  }
  if (etapeKind === "acceptation_famille" && "accepte" in (payload as object)) {
    return sectionsFromAcceptation(payload as FdAcceptationPayload, campagne.appelConfig);
  }
  if ("values" in (payload as object)) {
    return sectionsFromFamilleReponse(campagne.catalogue, payload as FdReponsePayload);
  }
  return [
    {
      title: "Réponse",
      lines: Object.entries(payload as Record<string, unknown>)
        .filter(([, value]) => value !== undefined && value !== null && value !== "")
        .map(([key, value]) => `${key} : ${Array.isArray(value) ? value.join(", ") : String(value)}`),
    },
  ];
}

/**
 * Dossier complet d’une fiche pour le staff (PP, direction, secrétariat).
 * Le parent déposant doit pouvoir être relu avant l’avis du conseil : on renvoie
 * donc les réponses, signatures, pièces et historique de liens sans exposer les
 * secrets (token / code d’accès).
 */
export async function getFdFicheDossier(
  etablissementId: string,
  ficheId: string,
): Promise<FdFicheDossier | null> {
  const fiche = await getFdFiche(etablissementId, ficheId);
  if (!fiche) return null;
  const campagne = await getFdCampagne(etablissementId, fiche.campagneId);
  if (!campagne) return null;
  const etapes = await listFdEtapes(etablissementId, fiche.campagneId);
  const etapeById = new Map(etapes.map((e) => [e.id, e]));
  const db = getDb();

  const [reponses, signatures, archives, tokens] = await Promise.all([
    db
      .select()
      .from(fdReponse)
      .where(eq(fdReponse.ficheId, fiche.id))
      .orderBy(asc(fdReponse.submittedAt)),
    db
      .select({
        id: fdSignature.id,
        etapeId: fdSignature.etapeId,
        role: fdSignature.role,
        signerName: fdSignature.signerName,
        signerEmail: fdSignature.signerEmail,
        method: fdSignature.method,
        signedAt: fdSignature.signedAt,
      })
      .from(fdSignature)
      .where(eq(fdSignature.ficheId, fiche.id))
      .orderBy(asc(fdSignature.signedAt)),
    db
      .select({
        id: fdPdfArchive.id,
        ficheId: fdPdfArchive.ficheId,
        etapeId: fdPdfArchive.etapeId,
        kind: fdPdfArchive.kind,
        title: fdPdfArchive.title,
        s3Key: fdPdfArchive.s3Key,
        pdfBase64: fdPdfArchive.pdfBase64,
        createdAt: fdPdfArchive.createdAt,
      })
      .from(fdPdfArchive)
      .where(eq(fdPdfArchive.ficheId, fiche.id))
      .orderBy(desc(fdPdfArchive.createdAt)),
    db
      .select({
        id: fdToken.id,
        etapeId: fdToken.etapeId,
        email: fdToken.email,
        purpose: fdToken.purpose,
        expiresAt: fdToken.expiresAt,
        usedAt: fdToken.usedAt,
        revokedAt: fdToken.revokedAt,
        createdAt: fdToken.createdAt,
      })
      .from(fdToken)
      .where(eq(fdToken.ficheId, fiche.id))
      .orderBy(desc(fdToken.createdAt)),
  ]);

  return {
    fiche,
    campagne,
    etapes: etapes.map((e) => ({
      id: e.id,
      ordre: e.ordre,
      kind: e.kind,
      label: e.label,
      closesAt: e.closesAt?.toISOString() ?? null,
      gelee: e.gelee,
    })),
    reponses: reponses.map((r) => {
      const etape = etapeById.get(r.etapeId);
      const etapeKind = etape?.kind ?? "saisie_famille";
      return {
        id: r.id,
        etapeId: r.etapeId,
        etapeLabel: etape?.label ?? "Étape",
        etapeKind,
        auteurRole: r.auteurRole,
        auteurLabel: r.auteurLabel,
        submittedAt: r.submittedAt.toISOString(),
        payload: r.payload,
        sections: dossierSections(campagne, etapeKind, r.auteurRole, r.payload),
      };
    }),
    signatures: signatures.map((s) => ({
      ...s,
      signedAt: s.signedAt.toISOString(),
    })),
    archives: archives.map((a) => ({
      id: a.id,
      ficheId: a.ficheId,
      etapeId: a.etapeId,
      kind: a.kind,
      title: a.title,
      createdAt: a.createdAt.toISOString(),
      storage: a.s3Key ? "s3" : a.pdfBase64 ? "db" : "none",
    })),
    tokens: tokens.map((t) => ({
      id: t.id,
      etapeId: t.etapeId,
      email: t.email,
      purpose: t.purpose,
      expiresAt: t.expiresAt?.toISOString() ?? null,
      usedAt: t.usedAt?.toISOString() ?? null,
      revokedAt: t.revokedAt?.toISOString() ?? null,
      createdAt: t.createdAt.toISOString(),
    })),
  };
}

export type FdEleveFicheView = {
  id: string;
  campagneLabel: string;
  anneeLabel: string;
  statut: FdFicheStatut;
  etapeLabel: string | null;
  updatedAt: string;
  closedAt: string | null;
  reponses: Array<{
    id: string;
    etapeLabel: string;
    auteurRole: string;
    submittedAt: string;
    sections: FdPdfSection[];
  }>;
  appel: {
    decision: "maintien" | "reformation" | null;
    motif: string | null;
    destinationProposee: string | null;
    closedAt: string | null;
    notifiedAt: string | null;
  } | null;
};

/**
 * Retrouve l’élève rattaché à un compte utilisateur (e-mail élève, clé de fusion
 * ou INE). Les comptes élèves sont optionnels dans l’ENT ; l’absence de
 * correspondance ne doit jamais donner accès à la fiche d’un autre élève.
 */
export async function findFdEleveForUser(params: {
  etablissementId: string;
  email: string;
  externalUserId?: string | null;
}): Promise<{ id: string; prenom: string; nom: string; classe: string | null } | null> {
  const email = params.email.trim().toLowerCase();
  if (!email) return null;
  const external = params.externalUserId?.trim() || "";
  const db = getDb();
  // Recherche d'identité (e-mail / clé de fusion / INE), pas une liste : on ne
  // filtre donc pas sur les élèves actifs, sinon un élève sorti perdrait l'accès
  // en lecture seule à ses fiches.
  const row = await db.query.eleve.findFirst({
    columns: {
      id: true,
      prenom: true,
      nom: true,
      classe: true,
    },
    where: and(
      eq(eleve.etablissementId, params.etablissementId),
      or(
        sql`lower(${eleve.email}) = ${email}`,
        external ? eq(eleve.sourceKey, external) : undefined,
        external ? eq(eleve.ine, external) : undefined,
      ),
    ),
  });
  return row ?? null;
}

/** Vue strictement en lecture seule pour l’élève : aucun contact, jeton ni secret. */
export async function listFdEleveFiches(
  etablissementId: string,
  eleveId: string,
): Promise<FdEleveFicheView[]> {
  const db = getDb();
  const fiches = await db
    .select()
    .from(fdFiche)
    .where(and(eq(fdFiche.etablissementId, etablissementId), eq(fdFiche.eleveId, eleveId)))
    .orderBy(desc(fdFiche.createdAt));

  const views: FdEleveFicheView[] = [];
  for (const fiche of fiches) {
    const dossier = await getFdFicheDossier(etablissementId, fiche.id);
    if (!dossier) continue;
    const etape = dossier.etapes.find((e) => e.id === fiche.etapeCouranteId);
    views.push({
      id: fiche.id,
      campagneLabel: dossier.campagne.label,
      anneeLabel: dossier.campagne.anneeLabel,
      statut: fiche.statut,
      etapeLabel: etape?.label ?? null,
      updatedAt: fiche.updatedAt.toISOString(),
      closedAt: fiche.closedAt?.toISOString() ?? null,
      reponses: dossier.reponses.map((r) => ({
        id: r.id,
        etapeLabel: r.etapeLabel,
        auteurRole: r.auteurRole,
        submittedAt: r.submittedAt,
        sections: r.sections,
      })),
      appel: fiche.appel
        ? {
            decision: fiche.appel.decision ?? null,
            motif: fiche.appel.motif ?? null,
            destinationProposee: fiche.appel.destinationProposee ?? null,
            closedAt: fiche.appel.closedAt ?? null,
            notifiedAt: fiche.appel.notifiedAt ?? null,
          }
        : null,
    });
  }
  return views;
}
