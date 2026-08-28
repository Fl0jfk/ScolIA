import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  loadOfficialSchoolClasses,
  mergeOfficialAndLocalClasses,
  mergeClassesByPoleWithSiecle,
  type SchoolPole,
} from "@/app/lib/nomenclature-classes";
import { resolveClassesByPoleCatalog } from "@/app/lib/school-classes-catalog";
import { listStageSiecleClassOptions } from "@/app/lib/stage-siecle-classes";

export type EstablishmentClassOption = {
  code: string;
  label: string;
  pole: Extract<SchoolPole, "COLLÈGE" | "LYCÉE"> | "ÉCOLE";
};

/** Liste unifiée des classes établissement (Siècle, élèves, planning, prof-room). */
export async function listEstablishmentClassNames(extra?: string[]): Promise<string[]> {
  const [config, eleves, etabId] = await Promise.all([
    loadAppConfig(),
    loadElevesRegistry(),
    resolveCurrentEtablissementId().catch(() => null),
  ]);

  const official = etabId ? await loadOfficialSchoolClasses(etabId) : null;

  let mergedByPole = resolveClassesByPoleCatalog(
    config.profRoom?.classesByPole,
    config.domainPlanning?.classesByPole,
  );

  if (official?.hasLockedSiecle) {
    mergedByPole = mergeClassesByPoleWithSiecle(official, mergedByPole);
  }

  const fromCatalog = Object.values(mergedByPole).flat();
  const fromEleves = [
    ...new Set(eleves.map((e) => String(e.classe ?? "").trim()).filter(Boolean)),
  ];

  if (official?.hasLockedSiecle) {
    return mergeOfficialAndLocalClasses(official, fromCatalog, fromEleves, extra ?? []);
  }

  return [...new Set([...fromCatalog, ...fromEleves, ...(extra ?? [])])].sort((a, b) =>
    a.localeCompare(b, "fr", { sensitivity: "base" }),
  );
}

function inferClassPole(className: string): EstablishmentClassOption["pole"] {
  const n = className.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  if (/^(cp|ce1|ce2|cm1|cm2|gs|ms|ps|tps)/.test(n)) return "ÉCOLE";
  if (/^(2nde|seconde|1re|1ere|premiere|terminale|tle|cap|bts)/.test(n)) return "LYCÉE";
  return "COLLÈGE";
}

/** Options classe pour sélecteurs Stages (Siècle + dossiers élèves + planning). */
export async function listEstablishmentClassOptions(): Promise<EstablishmentClassOption[]> {
  const [siecleOptions, names] = await Promise.all([
    listStageSiecleClassOptions(),
    listEstablishmentClassNames(),
  ]);

  const byCode = new Map<string, EstablishmentClassOption>();
  for (const option of siecleOptions) {
    byCode.set(option.code.toLowerCase(), option);
  }
  for (const name of names) {
    const key = name.toLowerCase();
    if (!byCode.has(key)) {
      byCode.set(key, { code: name, label: name, pole: inferClassPole(name) });
    }
  }

  return [...byCode.values()].sort((a, b) =>
    a.code.localeCompare(b.code, "fr", { sensitivity: "base" }),
  );
}
