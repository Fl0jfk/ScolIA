import "server-only";

import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { resolveEleveFolderName, type EleveConfig } from "@/app/lib/eleves-config";
import { loadMefSecteurMap } from "@/app/lib/mef-secteurs";
import { inferSecteurFromFolderName, resolveEleveSecteur } from "@/app/lib/onedrive-eleves";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import {
  canonicalClasseLabel,
  compareClassesForSort,
  dossierS3Key,
  indexS3Key,
  inferSecteurFromClasseLabel,
  slugPilotageKey,
  summaryFromDossier,
} from "@/app/lib/pilotage-eleves-logic";
import type {
  PilotageEleveDossier,
  PilotageEleveSummary,
  PilotageOverview,
  PilotageSecteurIndex,
} from "@/app/lib/pilotage-eleves-types";
import { getJson, putJson } from "@/app/lib/s3-storage";

export async function loadPilotageDossier(
  secteur: Secteur,
  key: string,
): Promise<PilotageEleveDossier | null> {
  const hit = await getJson<PilotageEleveDossier>(dossierS3Key(secteur, key));
  return hit?.data ?? null;
}

export async function savePilotageDossier(dossier: PilotageEleveDossier): Promise<void> {
  await putJson(dossierS3Key(dossier.secteur, dossier.key), dossier);
  const indexHit = await getJson<PilotageSecteurIndex>(indexS3Key(dossier.secteur));
  const index: PilotageSecteurIndex = indexHit?.data ?? {
    secteur: dossier.secteur,
    updatedAt: new Date().toISOString(),
    eleves: {},
  };
  index.eleves[dossier.key] = summaryFromDossier(dossier);
  index.updatedAt = new Date().toISOString();
  await putJson(indexS3Key(dossier.secteur), index);
}

export async function loadPilotageIndex(secteur: Secteur): Promise<PilotageSecteurIndex> {
  const hit = await getJson<PilotageSecteurIndex>(indexS3Key(secteur));
  return hit?.data ?? { secteur, updatedAt: "", eleves: {} };
}

export function elevePilotageKey(eleve: EleveConfig): string {
  return slugPilotageKey(eleve.ine, resolveEleveFolderName(eleve));
}

export async function listElevesForSecteurs(secteurs: Secteur[]): Promise<
  Array<EleveConfig & { secteur: Secteur; key: string }>
> {
  const mefMap = await loadMefSecteurMap();
  const all = await loadElevesRegistry();
  const allowed = new Set(secteurs);
  const out: Array<EleveConfig & { secteur: Secteur; key: string }> = [];
  for (const e of all) {
    const secteur =
      resolveEleveSecteur(e, mefMap) ||
      inferSecteurFromClasseLabel(e.classe) ||
      inferSecteurFromFolderName(e.folderName);
    if (!secteur || !allowed.has(secteur)) continue;
    out.push({
      ...e,
      secteur,
      key: elevePilotageKey(e),
      classe: canonicalClasseLabel(e.classe),
    });
  }
  return out.sort((a, b) => {
    const c = compareClassesForSort(a.classe ?? "", b.classe ?? "");
    if (c !== 0) return c;
    return `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" });
  });
}

function emptySummary(row: EleveConfig & { secteur: Secteur; key: string }): PilotageEleveSummary {
  return {
    key: row.key,
    nom: row.nom,
    prenom: row.prenom,
    classe: canonicalClasseLabel(row.classe),
    folderName: resolveEleveFolderName(row),
    emptyDossier: true,
    hasBulletin: false,
    hasPapPaiPps: false,
    dropSignal: false,
    mood: "neutral",
    signals: [],
  };
}

export async function buildPilotageOverview(
  secteurs: Secteur[],
  opts: { canWriteNotes: boolean; canIndex: boolean },
): Promise<PilotageOverview> {
  const eleves = await listElevesForSecteurs(secteurs);
  const indexes = new Map<Secteur, PilotageSecteurIndex>();
  for (const s of secteurs) {
    indexes.set(s, await loadPilotageIndex(s));
  }

  const byClass = new Map<
    string,
    { secteur: Secteur; classe: string; count: number; alerts: number; missingBulletin: number; drops: number }
  >();
  for (const e of eleves) {
    const classe = canonicalClasseLabel(e.classe);
    const k = `${e.secteur}::${classe}`;
    const indexed = indexes.get(e.secteur)?.eleves[e.key];
    const summary = indexed ?? emptySummary(e);
    const cur = byClass.get(k) ?? {
      secteur: e.secteur,
      classe,
      count: 0,
      alerts: 0,
      missingBulletin: 0,
      drops: 0,
    };
    cur.count += 1;
    if (summary.dropSignal || summary.emptyDossier) cur.alerts += 1;
    if (!summary.hasBulletin) cur.missingBulletin += 1;
    if (summary.dropSignal) cur.drops += 1;
    byClass.set(k, cur);
  }

  return {
    secteurs,
    classes: [...byClass.values()].sort((a, b) => {
      const s = a.secteur.localeCompare(b.secteur);
      if (s !== 0) return s;
      return compareClassesForSort(a.classe, b.classe);
    }),
    canWriteNotes: opts.canWriteNotes,
    canIndex: opts.canIndex,
  };
}

export async function listClassRoster(
  secteur: Secteur,
  classe: string,
  allowed: Secteur[],
): Promise<PilotageEleveSummary[]> {
  if (!allowed.includes(secteur)) return [];
  const eleves = await listElevesForSecteurs([secteur]);
  const wanted = canonicalClasseLabel(classe);
  const index = await loadPilotageIndex(secteur);
  const rows = eleves.filter((e) => canonicalClasseLabel(e.classe) === wanted);
  return Promise.all(
    rows.map(async (e) => {
      const dossier = await loadPilotageDossier(secteur, e.key);
      if (!dossier) return index.eleves[e.key] ?? emptySummary(e);
      return summaryFromDossier({
        ...dossier,
        nom: e.nom,
        prenom: e.prenom,
        classe: e.classe,
      });
    }),
  );
}

export async function findEleveRow(
  key: string,
  allowed: Secteur[],
): Promise<(EleveConfig & { secteur: Secteur; key: string }) | null> {
  const eleves = await listElevesForSecteurs(allowed);
  return eleves.find((e) => e.key === key) ?? null;
}

export async function appendPilotageAudit(entry: {
  userId: string;
  key: string;
  secteur: Secteur;
  classe?: string;
}): Promise<void> {
  const day = new Date().toISOString().slice(0, 10);
  const path = `pilotage/audit/${day}.json`;
  const hit = await getJson<Array<Record<string, string>>>(path);
  const rows = Array.isArray(hit?.data) ? hit.data : [];
  rows.push({
    at: new Date().toISOString(),
    userId: entry.userId,
    key: entry.key,
    secteur: entry.secteur,
    classe: entry.classe ?? "",
  });
  await putJson(path, rows);
}
