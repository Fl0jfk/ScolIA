import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { documentAccessRequest, eleve, eleveDocument, user } from "@/db/schema";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getAppSession } from "@/app/lib/intranet-session";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { hasGlobalAdminRole, INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import { normalizeDocumentAccessDurationDays } from "@/app/lib/eleve-document-access-duration";
import { stageCycleKindFromStudent } from "@/app/lib/stage-config";
import { directionSecteursFromRoles } from "@/app/lib/pilotage-eleves-access";

function canDecideAccess(roles: string[], opts: { orgAdmin?: boolean; platformAdmin?: boolean }) {
  if (opts.orgAdmin || opts.platformAdmin || roles.includes("admin") || hasGlobalAdminRole(roles)) {
    return true;
  }
  return INTRANET_DIRECTION_SLUGS.some((slug) => roles.includes(slug));
}

/** File direction — demandes d’accès documents en attente. */
export async function GET(req: Request) {
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
  const roles =
    session.user.roles.length > 0
      ? session.user.roles
      : await listUserRolesFromDb(session.user.id, etabId);
  const orgAdmin = Boolean(session.user.orgAdmin);
  const platformAdmin = Boolean(session.user.platformAdmin);

  const url = new URL(req.url);
  const status = url.searchParams.get("status") || "pending";

  const db = getDb();

  const selectShape = {
    id: documentAccessRequest.id,
    documentId: documentAccessRequest.documentId,
    requesterUserId: documentAccessRequest.requesterUserId,
    status: documentAccessRequest.status,
    durationDays: documentAccessRequest.durationDays,
    note: documentAccessRequest.note,
    createdAt: documentAccessRequest.createdAt,
    expiresAt: documentAccessRequest.expiresAt,
    docTitle: eleveDocument.title,
    docTiroir: eleveDocument.tiroir,
    eleveId: eleveDocument.eleveId,
    eleveNom: eleve.nom,
    elevePrenom: eleve.prenom,
    eleveClasse: eleve.classe,
    eleveSecteur: eleve.secteur,
    requesterName: user.name,
    requesterEmail: user.email,
  };

  if (!canDecideAccess(roles, { orgAdmin, platformAdmin })) {
    const mine = await db
      .select(selectShape)
      .from(documentAccessRequest)
      .innerJoin(eleveDocument, eq(documentAccessRequest.documentId, eleveDocument.id))
      .innerJoin(eleve, eq(eleveDocument.eleveId, eleve.id))
      .leftJoin(user, eq(documentAccessRequest.requesterUserId, user.id))
      .where(
        and(
          eq(documentAccessRequest.etablissementId, etabId),
          eq(documentAccessRequest.requesterUserId, session.user.id),
          eq(documentAccessRequest.status, status),
        ),
      )
      .orderBy(desc(documentAccessRequest.createdAt))
      .limit(100);
    return NextResponse.json({ requests: mine, canDecide: false });
  }

  let requests = await db
    .select(selectShape)
    .from(documentAccessRequest)
    .innerJoin(eleveDocument, eq(documentAccessRequest.documentId, eleveDocument.id))
    .innerJoin(eleve, eq(eleveDocument.eleveId, eleve.id))
    .leftJoin(user, eq(documentAccessRequest.requesterUserId, user.id))
    .where(
      and(
        eq(documentAccessRequest.etablissementId, etabId),
        eq(documentAccessRequest.status, status),
      ),
    )
    .orderBy(desc(documentAccessRequest.createdAt))
    .limit(100);

  // Direction cycle : ne voir que les demandes du collège / lycée / école concernés.
  const userSecteurs = directionSecteursFromRoles(roles);
  if (
    userSecteurs.length > 0 &&
    !orgAdmin &&
    !platformAdmin &&
    !roles.includes("admin") &&
    !hasGlobalAdminRole(roles)
  ) {
    requests = requests.filter((r) => {
      const cycle = stageCycleKindFromStudent(
        String(r.eleveSecteur || r.eleveClasse || ""),
        r.eleveClasse || undefined,
      );
      return userSecteurs.includes(cycle);
    });
  }

  return NextResponse.json({ requests, canDecide: true });
}

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
  const roles =
    session.user.roles.length > 0
      ? session.user.roles
      : await listUserRolesFromDb(session.user.id, etabId);
  const orgAdmin = Boolean(session.user.orgAdmin);
  const platformAdmin = Boolean(session.user.platformAdmin);
  if (!canDecideAccess(roles, { orgAdmin, platformAdmin })) {
    return NextResponse.json({ error: "Réservé à la direction." }, { status: 403 });
  }

  const body = (await req.json()) as {
    id?: string;
    decision?: "approved" | "rejected";
    durationDays?: number;
  };
  if (!body.id || (body.decision !== "approved" && body.decision !== "rejected")) {
    return NextResponse.json({ error: "Paramètres invalides." }, { status: 400 });
  }

  const db = getDb();
  const [reqRow] = await db
    .select()
    .from(documentAccessRequest)
    .where(
      and(
        eq(documentAccessRequest.etablissementId, etabId),
        eq(documentAccessRequest.id, body.id),
        eq(documentAccessRequest.status, "pending"),
      ),
    )
    .limit(1);
  if (!reqRow) {
    return NextResponse.json({ error: "Demande introuvable." }, { status: 404 });
  }

  const now = new Date();
  let expiresAt: Date | null = null;
  let durationDays = reqRow.durationDays;
  if (body.decision === "approved") {
    durationDays = normalizeDocumentAccessDurationDays(
      body.durationDays ?? reqRow.durationDays,
      7,
    );
    expiresAt = new Date(now.getTime() + durationDays * 24 * 60 * 60 * 1000);
  }

  const [updated] = await db
    .update(documentAccessRequest)
    .set({
      status: body.decision,
      durationDays,
      decidedByUserId: session.user.id,
      decidedAt: now,
      expiresAt,
    })
    .where(eq(documentAccessRequest.id, reqRow.id))
    .returning();

  const [doc] = await db
    .select({ eleveId: eleveDocument.eleveId })
    .from(eleveDocument)
    .where(
      and(eq(eleveDocument.etablissementId, etabId), eq(eleveDocument.id, reqRow.documentId)),
    )
    .limit(1);

  await recordEleveAccessAudit({
    etablissementId: etabId,
    actorUserId: session.user.id,
    resourceType: "document",
    resourceId: reqRow.documentId,
    eleveId: doc?.eleveId ?? null,
    action: body.decision === "approved" ? "grant" : "deny",
    metadata: {
      requestId: reqRow.id,
      durationDays,
      expiresAt: expiresAt?.toISOString() ?? null,
    },
  });

  return NextResponse.json({ success: true, request: updated });
}
