import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { tenantSettingAttr, tenantSettingSection } from "@/db/schema";

const SECTION = "eleve_scolarite_backfill";
const PATH_DONE = "courante_v1_done";

export async function isEleveScolariteBackfillDone(etablissementId: string): Promise<boolean> {
  const db = getDb();
  const [row] = await db
    .select({ value: tenantSettingAttr.value })
    .from(tenantSettingAttr)
    .where(
      and(
        eq(tenantSettingAttr.etablissementId, etablissementId),
        eq(tenantSettingAttr.section, SECTION),
        eq(tenantSettingAttr.path, PATH_DONE),
      ),
    )
    .limit(1);
  return row?.value === "1";
}

export async function markEleveScolariteBackfillDone(etablissementId: string): Promise<void> {
  const db = getDb();
  await db
    .insert(tenantSettingSection)
    .values({ etablissementId, section: SECTION, updatedAt: new Date() })
    .onConflictDoUpdate({
      target: [tenantSettingSection.etablissementId, tenantSettingSection.section],
      set: { updatedAt: new Date() },
    });
  await db
    .insert(tenantSettingAttr)
    .values({
      etablissementId,
      section: SECTION,
      path: PATH_DONE,
      value: "1",
    })
    .onConflictDoUpdate({
      target: [
        tenantSettingAttr.etablissementId,
        tenantSettingAttr.section,
        tenantSettingAttr.path,
      ],
      set: { value: "1" },
    });
}
