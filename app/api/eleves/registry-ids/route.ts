import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve } from "@/db/schema";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

/** Map folderName → uuid pour ouvrir les fiches depuis le registry. */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ byFolder: {} });
  }
  const db = getDb();
  const rows = await db
    .select({ id: eleve.id, folderName: eleve.folderName, nom: eleve.nom, prenom: eleve.prenom })
    .from(eleve)
    .where(eq(eleve.etablissementId, etabId));
  const byFolder: Record<string, string> = {};
  for (const r of rows) {
    byFolder[r.folderName] = r.id;
    byFolder[`${r.nom}_${r.prenom}`] = r.id;
  }
  return NextResponse.json({ byFolder });
}
