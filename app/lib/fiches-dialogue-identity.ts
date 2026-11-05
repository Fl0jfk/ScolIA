import "server-only";

import { createHmac, randomBytes } from "node:crypto";

import { and, eq, inArray, or } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, fdFiche, fdCampagne } from "@/db/schema";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import { collectEleveParentEmails, isValidParentEmail, normalizeParentEmail } from "@/app/lib/eleves-parent-emails";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";

function normalizePersonPart(value: string): string {
  return String(value || "")
    .trim()
    .toUpperCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

export function maskEmail(email: string): string {
  const e = normalizeParentEmail(email);
  const at = e.indexOf("@");
  if (at <= 0) return "••••@••••";
  const local = e.slice(0, at);
  const domain = e.slice(at + 1);
  const visible = local.slice(0, 1);
  return `${visible}••••@${domain}`;
}

export type FdIdentifyMatch = {
  ficheId: string;
  campagneId: string;
  campagneLabel: string;
  eleveNom: string;
  elevePrenom: string;
  classeActuelle: string;
  dateNaissance: string | null;
  optionsActuelles: string[];
  photoKey: string | null;
  /** Adresses masquées + référence opaque (jamais l’adresse réelle côté navigateur). */
  maskedEmails: Array<{ ref: string; masked: string }>;
  needsClass: boolean;
  classes?: string[];
};

/**
 * Identification publique : nom + prénom + date de naissance
 * parmi les fiches d’une campagne active (ou toutes les actives).
 */
export async function identifyFdFicheByIdentity(params: {
  etablissementId: string;
  nom: string;
  prenom: string;
  dateNaissance: string;
  classe?: string;
  campagneId?: string;
}): Promise<
  | { ok: true; match: FdIdentifyMatch }
  | { ok: true; needsClass: true; classes: string[] }
  | { ok: false; reason: "not_found" | "ambiguous" }
> {
  const dob = normalizeEleveDateNaissance(params.dateNaissance);
  if (!dob) return { ok: false, reason: "not_found" };

  const pk = `${normalizePersonPart(params.nom)}§${normalizePersonPart(params.prenom)}`;
  const db = getDb();

  const campagneFilter = params.campagneId
    ? and(
        eq(fdCampagne.etablissementId, params.etablissementId),
        eq(fdCampagne.id, params.campagneId),
      )
    : and(
        eq(fdCampagne.etablissementId, params.etablissementId),
        or(eq(fdCampagne.statut, "active"), eq(fdCampagne.statut, "brouillon")),
      );

  const campagnes = await db.select().from(fdCampagne).where(campagneFilter);
  if (!campagnes.length) return { ok: false, reason: "not_found" };

  const campagneIds = campagnes.map((c) => c.id);
  const fiches = await db
    .select()
    .from(fdFiche)
    .where(
      and(
        eq(fdFiche.etablissementId, params.etablissementId),
        inArray(fdFiche.campagneId, campagneIds),
      ),
    );

  const registry = await loadElevesRegistry();
  const byId = new Map(registry.filter((e) => e.id).map((e) => [e.id!, e]));

  type Cand = {
    fiche: (typeof fiches)[0];
    campagne: (typeof campagnes)[0];
    eleve: EleveConfig | null;
  };
  const candidates: Cand[] = [];

  for (const fiche of fiches) {
    const fichePk = `${normalizePersonPart(fiche.eleveNom)}§${normalizePersonPart(fiche.elevePrenom)}`;
    if (fichePk !== pk) continue;
    const ficheDob = fiche.eleveDateNaissance
      ? String(fiche.eleveDateNaissance).slice(0, 10)
      : "";
    const reg = byId.get(fiche.eleveId) ?? null;
    const regDob = reg?.dateNaissance?.trim() || "";
    const dobOk = ficheDob === dob || regDob === dob;
    if (!dobOk) continue;
    const campagne = campagnes.find((c) => c.id === fiche.campagneId);
    if (!campagne) continue;
    candidates.push({ fiche, campagne, eleve: reg });
  }

  if (!candidates.length) {
    // Repli : match registry puis fiche par eleveId
    const regMatches = registry.filter((e) => {
      const epk = `${normalizePersonPart(e.nom)}§${normalizePersonPart(e.prenom)}`;
      return epk === pk && (e.dateNaissance?.trim() || "") === dob;
    });
    for (const reg of regMatches) {
      if (!reg.id) continue;
      const fiche = fiches.find((f) => f.eleveId === reg.id);
      if (!fiche) continue;
      const campagne = campagnes.find((c) => c.id === fiche.campagneId);
      if (!campagne) continue;
      candidates.push({ fiche, campagne, eleve: reg });
    }
  }

  if (!candidates.length) return { ok: false, reason: "not_found" };

  if (candidates.length > 1 && !params.classe?.trim()) {
    const classes = [
      ...new Set(candidates.map((c) => c.fiche.classeActuelle).filter(Boolean)),
    ].sort();
    if (classes.length > 1) {
      return { ok: true, needsClass: true, classes };
    }
  }

  let chosen = candidates[0];
  if (params.classe?.trim()) {
    const wanted = params.classe.trim().toUpperCase();
    const hit = candidates.find(
      (c) => c.fiche.classeActuelle.trim().toUpperCase() === wanted,
    );
    if (!hit) return { ok: false, reason: "not_found" };
    chosen = hit;
  } else if (candidates.length > 1) {
    return { ok: false, reason: "ambiguous" };
  }

  const emails = [
    ...new Set(
      [
        ...(chosen.fiche.parentEmails ?? []),
        ...(chosen.eleve ? collectEleveParentEmails(chosen.eleve) : []),
      ]
        .map(normalizeParentEmail)
        .filter(isValidParentEmail),
    ),
  ];

  const optionsActuelles = [
    ...new Set([
      ...(chosen.fiche.optionsActuelles ?? []),
      ...(chosen.eleve?.lv1 ? [`LV1 ${chosen.eleve.lv1}`] : []),
      ...(chosen.eleve?.lv2 ? [`LV2 ${chosen.eleve.lv2}`] : []),
      ...(chosen.eleve?.options ?? []),
    ]),
  ];

  return {
    ok: true,
    match: {
      ficheId: chosen.fiche.id,
      campagneId: chosen.campagne.id,
      campagneLabel: chosen.campagne.label,
      eleveNom: chosen.fiche.eleveNom,
      elevePrenom: chosen.fiche.elevePrenom,
      classeActuelle: chosen.fiche.classeActuelle,
      dateNaissance:
        (chosen.fiche.eleveDateNaissance
          ? String(chosen.fiche.eleveDateNaissance).slice(0, 10)
          : null) ||
        chosen.eleve?.dateNaissance ||
        dob,
      optionsActuelles,
      photoKey: chosen.fiche.elevePhotoKey || chosen.eleve?.photoKey || null,
      maskedEmails: emails.map((email) => ({
        ref: fdParentEmailRef(chosen.fiche.id, email),
        masked: maskEmail(email),
      })),
      needsClass: false,
    },
  };
}

export async function assertFdParentEmailOnFiche(params: {
  etablissementId: string;
  ficheId: string;
  email: string;
}): Promise<boolean> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(fdFiche)
    .where(and(eq(fdFiche.etablissementId, params.etablissementId), eq(fdFiche.id, params.ficheId)))
    .limit(1);
  if (!row) return false;
  const wanted = normalizeParentEmail(params.email);
  const known = (row.parentEmails ?? []).map(normalizeParentEmail);
  if (known.includes(wanted)) return true;
  const [el] = await db
    .select()
    .from(eleve)
    .where(and(eq(eleve.etablissementId, params.etablissementId), eq(eleve.id, row.eleveId)))
    .limit(1);
  if (!el) return false;
  return collectEleveParentEmails({
    ine: el.ine ?? "",
    nom: el.nom,
    prenom: el.prenom,
    folderName: el.folderName,
    parentEmail: el.parentEmail ?? undefined,
    parent1Email: el.parent1Email ?? undefined,
    parent2Email: el.parent2Email ?? undefined,
  }).some((e) => normalizeParentEmail(e) === wanted);
}

