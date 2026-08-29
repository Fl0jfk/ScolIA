import "server-only";

import { and, asc, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  fdCampagne,
  fdEtape,
  fdFiche,
  fdReponse,
  fdSignature,
  type FdAcceptationPayload,
  type FdAppelConfig,
  type FdCalendrierMode,
  type FdCampagneRow,
  type FdCatalogueChoix,
  type FdConseilDecisionPayload,
  type FdEtapeKind,
  type FdEtapeRow,
  type FdFicheRow,
  type FdReponsePayload,
} from "@/db/schema";
import { collectEleveParentEmails, isValidParentEmail } from "@/app/lib/eleves-parent-emails";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { fileFicheDialoguePdfToDossier } from "@/app/lib/fiches-dialogue-filing";
import {
  notifyFdAcceptationRequest,
  notifyFdAppelProcedure,
  notifyFdDecisionPdf,
  notifyFdFamilleSaisie,
} from "@/app/lib/fiches-dialogue-notify";
import {
  buildFicheDialoguePdf,
  sectionsFromAcceptation,
  sectionsFromConseil,
  sectionsFromFamilleReponse,
} from "@/app/lib/fiches-dialogue-pdf";
import { getFdTemplate } from "@/app/lib/fiches-dialogue-templates";
import {
  createFdAccessToken,
  revokeFdTokensForFiche,
} from "@/app/lib/fiches-dialogue-tokens";

