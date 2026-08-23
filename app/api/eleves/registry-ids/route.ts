import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve } from "@/db/schema";
import { requireModule } from "@/app/lib/intranet-auth";
import { requireTenantId } from "@/app/lib/tenant-scope";

/** Map folderName → uuid pour ouvrir les fiches depuis le registry. */
export async function GET() {
  const gate = await requireModule("eleve-dossier");
  if (!gate.ok) return gate.response;

  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;

  const etabId = tenant.ctx.etablissementId;
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
