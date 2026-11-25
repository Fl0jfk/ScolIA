/**
 * Lien de parenté d'un responsable avec UN élève (le lien dépend du couple responsable / enfant).
 * Module pur (utilisable côté client et serveur).
 */
import { cleanText, normName } from "./normalize";

export const LIENS_PARENTE = [
  "Mère",
  "Père",
  "Beau-père",
  "Belle-mère",
  "Grand-mère",
  "Grand-père",
  "Oncle",
  "Tante",
  "Famille d'accueil",
  "Organisme (ASE…)",
  "Tuteur",
  "Autre",
] as const;

export type LienParente = (typeof LIENS_PARENTE)[number];

const MAX_LIEN = 60;

/** Libellé canonique si reconnu, sinon le texte nettoyé du fichier (tronqué). Vide → "". */
export function canonicalLien(raw: unknown): string {
  const text = cleanText(raw);
  if (!text) return "";
  const n = normName(text);
  for (const l of LIENS_PARENTE) if (normName(l) === n) return l;
  if (n.startsWith("organisme")) return "Organisme (ASE…)";
  if (n.startsWith("famille d accueil")) return "Famille d'accueil";
  if (n === "tutrice") return "Tuteur";
  return text.slice(0, MAX_LIEN);
}

/** Erreur Postgres « relation … does not exist » (migration non appliquée). */
export function isMissingRelationError(err: unknown): boolean {
  for (let e: unknown = err, i = 0; e && i < 4; i++) {
    const o = e as { code?: unknown; message?: unknown; cause?: unknown };
    if (o.code === "42P01") return true;
    if (typeof o.message === "string" && /relation .*does not exist/i.test(o.message)) return true;
    e = o.cause;
  }
  return false;
}