function now() {
  return new Date();
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
  templateKey: string;
  label: string;
  anneeLabel: string;
  anneeScolaireId?: string | null;
  siteKey?: string | null;
  classesCibles?: string[];
  delaiFamilleJours?: number;
  appelConfig?: FdAppelConfig;
  catalogueOverride?: FdCatalogueChoix;
  createdByUserId?: string | null;
}): Promise<{ campagne: FdCampagneRow; etapes: FdEtapeRow[] }> {
  const template = getFdTemplate(params.templateKey);
  if (!template) throw new Error("TEMPLATE_UNKNOWN");

  const db = getDb();
  const [campagne] = await db
    .insert(fdCampagne)
    .values({
      etablissementId: params.etablissementId,
      anneeScolaireId: params.anneeScolaireId ?? null,
      label: params.label.trim(),
      anneeLabel: params.anneeLabel.trim(),
      siteKey: params.siteKey ?? null,
      calendrierMode: template.calendrierMode as FdCalendrierMode,
      templateKey: template.key,
      statut: "brouillon",
      catalogue: params.catalogueOverride ?? template.catalogue,
      appelConfig: params.appelConfig ?? { enabled: true },
      delaiFamilleJours: params.delaiFamilleJours ?? 7,
      classesCibles: params.classesCibles ?? [],
      createdByUserId: params.createdByUserId ?? null,
    })
    .returning();

  const etapes: FdEtapeRow[] = [];
  for (let i = 0; i < template.etapes.length; i++) {
    const def = template.etapes[i];
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
      })
      .returning();
    etapes.push(etape);
  }

  return { campagne, etapes };
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
    calendrierMode: FdCalendrierMode;
  }>,
): Promise<FdCampagneRow> {
  const db = getDb();
  const [row] = await db
    .update(fdCampagne)
    .set({ ...patch, updatedAt: now() })
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
): Promise<{ created: number; skipped: number }> {
  const campagne = await getFdCampagne(etablissementId, campagneId);
  if (!campagne) throw new Error("CAMPAGNE_NOT_FOUND");
  const etapes = await listFdEtapes(etablissementId, campagneId);
  const firstEtape = etapes.find((e) => !e.optionnelle) ?? etapes[0];
  if (!firstEtape) throw new Error("NO_ETAPES");

  const registry = await loadElevesRegistry();
  const db = getDb();
  const elevesDb = await db
    .select()
    .from(eleve)
    .where(eq(eleve.etablissementId, etablissementId));

  const byIne = new Map(elevesDb.map((e) => [e.ine?.toUpperCase() ?? "", e]));
  const byId = new Map(elevesDb.map((e) => [e.id, e]));

  let created = 0;
  let skipped = 0;

  for (const reg of registry) {
    const classe = String(reg.classe || "");
    if (!matchClasse(classe, campagne.classesCibles ?? [])) {
      skipped += 1;
      continue;
    }
    const row =
      (reg.id && byId.get(reg.id)) ||
      (reg.ine ? byIne.get(reg.ine.toUpperCase()) : undefined);
    if (!row) {
      skipped += 1;
      continue;
    }

    const emails = collectEleveParentEmails(reg).filter(isValidParentEmail);
    try {
      await db.insert(fdFiche).values({
        etablissementId,
        campagneId,
        eleveId: row.id,
        eleveNom: row.nom || reg.nom,
        elevePrenom: row.prenom || reg.prenom,
        classeActuelle: classe || row.classe || "",
        optionsActuelles: [],
        parentEmails: emails,
        statut: "a_envoyer",
        etapeCouranteId: firstEtape.id,
      });
      created += 1;
    } catch {
      skipped += 1;
    }
  }

  return { created, skipped };
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

export async function sendFdFicheToFamille(params: {
  etablissementId: string;
  ficheId: string;
  reminder?: boolean;
}): Promise<{ ok: true; sent: boolean } | { ok: false; error: string }> {
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

  const emails = (fiche.parentEmails ?? []).filter(isValidParentEmail);
  if (!emails.length) return { ok: false, error: "Aucun e-mail parent renseigné." };

  await revokeFdTokensForFiche(fiche.id, "saisie");
  const tokenRow = await createFdAccessToken({
    etablissementId: params.etablissementId,
    ficheId: fiche.id,
    etapeId: etape.id,
    email: emails[0],
    purpose: etape.kind === "acceptation_famille" ? "acceptation" : "saisie",
    expiresInDays: Math.max(campagne.delaiFamilleJours, 14),
  });

  const notify =
    etape.kind === "acceptation_famille"
      ? await notifyFdAcceptationRequest({
          to: emails,
          elevePrenom: fiche.elevePrenom,
          eleveNom: fiche.eleveNom,
          token: tokenRow.token,
          secureCode: tokenRow.secureCode ?? "",
        })
      : await notifyFdFamilleSaisie({
          to: emails,
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

  const db = getDb();
  await db
    .update(fdFiche)
    .set({
      statut: etape.kind === "acceptation_famille" ? "en_attente_acceptation" : "en_attente_famille",
      lastSentAt: now(),
      lastReminderAt: params.reminder ? now() : fiche.lastReminderAt,
      reminderCount: params.reminder ? fiche.reminderCount + 1 : fiche.reminderCount,
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
  let sent = 0;
  let failed = 0;
  const errors: string[] = [];
  for (const fiche of fiches) {
    if (params.onlyMissing || params.reminder) {
      const waiting =
        fiche.statut === "a_envoyer" ||
        fiche.statut === "en_attente_famille" ||
        fiche.statut === "en_attente_acceptation";
      if (!waiting) continue;
    }
    const res = await sendFdFicheToFamille({
      etablissementId: params.etablissementId,
      ficheId: fiche.id,
      reminder: params.reminder,
    });
    if (res.ok) sent += 1;
    else {
      failed += 1;
      errors.push(`${fiche.elevePrenom} ${fiche.eleveNom}: ${res.error}`);
    }
  }
  const campagne = await getFdCampagne(params.etablissementId, params.campagneId);
  if (campagne && campagne.statut === "brouillon") {
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
}): Promise<{ ok: true } | { ok: false; error: string }> {
  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  const etape = await getEtape(params.etablissementId, params.etapeId);
  if (!etape) return { ok: false, error: "Étape introuvable." };
  if (etape.gelee) return { ok: false, error: "Étape figée : modification impossible." };
  if (etape.kind !== "saisie_famille" && etape.kind !== "choix_definitifs") {
    return { ok: false, error: "Cette étape n’accepte pas une saisie de vœux." };
  }
  if (fiche.etapeCouranteId !== etape.id) {
    return { ok: false, error: "Cette étape n’est plus la étape courante." };
  }

  const campagne = await getFdCampagne(params.etablissementId, fiche.campagneId);
  if (!campagne) return { ok: false, error: "Campagne introuvable." };

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

  await db
    .update(fdFiche)
    .set({ statut: "saisie_recue", updatedAt: now() })
    .where(eq(fdFiche.id, fiche.id));

  await revokeFdTokensForFiche(fiche.id, "saisie");
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
}): Promise<{ ok: true; pdfBytes: Uint8Array } | { ok: false; error: string }> {
  const fiche = await getFdFiche(params.etablissementId, params.ficheId);
  if (!fiche) return { ok: false, error: "Fiche introuvable." };
  const etape = await getEtape(params.etablissementId, params.etapeId);
  if (!etape) return { ok: false, error: "Étape introuvable." };
  if (!isConseilEtape(etape.kind)) {
    return { ok: false, error: "Cette étape n’est pas une étape de conseil." };
  }
  const campagne = await getFdCampagne(params.etablissementId, fiche.campagneId);
  if (!campagne) return { ok: false, error: "Campagne introuvable." };

  const db = getDb();
  await db
    .insert(fdReponse)
    .values({
      etablissementId: params.etablissementId,
      ficheId: fiche.id,
      etapeId: etape.id,
      auteurRole: "conseil",
      auteurUserId: params.auteurUserId ?? null,
      auteurLabel: params.auteurLabel ?? "Conseil de classe",
      payload: params.payload,
    })
    .onConflictDoUpdate({
      target: [fdReponse.ficheId, fdReponse.etapeId, fdReponse.auteurRole],
      set: { payload: params.payload, submittedAt: now(), auteurLabel: params.auteurLabel },
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

  const history = await loadFicheHistorySections(params.etablissementId, fiche, campagne.catalogue);
  const sections = [
    ...history,
    ...sectionsFromConseil(campagne.catalogue, params.payload),
  ];
  const pdfBytes = await buildFicheDialoguePdf({
    title:
      etape.kind === "decision_finale_conseil"
        ? "Décision définitive du conseil de classe"
        : "Avis du conseil de classe",
    campagneLabel: campagne.label,
    anneeLabel: campagne.anneeLabel,
    eleveNom: fiche.eleveNom,
    elevePrenom: fiche.elevePrenom,
    classeActuelle: fiche.classeActuelle,
    etapeLabel: etape.label,
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
    kind: etape.kind,
    title: `Fiche de dialogue — ${etape.label}`,
    pdfBytes,
    anneeLabel: campagne.anneeLabel,
    filedByUserId: params.auteurUserId,
  });

  const emails = (fiche.parentEmails ?? []).filter(isValidParentEmail);
  await notifyFdDecisionPdf({
    to: emails,
    elevePrenom: fiche.elevePrenom,
    eleveNom: fiche.eleveNom,
    etapeLabel: etape.label,
    pdfBytes,
    fileName: `fiche-dialogue-${etape.kind}.pdf`,
    intro:
      etape.kind === "decision_finale_conseil"
        ? "Voici la décision définitive du conseil de classe concernant votre enfant."
        : "Voici l’avis du conseil de classe concernant la fiche de dialogue de votre enfant.",
  });

  const etapes = await listFdEtapes(params.etablissementId, fiche.campagneId);
  const next = findNextEtape(etapes, etape, { activateAppel: false });

  if (etape.kind === "decision_finale_conseil") {
    const acceptEtape = etapes.find((e) => e.kind === "acceptation_famille");
    await db
      .update(fdFiche)
      .set({
        statut: "en_attente_acceptation",
        etapeCouranteId: acceptEtape?.id ?? next?.id ?? etape.id,
        updatedAt: now(),
      })
      .where(eq(fdFiche.id, fiche.id));
    if (acceptEtape) {
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
  }

  return { ok: true, pdfBytes };
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
  if (etape.gelee) return { ok: false, error: "Étape figée." };

  const campagne = await getFdCampagne(params.etablissementId, fiche.campagneId);
  if (!campagne) return { ok: false, error: "Campagne introuvable." };

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
    if (campagne.appelConfig?.enabled) {
      await notifyFdAppelProcedure({
        to: emails,
        elevePrenom: fiche.elevePrenom,
        eleveNom: fiche.eleveNom,
        appel: campagne.appelConfig,
        pdfBytes,
      });
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

export async function getFdPublicContext(token: string) {
  const { resolveFdToken } = await import("@/app/lib/fiches-dialogue-tokens");
  const tokenRow = await resolveFdToken(token);
  if (!tokenRow) return null;

  const fiche = await getFdFiche(tokenRow.etablissementId, tokenRow.ficheId);
  if (!fiche) return null;
  const campagne = await getFdCampagne(tokenRow.etablissementId, fiche.campagneId);
  if (!campagne) return null;
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
  };
}

export async function advanceCampagneToConseil(params: {
  etablissementId: string;
  campagneId: string;
  fromEtapeId: string;
}): Promise<{ advanced: number }> {
  const etapes = await listFdEtapes(params.etablissementId, params.campagneId);
  const from = etapes.find((e) => e.id === params.fromEtapeId);
  if (!from) throw new Error("ETAPE_NOT_FOUND");
  await freezeFdEtape({ etablissementId: params.etablissementId, etapeId: from.id });

  const nextConseil = etapes.find(
    (e) => e.ordre > from.ordre && isConseilEtape(e.kind),
  );
  if (!nextConseil) return { advanced: 0 };

  const db = getDb();
  const fiches = await listFdFiches(params.etablissementId, params.campagneId);
  let advanced = 0;
  for (const fiche of fiches) {
    if (fiche.statut === "cloturee" || fiche.statut === "acceptee" || fiche.statut === "refusee") {
      continue;
    }
    await db
      .update(fdFiche)
      .set({
        etapeCouranteId: nextConseil.id,
        statut: "en_conseil",
        updatedAt: now(),
      })
      .where(eq(fdFiche.id, fiche.id));
    advanced += 1;
  }
  return { advanced };
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
