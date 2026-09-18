import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { documentAccessRequest, eleve, eleveDocument, user } from "@/db/schema";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getAppSession } from "@/app/lib/intranet-session";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import {
  canDecideDocumentAccessGrant,
  recordEleveAccessAudit,
} from "@/app/lib/eleve-dossier-access";
import { normalizeDocumentAccessDurationDays } from "@/app/lib/eleve-document-access-duration";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { hasGlobalAdminRole, INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

function canSeeAccessQueue(
  roles: string[],
  opts: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  if (opts.orgAdmin || opts.platformAdmin || roles.includes("admin") || hasGlobalAdminRole(roles)) {
    return true;
  }
  if (INTRANET_DIRECTION_SLUGS.some((slug) => roles.includes(slug))) return true;
  return (
    hasRole(roles, "psychologue") ||
    hasRole(roles, "infirmerie") ||
    hasRole(roles, "administratif") ||
    hasRole(roles, "cpe") ||
    hasRole(roles, "comptabilite")
  );
}

/** File des demandes d’accès documents (psy / infirmerie / direction selon le document). */
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

  if (!canSeeAccessQueue(roles, { orgAdmin, platformAdmin })) {
    return NextResponse.json({ requests: [], canDecide: false });
  }

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
    docConfidentialite: eleveDocument.confidentialite,
    eleveId: eleveDocument.eleveId,
    eleveNom: eleve.nom,
    elevePrenom: eleve.prenom,
    eleveClasse: eleve.classe,
    requesterName: user.name,
    requesterEmail: user.email,
  };

  const all = await db
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
    .limit(150);

  const roleOpts = { orgAdmin, platformAdmin };
  const requests = all.filter((r) => {
    // Mes demandes : toujours visibles.
    if (r.requesterUserId === session.user.id) return true;
    return canDecideDocumentAccessGrant(
      {
        tiroir: r.docTiroir,
        title: r.docTitle,
        confidentialite: r.docConfidentialite,
      },
      roles,
      roleOpts,
    );
  });

  const canDecide = requests.some((r) =>
    canDecideDocumentAccessGrant(
      {
        tiroir: r.docTiroir,
        title: r.docTitle,
        confidentialite: r.docConfidentialite,
      },
      roles,
      roleOpts,
    ),
  );

  return NextResponse.json({ requests, canDecide });
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

  const [doc] = await db
    .select()
    .from(eleveDocument)
    .where(
      and(eq(eleveDocument.etablissementId, etabId), eq(eleveDocument.id, reqRow.documentId)),
    )
    .limit(1);
  if (!doc) {
    return NextResponse.json({ error: "Document introuvable." }, { status: 404 });
  }

  if (!canDecideDocumentAccessGrant(doc, roles, { orgAdmin, platformAdmin })) {
    return NextResponse.json(
      { error: "Réservé au propriétaire du silo (psychologue / infirmerie)." },
      { status: 403 },
    );
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

  await recordEleveAccessAudit({
    etablissementId: etabId,
    actorUserId: session.user.id,
    resourceType: "document",
    resourceId: reqRow.documentId,
    eleveId: doc.eleveId,
    action: body.decision === "approved" ? "grant" : "deny",
    metadata: {
      requestId: reqRow.id,
      durationDays,
      expiresAt: expiresAt?.toISOString() ?? null,
    },
  });

  return NextResponse.json({ success: true, request: updated });
}
