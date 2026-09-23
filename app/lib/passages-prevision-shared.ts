/** Prévision repas — types & helpers purs (safe client + serveur). */

import { classifyRegime } from "@/app/lib/eleve-regime";
import type { MealDayKey } from "@/app/lib/eleve-grille-repas";

export const REPAS_SERVICES = ["midi", "soir"] as const;
export type RepasService = (typeof REPAS_SERVICES)[number];

export const REPAS_SERVICE_LABELS: Record<RepasService, string> = {
  midi: "Midi",
  soir: "Soir",
};

export type PrevisionSourceDroit = "grille" | "regime";

export type PrevisionStatut =
  | "attendu_pris"
  | "attendu_manquant"
  | "imprevu"
  | "en_sortie";

export const PREVISION_STATUT_LABELS: Record<PrevisionStatut, string> = {
  attendu_pris: "Attendu · pris",
  attendu_manquant: "Attendu · manquant",
  imprevu: "Imprévu (pris sans droit)",
  en_sortie: "En sortie scolaire",
};

export type PrevisionLigne = {
  eleveId: string;
  nom: string;
  prenom: string;
  classe: string | null;
  service: RepasService;
  droit: boolean;
  pris: boolean;
  statut: PrevisionStatut;
  sourceDroit: PrevisionSourceDroit | null;
};

export type PrevisionResume = {
  date: string;
  jourCle: MealDayKey | null;
  jourLabel: string | null;
  attendus: number;
  pris: number;
  manquants: number;
  imprevus: number;
  enSortie: number;
};

export type PrevisionRepasPayload = {
  resume: PrevisionResume;
  lignes: PrevisionLigne[];
};

/** Midi avant 16 h Paris ; soir à partir de 16 h. */
export function repasServiceFromHorodatage(iso: string): RepasService {
  const hourRaw = new Intl.DateTimeFormat("en-GB", {
    timeZone: "Europe/Paris",
    hour: "numeric",
    hour12: false,
  }).format(new Date(iso));
  const hour = Number.parseInt(hourRaw, 10);
  if (!Number.isFinite(hour)) return "midi";
  return hour < 16 ? "midi" : "soir";
}

/**
 * Droit repas déduit du régime (sans grille) — Lun–Ven uniquement côté appelant.
 * Interne → midi + soir ; DP / demiPension → midi ; sinon rien.
 */
export function droitRepasFromRegime(
  regime: string | null | undefined,
  demiPension?: boolean | null,
): { midi: boolean; soir: boolean } {
  const kind = classifyRegime(regime);
  if (kind === "interne") return { midi: true, soir: true };
  if (kind === "demi_pension" || demiPension === true) return { midi: true, soir: false };
  return { midi: false, soir: false };
}

export function statutPrevision(droit: boolean, pris: boolean): PrevisionStatut | null {
  if (droit && pris) return "attendu_pris";
  if (droit && !pris) return "attendu_manquant";
  if (!droit && pris) return "imprevu";
  return null;
}

export function resumeFromLignes(
  date: string,
  jourCle: MealDayKey | null,
  jourLabel: string | null,
  lignes: PrevisionLigne[],
): PrevisionResume {
  let attendus = 0;
  let pris = 0;
  let manquants = 0;
  let imprevus = 0;
  let enSortie = 0;
  for (const l of lignes) {
    if (l.statut === "en_sortie") enSortie += 1;
    if (l.droit) attendus += 1;
    if (l.pris) pris += 1;
    if (l.statut === "attendu_manquant") manquants += 1;
    if (l.statut === "imprevu") imprevus += 1;
  }
  return { date, jourCle, jourLabel, attendus, pris, manquants, imprevus, enSortie };
}