/** Clé de repli propre au processus si aucun secret serveur n’est configuré. */
const FALLBACK_REF_KEY = randomBytes(32);

function fdEmailRefKey(): string | Buffer {
  return (
    process.env.FD_PARENT_EMAIL_REF_SECRET?.trim() ||
    process.env.BETTER_AUTH_SECRET?.trim() ||
    process.env.AUTH_SECRET?.trim() ||
    FALLBACK_REF_KEY
  );
}

/**
 * Référence opaque d’une adresse parent pour une fiche : l’adresse ne quitte pas le serveur.
 * HMAC avec un secret serveur : impossible de tester hors ligne une adresse devinée.
 */
export function fdParentEmailRef(ficheId: string, email: string): string {
  return createHmac("sha256", fdEmailRefKey())
    .update(`fd-parent-email:${ficheId}:${normalizeParentEmail(email)}`)
    .digest("hex")
    .slice(0, 32);
}

/** Retrouve l’adresse réelle derrière une référence (fiche + fiche élève en base), ou null. */
export async function findFdParentEmailByRef(params: {
  etablissementId: string;
  ficheId: string;
  ref: string;
}): Promise<string | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(fdFiche)
    .where(and(eq(fdFiche.etablissementId, params.etablissementId), eq(fdFiche.id, params.ficheId)))
    .limit(1);
  if (!row) return null;
  const [el] = await db
    .select()
    .from(eleve)
    .where(and(eq(eleve.etablissementId, params.etablissementId), eq(eleve.id, row.eleveId)))
    .limit(1);
  const fromEleve = el
    ? collectEleveParentEmails({
        ine: el.ine ?? "",
        nom: el.nom,
        prenom: el.prenom,
        folderName: el.folderName,
        parentEmail: el.parentEmail ?? undefined,
        parent1Email: el.parent1Email ?? undefined,
        parent2Email: el.parent2Email ?? undefined,
      })
    : [];
  const candidates = [...(row.parentEmails ?? []), ...fromEleve]
    .map(normalizeParentEmail)
    .filter(isValidParentEmail);
  return candidates.find((e) => fdParentEmailRef(params.ficheId, e) === params.ref) ?? null;
}
