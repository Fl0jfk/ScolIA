import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { refNomenclature } from "@/db/schema";
import {
  listCompetenceDomaines,
  upsertCompetenceDomaine,
  upsertCompetenceItem,
} from "@/app/lib/notes-competences-db";

const DOMAIN_CODE = "SOCLE_SIECLE";

/** Alimente note_competence_* depuis ref_nomenclature (type programme). */
export async function syncProgrammesToCompetences(etablissementId: string): Promise<{
  domaines: number;
  items: number;
}> {
  const db = getDb();
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
      ),
    );

  if (!programmes.length) return { domaines: 0, items: 0 };

  const existing = await listCompetenceDomaines(etablissementId);
  let domaine = existing.find((d) => d.code === DOMAIN_CODE);
  if (!domaine) {
    domaine = await upsertCompetenceDomaine(etablissementId, {
      code: DOMAIN_CODE,
      libelle: "Programmes Siècle (nomenclature)",
      cycle: "college",
      ordre: 99,
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
