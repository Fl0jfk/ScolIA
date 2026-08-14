import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import { validateElevesJson } from "@/app/lib/eleves-config";
import { getJson, putJson } from "@/app/lib/s3-storage";

/** Référentiel élèves unique du tenant — partagé par tous les modules. */
const ELEVES_REGISTRY_KEY = "eleves.json";

function normalizeName(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

export async function loadElevesRegistry(): Promise<EleveConfig[]> {
  const hit = await getJson<EleveConfig[]>(ELEVES_REGISTRY_KEY);
  if (!Array.isArray(hit?.data)) return [];
  const validated = validateElevesJson(hit.data);
  return validated.ok ? validated.eleves : [];
}

export async function saveElevesRegistry(eleves: EleveConfig[]): Promise<EleveConfig[]> {
  const validated = validateElevesJson(eleves);
  if (!validated.ok) throw new Error(validated.error);
  await putJson(ELEVES_REGISTRY_KEY, validated.eleves);
  return validated.eleves;
}

export async function countElevesRegistry(): Promise<number> {
  const eleves = await loadElevesRegistry();
  return eleves.length;
}

export async function findEleveByIne(ine: string): Promise<EleveConfig | null> {
  const key = ine.trim().toUpperCase();
  if (!key) return null;
  const eleves = await loadElevesRegistry();
  return eleves.find((e) => e.ine?.trim().toUpperCase() === key) ?? null;
}

type EleveNameMatch = {
  eleve: EleveConfig;
  score: number;
};

export function scoreEleveNameMatch(
  nom: string,
  prenom: string,
  candidate: EleveConfig,
): number {
  const an = normalizeName(nom);
  const ap = normalizeName(prenom);
  const bn = normalizeName(candidate.nom);
  const bp = normalizeName(candidate.prenom);
  let score = 0;
  if (an && bn && (an === bn || bn.includes(an) || an.includes(bn))) score += 2;
  if (ap && bp && (ap === bp || bp.includes(ap) || ap.includes(bp))) score += 2;
  return score;
}

/** Recherche fuzzy nom/prénom — utilisée par stages, OCR, certificats, etc. */
async function matchElevesByName(
  nom: string,
  prenom: string,
  opts?: { minScore?: number; limit?: number; classe?: string },
): Promise<EleveNameMatch[]> {
  const minScore = opts?.minScore ?? 3;
  const limit = opts?.limit ?? 5;
  const classeFilter = opts?.classe?.trim().toLowerCase() || "";
  const eleves = await loadElevesRegistry();
  return eleves
    .filter((e) => !classeFilter || String(e.classe || "").toLowerCase().includes(classeFilter))
    .map((eleve) => ({
      eleve,
      score: scoreEleveNameMatch(nom, prenom, eleve),
    }))
    .filter((m) => m.score >= minScore)
    .sort((a, b) => b.score - a.score)
    .slice(0, limit);
}

async function searchElevesRegistry(q: string, limit = 50): Promise<EleveConfig[]> {
  const query = q.trim().toLowerCase();
  const eleves = await loadElevesRegistry();
  if (!query) return eleves.slice(0, limit);
  return eleves
    .filter((e) => {
      const label = `${e.nom} ${e.prenom} ${e.classe || ""} ${e.ine || ""}`.toLowerCase();
      return label.includes(query);
    })
    .slice(0, limit);
}
