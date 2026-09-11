import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { requireTenantId } from "@/app/lib/tenant-scope";
import {
  canViewFullElevesDossierHub,
  isProfesseurScopedDossierViewer,
  listAssignedClassesForTeacher,
  teacherCanAccessEleveClasse,
} from "@/app/lib/eleve-dossier-prof";
import { PROFESSEUR_DOSSIER_SEE_ALL_CLASSES_TEMPORARY } from "@/app/lib/eleve-dossier-scope";
import { getElevePhotoUrl } from "@/app/lib/eleve-photos";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { eleve } from "@/db/schema";
import { and, eq } from "drizzle-orm";

type Ctx = { params: Promise<{ id: string }> };

/** Photo élève via proxy auth — évite de pré-signer N URLs sur la liste des dossiers. */
export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("eleve-dossier");
  if (!gate.ok) return gate.response;
  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;
  if (!isDatabaseConfigured()) {
    return NextResponse.json({ error: "Base indisponible." }, { status: 503 });
  }

  const { id: eleveId } = await ctx.params;
  const user = gate.ctx.user;
  const fullHub = canViewFullElevesDossierHub({
    roles: user.roles,
    orgAdmin: user.orgAdmin,
    platformAdmin: user.platformAdmin,
  });
  const profScoped = isProfesseurScopedDossierViewer({
    roles: user.roles,
    orgAdmin: user.orgAdmin,
    platformAdmin: user.platformAdmin,
  });
  if (!fullHub && !profScoped) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const db = getDb();
  const [row] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      ine: eleve.ine,
      photoKey: eleve.photoKey,
      folderName: eleve.folderName,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, tenant.ctx.etablissementId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!row) {
    return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
  }

  if (profScoped && !PROFESSEUR_DOSSIER_SEE_ALL_CLASSES_TEMPORARY) {
    const assignedClasses = await listAssignedClassesForTeacher(user.businessUserId);
    if (!teacherCanAccessEleveClasse(row.classe, assignedClasses)) {
      return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
    }
  }

  const url = await getElevePhotoUrl({
    nom: row.nom,
    prenom: row.prenom,
    ine: row.ine || "",
    photoKey: row.photoKey ?? undefined,
    folderName: row.folderName || "",
  });
  if (!url) {
    return NextResponse.json({ error: "Photo introuvable." }, { status: 404 });
  }
  return NextResponse.redirect(url);
}
