import "server-only";

import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { loadOfficialSchoolClasses, type SchoolPole } from "@/app/lib/nomenclature-classes";

export type StageSiecleClassOption = {
  code: string;
  label: string;
  pole: Extract<SchoolPole, "COLLÈGE" | "LYCÉE">;
};

/** Classes collège + lycée importées SIECLE (hors école primaire). */
export async function listStageSiecleClassOptions(): Promise<StageSiecleClassOption[]> {
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return [];

  const official = await loadOfficialSchoolClasses(etabId);
  const out: StageSiecleClassOption[] = [];

  for (const code of official.lockedClasses) {
    const division = official.divisions.find((d) => d.code === code);
    const pole =
      official.lockedClassesByPole.COLLÈGE?.includes(code) ? "COLLÈGE" : "LYCÉE";
    out.push({
      code,
      label: division?.libelleLong || division?.libelleCourt || code,
      pole,
    });
  }

  return out.sort((a, b) => a.code.localeCompare(b.code, "fr", { sensitivity: "base" }));
}

export async function listStageSiecleClassCodes(): Promise<string[]> {
  const options = await listStageSiecleClassOptions();
  return options.map((o) => o.code);
}
