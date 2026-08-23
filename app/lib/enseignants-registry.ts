import "server-only";

import { buildEleveFolderName } from "@/app/lib/eleves-config";
import type { EnseignantConfig } from "@/app/lib/enseignants-types";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { enseignant } from "@/db/schema";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";

export type { EnseignantConfig } from "@/app/lib/enseignants-types";

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
      email: String(row.email ?? row.emailPerso ?? "").trim().toLowerCase() || undefined,
      emailPro: String(row.emailPro ?? "").trim().toLowerCase() || undefined,
    });
  }
  return { ok: true, enseignants };
}

function rowToConfig(row: typeof enseignant.$inferSelect): EnseignantConfig {
  return {
    id: row.id,
    nom: row.nom,
    prenom: row.prenom,
    folderName: row.folderName,
    secteur: row.secteur as Secteur,
    ...(row.email ? { email: row.email } : {}),
    ...(row.emailPro ? { emailPro: row.emailPro } : {}),
  };
}

export async function loadEnseignantsRegistry(): Promise<EnseignantConfig[]> {
  if (!isEntCoreDbEnabled()) return [];
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return [];
  const db = getDb();
  const rows = await db.select().from(enseignant).where(eq(enseignant.etablissementId, etabId));
  return rows.map(rowToConfig);
}

export async function saveEnseignantsRegistry(
  enseignants: EnseignantConfig[],
): Promise<EnseignantConfig[]> {
  const validated = validateEnseignantsJson(enseignants);
  if (!validated.ok) throw new Error(validated.error);
  if (!isEntCoreDbEnabled()) throw new Error("[enseignants] Postgres requis");
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) throw new Error("[enseignants] établissement introuvable");
  const db = getDb();
  await db.delete(enseignant).where(eq(enseignant.etablissementId, etabId));
  if (validated.enseignants.length > 0) {
    await db.insert(enseignant).values(
      validated.enseignants.map((e) => ({
        id: e.id,
        etablissementId: etabId,
        nom: e.nom,
        prenom: e.prenom,
        folderName: e.folderName,
        secteur: e.secteur,
        email: e.email ?? null,
        emailPro: e.emailPro ?? null,
        updatedAt: new Date(),
      })),
    );
  }
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

/** Une entrée par dossier OneDrive (évite collège + lycée = 2 homonymes). */
export function dedupeEnseignantsByFolder(enseignants: EnseignantConfig[]): EnseignantConfig[] {
  const byFolder = new Map<string, EnseignantConfig>();
  for (const e of enseignants) {
    const key = e.folderName.trim().toLowerCase() || `${e.nom}|${e.prenom}`.toLowerCase();
    if (!byFolder.has(key)) byFolder.set(key, e);
  }
  return [...byFolder.values()];
}
