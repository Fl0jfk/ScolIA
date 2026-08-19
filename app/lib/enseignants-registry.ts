import "server-only";

import { buildEleveFolderName } from "@/app/lib/eleves-config";
import type { EnseignantConfig } from "@/app/lib/enseignants-types";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { getJson, putJson } from "@/app/lib/s3-storage";

export type { EnseignantConfig } from "@/app/lib/enseignants-types";

const ENSEIGNANTS_REGISTRY_KEY = "enseignants.json";

function newId() {
  return `ens_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

function asSecteur(value: unknown): Secteur | null {
  const s = String(value ?? "").trim().toLowerCase();
  return s === "ecole" || s === "college" || s === "lycee" ? s : null;
}

export function validateEnseignantsJson(
  raw: unknown,
): { ok: true; enseignants: EnseignantConfig[] } | { ok: false; error: string } {
  if (!Array.isArray(raw)) return { ok: false, error: "Liste enseignants invalide." };
  const enseignants: EnseignantConfig[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const row = item as Record<string, unknown>;
    const nom = String(row.nom ?? "").trim();
    const prenom = String(row.prenom ?? "").trim();
    const secteur = asSecteur(row.secteur);
    if (!nom || !prenom || !secteur) continue;
    enseignants.push({
      id: String(row.id ?? "").trim() || newId(),
      nom,
      prenom,
      folderName: String(row.folderName ?? "").trim() || buildEleveFolderName(nom, prenom),
      secteur,
      email: String(row.email ?? "").trim().toLowerCase() || undefined,
    });
  }
  return { ok: true, enseignants };
}

export async function loadEnseignantsRegistry(): Promise<EnseignantConfig[]> {
  const hit = await getJson<EnseignantConfig[]>(ENSEIGNANTS_REGISTRY_KEY);
  if (!Array.isArray(hit?.data)) return [];
  const validated = validateEnseignantsJson(hit.data);
  return validated.ok ? validated.enseignants : [];
}

export async function saveEnseignantsRegistry(
  enseignants: EnseignantConfig[],
): Promise<EnseignantConfig[]> {
  const validated = validateEnseignantsJson(enseignants);
  if (!validated.ok) throw new Error(validated.error);
  await putJson(ENSEIGNANTS_REGISTRY_KEY, validated.enseignants);
  return validated.enseignants;
}

export function filterEnseignantsForSecteurs(
  enseignants: EnseignantConfig[],
  secteurs: Secteur[],
): EnseignantConfig[] {
  if (secteurs.length === 0) return [];
  const set = new Set(secteurs);
  return enseignants.filter((e) => set.has(e.secteur));
}
