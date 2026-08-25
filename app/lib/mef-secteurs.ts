import { getJson, putJson } from "@/app/lib/s3-storage";
import type { Secteur } from "@/app/lib/onedrive-eleves";
import { isEntCoreDbEnabled, resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { loadMefSecteurMapFromNomenclature } from "@/app/lib/mef-secteurs-nomenclature";

export const MEF_SECTEURS_KEY = "settings/mef-secteurs.json";

export type MefSecteursConfig = {
  lycee: string[];
  college: string[];
  ecole: string[];
};

/** Codes MEF numériques ou libellés formation (ex. « Cycle 2 - COURS PREPARATOIRE »). */
export function normMefCode(raw: string): string {
  return String(raw ?? "")
    .trim()
    .toUpperCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^A-Z0-9]+/g, "");
}

export function parseMefSecteursConfig(
  data: unknown,
): { ok: true; config: MefSecteursConfig } | { ok: false; error: string } {
  if (!data || typeof data !== "object") {
    return { ok: false, error: "Le fichier doit être un objet JSON." };
  }
  const o = data as Record<string, unknown>;
  const pick = (key: keyof MefSecteursConfig): string[] => {
    const arr = o[key];
    if (!Array.isArray(arr)) return [];
    return arr
      .map((x) => normMefCode(String(x)))
      .filter(Boolean);
  };
  const config: MefSecteursConfig = {
    lycee: pick("lycee"),
    college: pick("college"),
    ecole: pick("ecole"),
  };
  const total = config.lycee.length + config.college.length + config.ecole.length;
  if (total === 0) {
    return {
      ok: false,
      error: "Ajoutez au moins un code ou libellé formation dans lycee, college ou ecole.",
    };
  }
  const seen = new Set<string>();
  for (const [secteur, codes] of Object.entries(config) as [Secteur, string[]][]) {
    for (const code of codes) {
      if (seen.has(code)) {
        return { ok: false, error: `Code MEF en double : ${code} (présent dans ${secteur} et ailleurs).` };
      }
      seen.add(code);
    }
  }
  return { ok: true, config };
}

function buildMefToSecteurMap(config: MefSecteursConfig): Map<string, Secteur> {
  const map = new Map<string, Secteur>();
  for (const code of config.lycee) map.set(code, "lycee");
  for (const code of config.college) map.set(code, "college");
  for (const code of config.ecole) map.set(code, "ecole");
  return map;
}

export async function loadMefSecteurMap(): Promise<Map<string, Secteur>> {
  const map = new Map<string, Secteur>();

  const hit = await getJson<MefSecteursConfig>(MEF_SECTEURS_KEY);
  if (hit?.data) {
    const parsed = parseMefSecteursConfig(hit.data);
    if (parsed.ok) {
      for (const [code, secteur] of buildMefToSecteurMap(parsed.config)) {
        map.set(code, secteur);
      }
    }
  }

  if (isEntCoreDbEnabled()) {
    try {
      const etabId = await resolveCurrentEtablissementId();
      if (etabId) {
        const nomMap = await loadMefSecteurMapFromNomenclature(etabId);
        for (const [code, secteur] of nomMap) {
          if (!map.has(code)) map.set(code, secteur);
        }
      }
    } catch (error) {
      console.error("[mef-secteurs] nomenclature", error);
    }
  }

  return map;
}

export async function saveMefSecteursConfig( config: MefSecteursConfig) {
  await putJson(MEF_SECTEURS_KEY, config);
}

export function countMefCodes(config: MefSecteursConfig) {
  return {
    lycee: config.lycee.length,
    college: config.college.length,
    ecole: config.ecole.length,
    total: config.lycee.length + config.college.length + config.ecole.length,
  };
}
