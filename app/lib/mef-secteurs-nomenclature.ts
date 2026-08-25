import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { refNomenclature } from "@/db/schema";
import type { Secteur } from "@/app/lib/onedrive-eleves";
import { normMefCode } from "@/app/lib/mef-secteurs";

function inferSecteurFromMef(code: string, libelle: string): Secteur | null {
  const hay = `${code} ${libelle}`
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toUpperCase();

  if (
    /\b(2NDE|1RE|1ERE|TERMINALE|CAP|BP|BAC|PROFESSIONNEL|LYCEE|LYCEEN|SECONDE)\b/.test(hay) ||
    /\b\d{4}(001|002|003|010|011)\b/.test(hay)
  ) {
    return "lycee";
  }
  if (
    /\b(6E|5E|4E|3E|COLLEGE|COLLÈGE|CYCLE 4|CYCLE4|CYCLE 3|CYCLE3)\b/.test(hay) ||
    /\b\d{4}(100|200|300|400)\b/.test(hay)
  ) {
    return "college";
  }
  if (
    /\b(CP|CE1|CE2|CM1|CM2|MATERNELLE|ELEMENTAIRE|ÉLÉMENTAIRE|ECOLE|CYCLE 2|CYCLE2|CYCLE 1)\b/.test(
      hay,
    )
  ) {
    return "ecole";
  }
  return null;
}

/** MEF importés Siècle (ref_nomenclature) → secteur scolaire. */
export async function loadMefSecteurMapFromNomenclature(
  etablissementId: string,
): Promise<Map<string, Secteur>> {
  const db = getDb();
  const rows = await db
    .select({
      code: refNomenclature.code,
      libelleCourt: refNomenclature.libelleCourt,
      libelleLong: refNomenclature.libelleLong,
    })
    .from(refNomenclature)
    .where(
      and(eq(refNomenclature.etablissementId, etablissementId), eq(refNomenclature.type, "mef")),
    );

  const map = new Map<string, Secteur>();
  for (const row of rows) {
    const code = normMefCode(row.code);
    if (!code) continue;
    const libelle = row.libelleLong || row.libelleCourt || row.code;
    const secteur = inferSecteurFromMef(row.code, libelle);
    if (secteur) map.set(code, secteur);
  }
  return map;
}

export async function countMefNomenclature(etablissementId: string): Promise<number> {
  const map = await loadMefSecteurMapFromNomenclature(etablissementId);
  return map.size;
}
