import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, preinscription } from "@/db/schema";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getAppSession } from "@/app/lib/intranet-session";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { canViewFullElevesDossierHub } from "@/app/lib/eleve-dossier-scope";

/** Liste des préinscriptions du tenant (filtrable par site). */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const session = await getAppSession();
  if (!session?.user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }
  const fullHub = canViewFullElevesDossierHub({
    roles: session.user.roles,
    orgAdmin: session.user.orgAdmin,
    platformAdmin: session.user.platformAdmin,
  });
  if (!fullHub) {
    return NextResponse.json({ error: "Accès réservé au staff administratif." }, { status: 403 });
  }

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const url = new URL(req.url);
  const siteId = url.searchParams.get("siteId");
  const status = url.searchParams.get("status") || "pending";

  const db = getDb();
  const conditions = [
    eq(preinscription.etablissementId, etabId),
    eq(preinscription.status, status),
  ];
  if (siteId) conditions.push(eq(preinscription.siteId, siteId));

  const rows = await db
    .select()
    .from(preinscription)
    .where(and(...conditions))
    .orderBy(desc(preinscription.createdAt))
    .limit(200);

  return NextResponse.json({ count: rows.length, preinscriptions: rows });
}

/** Création publique ou admin d’une préinscription. */
export async function POST(req: Request) {
  const body = (await req.json()) as {
    nom?: string;
    prenom?: string;
    dateNaissance?: string;
    lieuNaissance?: string;
    siteId?: string;
    niveauVise?: string;
    filiereVisee?: string;
    demiPension?: boolean;
    etablissementPrecedent?: string;
    public?: boolean;
  };

  const nom = String(body.nom || "").trim();
  const prenom = String(body.prenom || "").trim();
  if (!nom || !prenom) {
    return NextResponse.json({ error: "Nom et prénom requis." }, { status: 400 });
  }

  // Public : résolu via cookie tenant / DEFAULT_TENANT — même helper
  if (!body.public) {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;
  }

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }

  const db = getDb();
  const [row] = await db
    .insert(preinscription)
    .values({
      etablissementId: etabId,
      siteId: body.siteId?.trim() || null,
      niveauVise: body.niveauVise?.trim() || null,
      filiereVisee: body.filiereVisee?.trim() || null,
      nom,
      prenom,
      dateNaissance: body.dateNaissance || null,
      lieuNaissance: body.lieuNaissance?.trim() || null,
      demiPension: Boolean(body.demiPension),
      etablissementPrecedent: body.etablissementPrecedent?.trim() || null,
      status: "pending",
      payload: body,
    })
    .returning();

  const session = await getAppSession();
  await recordEleveAccessAudit({
    etablissementId: etabId,
    actorUserId: session?.user?.id ?? null,
    resourceType: "preinscription",
    resourceId: row.id,
    action: "create",
  });

  return NextResponse.json({ success: true, preinscription: row });
}

/** Accepter une préinscription → crée l’élève + scolarité. */
export async function PATCH(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const session = await getAppSession();
  if (!session?.user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }
  const roles = session.user.roles;
  const canDecide =
    session.user.orgAdmin ||
    session.user.platformAdmin ||
    roles.includes("admin") ||
    hasGlobalAdminRole(roles) ||
    hasRole(roles, "administratif") ||
    INTRANET_DIRECTION_SLUGS.some((s) => roles.includes(s));
  if (!canDecide) {
    return NextResponse.json({ error: "Droits insuffisants." }, { status: 403 });
  }

  const body = (await req.json()) as { id?: string; action?: "accept" | "reject" };
  if (!body.id || !body.action) {
    return NextResponse.json({ error: "id et action requis." }, { status: 400 });
  }

  const db = getDb();
  const [pre] = await db
    .select()
    .from(preinscription)
    .where(and(eq(preinscription.etablissementId, etabId), eq(preinscription.id, body.id)))
    .limit(1);
  if (!pre) return NextResponse.json({ error: "Préinscription introuvable." }, { status: 404 });

  if (body.action === "reject") {
    await db
      .update(preinscription)
      .set({ status: "rejected", updatedAt: new Date() })
      .where(eq(preinscription.id, pre.id));
    return NextResponse.json({ success: true, status: "rejected" });
  }

  const folderName = `${pre.nom}_${pre.prenom}`.replace(/\s+/g, "_");
  const sourceKey = `preinsc:${pre.id}`;
  const [created] = await db
    .insert(eleve)
    .values({
      etablissementId: etabId,
      sourceKey,
      nom: pre.nom,
      prenom: pre.prenom,
      folderName,
      classe: pre.niveauVise,
      dateNaissance: pre.dateNaissance,
      lieuNaissance: pre.lieuNaissance,
      status: "preinscrit",
      secteur: pre.siteId,
    })
    .returning();

  const { eleveScolarite } = await import("@/db/schema");
  await db.insert(eleveScolarite).values({
    etablissementId: etabId,
    eleveId: created.id,
    siteId: pre.siteId,
    classe: pre.niveauVise,
    statut: "prevue",
    demiPension: pre.demiPension,
    etablissementPrecedent: pre.etablissementPrecedent,
  });

  await db
    .update(preinscription)
    .set({ status: "accepted", eleveId: created.id, updatedAt: new Date() })
    .where(eq(preinscription.id, pre.id));

  await recordEleveAccessAudit({
    etablissementId: etabId,
    actorUserId: session.user.id,
    resourceType: "preinscription",
    resourceId: pre.id,
    eleveId: created.id,
    action: "update",
    metadata: { decision: "accepted" },
  });

  return NextResponse.json({ success: true, eleve: created });
}
