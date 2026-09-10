import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import { loadElevesRegistry, saveElevesRegistry } from "@/app/lib/eleves-registry";
import { listNomenclatureByType } from "@/app/lib/ref-nomenclature-db";
import { normMefCode } from "@/app/lib/mef-secteurs";

export type MefLabelMaps = {
  /** CODE_MEF normalisé → libellé affichable */
  codeToLabel: Map<string, string>;
  /** libellé / code normalisé → CODE_MEF officiel */
  labelToCode: Map<string, string>;
};

/** Construit les maps code ↔ libellé depuis Nomenclature.xml (type mef). */
const mefMapsCache = new Map<string, { maps: MefLabelMaps; expiresAt: number }>();
const MEF_MAPS_TTL_MS = 10 * 60_000;

export async function lookupMefLabel(
  etablissementId: string,
  raw: string | undefined | null,
): Promise<string> {
  const value = String(raw ?? "").trim();
  if (!value) return "";
  const maps = await buildMefLabelMaps(etablissementId);
  return resolveMefDisplayValue(value, maps) || value;
}

export async function buildMefLabelMaps(etablissementId: string): Promise<MefLabelMaps> {
  const cached = mefMapsCache.get(etablissementId);
  if (cached && cached.expiresAt > Date.now()) return cached.maps;

  const rows = await listNomenclatureByType(etablissementId, "mef");
  const codeToLabel = new Map<string, string>();
  const labelToCode = new Map<string, string>();

  for (const row of rows) {
    const code = String(row.code || "").trim();
    if (!code) continue;
    const label = String(row.libelleLong || row.libelleCourt || "").trim() || code;
    const codeKey = normMefCode(code);
    const labelKey = normMefCode(label);
    if (codeKey) {
      codeToLabel.set(codeKey, label);
      labelToCode.set(codeKey, code);
    }
    if (labelKey) labelToCode.set(labelKey, code);
    if (row.libelleCourt) {
      const shortKey = normMefCode(row.libelleCourt);
      if (shortKey) labelToCode.set(shortKey, code);
    }
  }

  const maps = { codeToLabel, labelToCode };
  mefMapsCache.set(etablissementId, { maps, expiresAt: Date.now() + MEF_MAPS_TTL_MS });
  return maps;
}

/** Remplace un CODE_MEF brut par son libellé Siècle quand disponible. */
export function resolveMefDisplayValue(
  raw: string | undefined | null,
  maps: MefLabelMaps,
): string {
  const value = String(raw ?? "").trim();
  if (!value) return "";
  const key = normMefCode(value);
  if (!key) return value;
  return maps.codeToLabel.get(key) || value;
}

/** CODE_MEF attendu à l'export (libellé → code, ou code inchangé). */
export function resolveMefExportCode(
  raw: string | undefined | null,
  maps: MefLabelMaps,
): string {
  const value = String(raw ?? "").trim();
  if (!value) return "";
  const key = normMefCode(value);
  if (!key) return value;
  return maps.labelToCode.get(key) || value;
}

export function enrichElevesMefLabels(
  eleves: EleveConfig[],
  maps: MefLabelMaps,
): { eleves: EleveConfig[]; enriched: number } {
  if (!maps.codeToLabel.size) return { eleves, enriched: 0 };
  let enriched = 0;
  const out = eleves.map((e) => {
    const raw = String(e.mef ?? e.formation ?? "").trim();
    if (!raw) return e;
    const next = resolveMefDisplayValue(raw, maps);
    if (!next || next === raw) return e;
    enriched += 1;
    return { ...e, mef: next };
  });
  return { eleves: out, enriched };
}

/**
 * Après import Nomenclature (MEF) : remplace les codes bruts déjà stockés
 * sur les élèves par les libellés (ex. « 2nde générale et technologique »).
 */
export async function enrichElevesRegistryMefFromNomenclature(
  etablissementId: string,
): Promise<{ enriched: number }> {
  const maps = await buildMefLabelMaps(etablissementId);
  if (!maps.codeToLabel.size) return { enriched: 0 };
  const registry = await loadElevesRegistry();
  if (!registry.length) return { enriched: 0 };
  const { eleves, enriched } = enrichElevesMefLabels(registry, maps);
  if (enriched > 0) await saveElevesRegistry(eleves);
  return { enriched };
}
