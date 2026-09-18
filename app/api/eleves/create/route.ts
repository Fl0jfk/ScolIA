import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getAppSession } from "@/app/lib/intranet-session";
import { canManageElevePreinscriptions } from "@/app/lib/eleve-dossier-scope";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import { createElevePreinscrit } from "@/app/lib/eleve-create-preinscrit";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";

/**
 * Création manuelle d’un dossier élève `preinscrit`
 * (nom, prénom, e-mail parent — pour matching RDV inscription).
 */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const session = await getAppSession();
  if (!session?.user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }

  const roles =
    session.user.roles.length > 0
      ? session.user.roles
      : await listUserRolesFromDb(session.user.id, etabId);

  if (
    !canManageElevePreinscriptions({
      roles,
      orgAdmin: session.user.orgAdmin,
      platformAdmin: session.user.platformAdmin,
    })
  ) {
    return NextResponse.json(
      { error: "Création réservée à la direction, l’admin et l’administratif." },
      { status: 403 },
    );
  }

  const body = (await req.json()) as Record<string, unknown>;
  try {
    const created = await createElevePreinscrit({
      etablissementId: etabId,
      nom: String(body.nom || ""),
      prenom: String(body.prenom || ""),
      parentEmail: String(body.parentEmail || ""),
      parentPhone: String(body.parentPhone || ""),
      parentFirstName: body.parentFirstName ? String(body.parentFirstName) : null,
      parentLastName: body.parentLastName ? String(body.parentLastName) : null,
      classe: body.classe ? String(body.classe) : null,
      siteId: body.siteId ? String(body.siteId) : null,
      sourcePrefix: "manuel",
    });

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: session.user.id,
      resourceType: "eleve",
      resourceId: created.id,
      eleveId: created.id,
      action: "create",
      metadata: { source: "manuel", status: "preinscrit" },
    });

    return NextResponse.json({
      success: true,
      eleve: created,
      dossierUrl: `/eleves/dossier/${created.id}`,
      inscriptionDocsUrl: `/eleves/dossier/${created.id}/inscription`,
    });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    const status = /requis|invalide/i.test(msg) ? 400 : 500;
    return NextResponse.json({ error: msg }, { status });
  }
}
