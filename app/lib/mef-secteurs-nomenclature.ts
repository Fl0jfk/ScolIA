import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { refNomenclature } from "@/db/schema";
import type { Secteur } from "@/app/lib/onedrive-eleves";
import { normMefCode } from "@/app/lib/mef-secteurs";
import { inferSecteurFromMef } from "@/app/lib/mef-secteur-inference";

export { inferSecteurFromMef } from "@/app/lib/mef-secteur-inference";

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
