import "server-only";

import { and, asc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { refNomenclature } from "@/db/schema";

export type NomenclatureEntry = {
  code: string;
  libelleCourt: string | null;
  libelleLong: string | null;
  metadataJson: Record<string, unknown> | null;
};

export async function listNomenclatureByType(
  etablissementId: string,
  type: string,
): Promise<NomenclatureEntry[]> {
  const db = getDb();
  return db
    .select({
      code: refNomenclature.code,
      libelleCourt: refNomenclature.libelleCourt,
      libelleLong: refNomenclature.libelleLong,
      metadataJson: refNomenclature.metadataJson,
    })
    .from(refNomenclature)
    .where(and(eq(refNomenclature.etablissementId, etablissementId), eq(refNomenclature.type, type)))
    .orderBy(asc(refNomenclature.code));
}

export async function listDivisionCodes(etablissementId: string): Promise<string[]> {
  const rows = await listNomenclatureByType(etablissementId, "division");
  return rows.map((r) => r.code);
}

export async function resolveDivisionLabel(
  etablissementId: string,
  code: string,
): Promise<string | null> {
  const db = getDb();
  const [row] = await db
    .select({ libelleLong: refNomenclature.libelleLong, libelleCourt: refNomenclature.libelleCourt })
    .from(refNomenclature)
    .where(
      and(
        eq(refNomenclature.etablissementId, etablissementId),
        eq(refNomenclature.type, "division"),
        eq(refNomenclature.code, code),
      ),
    )
    .limit(1);
  return row?.libelleLong || row?.libelleCourt || null;
}

export async function countNomenclatureByType(
  etablissementId: string,
  type: string,
): Promise<number> {
  const rows = await listNomenclatureByType(etablissementId, type);
  return rows.length;
}

export async function resolveNomenclatureLabel(
  etablissementId: string,
  type: string,
  code: string,
): Promise<string | null> {
  const db = getDb();
  const [row] = await db
    .select({ libelleLong: refNomenclature.libelleLong, libelleCourt: refNomenclature.libelleCourt })
    .from(refNomenclature)
    .where(
      and(
        eq(refNomenclature.etablissementId, etablissementId),
        eq(refNomenclature.type, type),
        eq(refNomenclature.code, code),
      ),
    )
    .limit(1);
  return row?.libelleLong || row?.libelleCourt || null;
}
