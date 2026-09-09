import "server-only";

import { and, eq, or } from "drizzle-orm";
import { getDb } from "@/db/index";
import { refNomenclature } from "@/db/schema";
import {
  listCompetenceDomaines,
  upsertCompetenceDomaine,
  upsertCompetenceItem,
} from "@/app/lib/notes-competences-db";
import {
  siecleCycleColumnValue,
  siecleCycleLabel,
  type SiecleImportCycle,
} from "@/app/lib/nomenclature-import/siecle-import-cycle";

function domaineCodeForCycle(cycle: SiecleImportCycle | undefined): string {
  if (cycle === "lycee") return "SOCLE_SIECLE_LYCEE";
  if (cycle === "college") return "SOCLE_SIECLE_COLLEGE";
  return "SOCLE_SIECLE";
}

/** Alimente note_competence_* depuis ref_nomenclature (type programme). */
export async function syncProgrammesToCompetences(
  etablissementId: string,
  opts?: { cycle?: SiecleImportCycle },
): Promise<{
  domaines: number;
  items: number;
}> {
  const db = getDb();
  const cycle = opts?.cycle;
  const cycleCol = siecleCycleColumnValue(cycle);
  const cycleFilter = cycle
    ? or(eq(refNomenclature.cycle, cycleCol), eq(refNomenclature.cycle, ""))
    : undefined;

  const programmes = await db
    .select({
      code: refNomenclature.code,
      libelle: refNomenclature.libelleLong,
      libelleCourt: refNomenclature.libelleCourt,
    })
    .from(refNomenclature)
    .where(
      and(
        eq(refNomenclature.etablissementId, etablissementId),
        eq(refNomenclature.type, "programme"),
        ...(cycleFilter ? [cycleFilter] : []),
      ),
    );

  if (!programmes.length) return { domaines: 0, items: 0 };

  const domainCode = domaineCodeForCycle(cycle);
  const existing = await listCompetenceDomaines(etablissementId);
  let domaine = existing.find((d) => d.code === domainCode);
  if (!domaine) {
    domaine = await upsertCompetenceDomaine(etablissementId, {
      code: domainCode,
      libelle: cycle
        ? `Programmes Siècle — ${siecleCycleLabel(cycle)}`
        : "Programmes Siècle (nomenclature)",
      cycle: cycle === "lycee" ? "lycee" : "college",
      ordre: cycle === "lycee" ? 100 : 99,
    });
  }

  let items = 0;
  for (let i = 0; i < programmes.length; i += 1) {
    const p = programmes[i];
    const libelle = p.libelle || p.libelleCourt || p.code;
    await upsertCompetenceItem(etablissementId, {
      domaineId: domaine.id,
      code: p.code.slice(0, 40),
      libelle: libelle.slice(0, 240),
      ordre: i + 1,
    });
    items += 1;
  }

  return { domaines: 1, items };
}
