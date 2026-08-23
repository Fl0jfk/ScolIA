import { NextResponse } from "next/server";
import { and, asc, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  anneeScolaire,
  documentAccessRequest,
  eleve,
  eleveDocument,
  eleveFoyerLink,
  eleveScolarite,
  etablissementSite,
  foyer,
  foyerResponsable,
} from "@/db/schema";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import {
  canRegisterEleveDocument,
  eleveDossierSectionsForRoles,
  listEleveDocumentsForViewer,
  recordEleveAccessAudit,
  type EleveDocConfidentialite,
  type EleveDocTiroir,
} from "@/app/lib/eleve-dossier-access";
import {
  isProfesseurScopedDossierViewer,
  listAssignedClassesForTeacher,
  sanitizeEleveRowForProfViewer,
  teacherCanAccessEleveClasse,
} from "@/app/lib/eleve-dossier-prof";
import { getAppSession } from "@/app/lib/intranet-session";
import { hasGlobalAdminRole, INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { loadAppConfig } from "@/app/lib/app-config";
import { resolveEleveLiveCourse } from "@/app/lib/rh/planning-class-live";

type Ctx = { params: Promise<{ id: string }> };

const TIROIRS: EleveDocTiroir[] = [
  "scolaire",
  "inscription",
  "facturation",
  "voyages",
  "sante",
  "vie_scolaire",
];
const CONFIDS: EleveDocConfidentialite[] = ["standard", "restreint", "sante"];

function isExactAdmin(roles: string[]): boolean {
  return roles.includes("admin") || hasGlobalAdminRole(roles);
}

function isDirection(roles: string[]): boolean {
  return INTRANET_DIRECTION_SLUGS.some((slug) => roles.includes(slug));
}

function canEditStructure(
  roles: string[],
  opts: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  return Boolean(
    opts.orgAdmin ||
      opts.platformAdmin ||
      isExactAdmin(roles) ||
      isDirection(roles) ||
      hasRole(roles, "administratif"),
  );
}

function canDecideAccess(
  roles: string[],
  opts: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  return Boolean(
    opts.orgAdmin || opts.platformAdmin || isExactAdmin(roles) || isDirection(roles),
  );
}

async function resolveViewer(etabId: string) {
  const session = await getAppSession();
  if (!session?.user) return null;
  const roles =
    session.user.roles.length > 0
      ? session.user.roles
      : await listUserRolesFromDb(session.user.id, etabId);
  return {
    userId: session.user.id,
    businessUserId: session.user.businessUserId,
    roles,
    orgAdmin: Boolean(session.user.orgAdmin),
    platformAdmin: Boolean(session.user.platformAdmin),
  };
}

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const { id } = await ctx.params;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }

  const viewer = await resolveViewer(etabId);
  if (!viewer) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }
  const { userId: authUserId, businessUserId, roles, orgAdmin, platformAdmin } = viewer;
  const sections = [...eleveDossierSectionsForRoles(roles, { orgAdmin, platformAdmin })];
  const profRestrictedView = isProfesseurScopedDossierViewer({ roles, orgAdmin, platformAdmin });

  const db = getDb();
  const [row] = await db
    .select()
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, id)))
    .limit(1);
  if (!row) {
    return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
  }

  if (profRestrictedView) {
    const assignedClasses = await listAssignedClassesForTeacher(businessUserId);
    // Hors classe = invisible (même réponse qu’introuvable), pas un 403.
    if (!teacherCanAccessEleveClasse(row.classe, assignedClasses)) {
      return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
    }
  }

  const scolarites = await db
    .select()
    .from(eleveScolarite)
    .where(and(eq(eleveScolarite.etablissementId, etabId), eq(eleveScolarite.eleveId, id)))
    .orderBy(desc(eleveScolarite.createdAt));

  const links = await db
    .select()
    .from(eleveFoyerLink)
    .where(and(eq(eleveFoyerLink.etablissementId, etabId), eq(eleveFoyerLink.eleveId, id)));

  const foyers = [];
  for (const link of links) {
    const [f] = await db
      .select()
      .from(foyer)
      .where(and(eq(foyer.etablissementId, etabId), eq(foyer.id, link.foyerId)))
      .limit(1);
    if (!f) continue;
    const responsables = await db
      .select()
      .from(foyerResponsable)
      .where(
        and(eq(foyerResponsable.etablissementId, etabId), eq(foyerResponsable.foyerId, f.id)),
      )
      .orderBy(asc(foyerResponsable.rang));
    foyers.push({
      id: f.id,
      label: f.label,
      adresse: f.adresse,
      codePostal: f.codePostal,
      ville: f.ville,
      payeurEstFoyer: f.payeurEstFoyer,
      relation: link.relation,
      responsables: responsables.map((r) => ({
        id: r.id,
        nom: r.nom,
        prenom: r.prenom,
        email: r.email,
        telephone: r.telephone,
        autoriteParentale: r.autoriteParentale,
        contactUrgence: r.contactUrgence,
        payeur: r.payeur,
        rang: r.rang,
      })),
    });
  }

  const documents = sections.includes("documents")
    ? await listEleveDocumentsForViewer({
        etablissementId: etabId,
        eleveId: id,
        userId: authUserId,
        roles,
        orgAdmin,
        platformAdmin,
      })
    : [];

  const sites = await db
    .select({
      siteId: etablissementSite.siteId,
      label: etablissementSite.label,
      kind: etablissementSite.kind,
    })
    .from(etablissementSite)
    .where(eq(etablissementSite.etablissementId, etabId))
    .orderBy(asc(etablissementSite.label));

  const annees = await db
    .select({
      id: anneeScolaire.id,
      label: anneeScolaire.label,
      isCurrent: anneeScolaire.isCurrent,
    })
    .from(anneeScolaire)
    .where(eq(anneeScolaire.etablissementId, etabId))
    .orderBy(desc(anneeScolaire.label));

  const pendingAccess =
    sections.includes("documents") && canDecideAccess(roles, { orgAdmin, platformAdmin })
      ? await db
          .select({
            id: documentAccessRequest.id,
            documentId: documentAccessRequest.documentId,
            requesterUserId: documentAccessRequest.requesterUserId,
            durationDays: documentAccessRequest.durationDays,
            note: documentAccessRequest.note,
            createdAt: documentAccessRequest.createdAt,
            docTitle: eleveDocument.title,
          })
          .from(documentAccessRequest)
          .innerJoin(eleveDocument, eq(documentAccessRequest.documentId, eleveDocument.id))
          .where(
            and(
              eq(documentAccessRequest.etablissementId, etabId),
              eq(eleveDocument.eleveId, id),
              eq(documentAccessRequest.status, "pending"),
            ),
          )
          .orderBy(desc(documentAccessRequest.createdAt))
          .limit(50)
      : [];

  await recordEleveAccessAudit({
    etablissementId: etabId,
    actorUserId: authUserId,
    resourceType: "fiche_eleve",
    resourceId: id,
    eleveId: id,
    action: "view",
  });

  const classmates =
    row.classe && sections.includes("scolarite")
      ? await db
          .select({
            id: eleve.id,
            nom: eleve.nom,
            prenom: eleve.prenom,
            classe: eleve.classe,
          })
          .from(eleve)
          .where(and(eq(eleve.etablissementId, etabId), eq(eleve.classe, row.classe)))
          .limit(80)
      : [];

  let enCoursMaintenant = await (async () => {
    try {
      const cfg = await loadAppConfig();
      return resolveEleveLiveCourse({
        classe: row.classe,
        zone: cfg.identity.schoolHolidayZone ?? null,
      });
    } catch {
      return resolveEleveLiveCourse({ classe: row.classe });
    }
  })();

  return NextResponse.json({
    eleve: profRestrictedView ? sanitizeEleveRowForProfViewer(row) : row,
    sections,
    scolarites,
    foyers: sections.includes("famille") ? foyers : [],
    documents,
    classmates: classmates.filter((c) => c.id !== id),
    meta: {
      sites,
      annees,
      canEditStructure: canEditStructure(roles, { orgAdmin, platformAdmin }),
      canDecideAccess: canDecideAccess(roles, { orgAdmin, platformAdmin }),
      profRestrictedView,
      tiroirs: TIROIRS,
    },
    pendingAccessRequests: pendingAccess,
    enCoursMaintenant,
  });
}

type DossierBody = {
  action?: string;
  title?: string;
  anneeLabel?: string;
  label?: string;
  adresse?: string;
  codePostal?: string;
  ville?: string;
  payeurEstFoyer?: boolean;
  relation?: string;
  foyerId?: string;
  responsable?: {
    nom?: string;
    prenom?: string;
    email?: string | null;
    telephone?: string | null;
    autoriteParentale?: boolean;
    contactUrgence?: boolean;
    payeur?: boolean;
  };
  siteId?: string | null;
  classe?: string | null;
  anneeScolaireId?: string | null;
  statut?: string;
  demiPension?: boolean;
  etablissementPrecedent?: string | null;
  closePrevious?: boolean;
  updateEleveClasse?: boolean;
  documentId?: string;
  durationDays?: number;
  note?: string | null;
  requestId?: string;
  decision?: "approved" | "rejected";
  tiroir?: string;
  confidentialite?: string;
  s3Key?: string | null;
  fileUrl?: string | null;
  mimeType?: string | null;
  source?: string;
};

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { id } = await ctx.params;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }

  const viewer = await resolveViewer(etabId);
  if (!viewer) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }
  const { userId: authUserId, businessUserId, roles, orgAdmin, platformAdmin } = viewer;
  const sections = eleveDossierSectionsForRoles(roles, { orgAdmin, platformAdmin });
  const body = (await req.json()) as DossierBody;
  const action = String(body.action || "");

  const db = getDb();
  const [row] = await db
    .select()
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, id)))
    .limit(1);
  if (!row) return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });

  if (isProfesseurScopedDossierViewer({ roles, orgAdmin, platformAdmin })) {
    const assignedClasses = await listAssignedClassesForTeacher(businessUserId);
    // Hors classe = invisible, pas un 403.
    if (!teacherCanAccessEleveClasse(row.classe, assignedClasses)) {
      return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
    }
  }

  if (action === "create_foyer") {
    if (!sections.has("famille") || !canEditStructure(roles, { orgAdmin, platformAdmin })) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    const resp = body.responsable;
    const nom = String(resp?.nom || "").trim();
    const prenom = String(resp?.prenom || "").trim();
    if (!nom || !prenom) {
      return NextResponse.json({ error: "Responsable : nom et prénom requis." }, { status: 400 });
    }
    const [f] = await db
      .insert(foyer)
      .values({
        etablissementId: etabId,
        label: String(body.label || `Foyer ${nom}`).trim() || "Foyer",
        adresse: body.adresse?.trim() || null,
        codePostal: body.codePostal?.trim() || null,
        ville: body.ville?.trim() || null,
        payeurEstFoyer: body.payeurEstFoyer !== false,
      })
      .returning();

    await db.insert(foyerResponsable).values({
      etablissementId: etabId,
      foyerId: f.id,
      nom,
      prenom,
      email: resp?.email?.trim() || null,
      telephone: resp?.telephone?.trim() || null,
      autoriteParentale: Boolean(resp?.autoriteParentale),
      contactUrgence: Boolean(resp?.contactUrgence),
      payeur: Boolean(resp?.payeur),
      rang: 1,
    });

    await db.insert(eleveFoyerLink).values({
      etablissementId: etabId,
      eleveId: id,
      foyerId: f.id,
      relation: String(body.relation || "principal").trim() || "principal",
    });

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: authUserId,
      resourceType: "fiche_foyer",
      resourceId: f.id,
      eleveId: id,
      action: "create",
    });
    return NextResponse.json({ success: true, foyerId: f.id });
  }

  if (action === "add_responsable") {
    if (!sections.has("famille") || !canEditStructure(roles, { orgAdmin, platformAdmin })) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    const foyerId = String(body.foyerId || "");
    if (!foyerId) {
      return NextResponse.json({ error: "foyerId requis." }, { status: 400 });
    }
    const [link] = await db
      .select()
      .from(eleveFoyerLink)
      .where(
        and(
          eq(eleveFoyerLink.etablissementId, etabId),
          eq(eleveFoyerLink.eleveId, id),
          eq(eleveFoyerLink.foyerId, foyerId),
        ),
      )
      .limit(1);
    if (!link) {
      return NextResponse.json({ error: "Foyer non lié à cet élève." }, { status: 404 });
    }
    const existing = await db
      .select({ id: foyerResponsable.id, rang: foyerResponsable.rang })
      .from(foyerResponsable)
      .where(
        and(eq(foyerResponsable.etablissementId, etabId), eq(foyerResponsable.foyerId, foyerId)),
      );
    if (existing.length >= 4) {
      return NextResponse.json({ error: "Maximum 4 responsables par foyer." }, { status: 400 });
    }
    const resp = body.responsable;
    const nom = String(resp?.nom || "").trim();
    const prenom = String(resp?.prenom || "").trim();
    if (!nom || !prenom) {
      return NextResponse.json({ error: "Responsable : nom et prénom requis." }, { status: 400 });
    }
    const usedRangs = new Set(existing.map((r) => r.rang));
    let rang = 1;
    while (usedRangs.has(rang) && rang <= 4) rang += 1;

    const [created] = await db
      .insert(foyerResponsable)
      .values({
        etablissementId: etabId,
        foyerId,
        nom,
        prenom,
        email: resp?.email?.trim() || null,
        telephone: resp?.telephone?.trim() || null,
        autoriteParentale: Boolean(resp?.autoriteParentale),
        contactUrgence: Boolean(resp?.contactUrgence),
        payeur: Boolean(resp?.payeur),
        rang,
      })
      .returning();

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: authUserId,
      resourceType: "fiche_foyer",
      resourceId: foyerId,
      eleveId: id,
      action: "update",
      metadata: { responsableId: created.id },
    });
    return NextResponse.json({ success: true, responsable: created });
  }

  if (action === "create_scolarite") {
    if (!sections.has("scolarite") || !canEditStructure(roles, { orgAdmin, platformAdmin })) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    const classe = body.classe?.trim() || null;
    const siteId = body.siteId?.trim() || null;
    const statut = String(body.statut || "en_cours").trim() || "en_cours";

    if (body.closePrevious !== false) {
      await db
        .update(eleveScolarite)
        .set({ statut: "terminee", updatedAt: new Date() })
        .where(
          and(
            eq(eleveScolarite.etablissementId, etabId),
            eq(eleveScolarite.eleveId, id),
            eq(eleveScolarite.statut, "en_cours"),
          ),
        );
    }

    const [sc] = await db
      .insert(eleveScolarite)
      .values({
        etablissementId: etabId,
        eleveId: id,
        anneeScolaireId: body.anneeScolaireId || null,
        siteId,
        classe,
        statut,
        demiPension: Boolean(body.demiPension),
        etablissementPrecedent: body.etablissementPrecedent?.trim() || null,
      })
      .returning();

    if (body.updateEleveClasse !== false && classe) {
      await db
        .update(eleve)
        .set({ classe, updatedAt: new Date(), status: "inscrit" })
        .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, id)));
    }

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: authUserId,
      resourceType: "fiche_eleve",
      resourceId: id,
      eleveId: id,
      action: "create_scolarite",
      metadata: { scolariteId: sc.id, siteId, classe },
    });
    return NextResponse.json({ success: true, scolarite: sc });
  }

  if (action === "register_document") {
    if (!sections.has("documents")) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    const tiroir = String(body.tiroir || "") as EleveDocTiroir;
    if (!TIROIRS.includes(tiroir)) {
      return NextResponse.json({ error: "Tiroir invalide." }, { status: 400 });
    }
    const confidentialite = String(
      body.confidentialite || "standard",
    ) as EleveDocConfidentialite;
    if (!CONFIDS.includes(confidentialite)) {
      return NextResponse.json({ error: "Confidentialité invalide." }, { status: 400 });
    }
    const title = String(body.title || "").trim();
    if (!title) {
      return NextResponse.json({ error: "Titre requis." }, { status: 400 });
    }
    if (
      !canRegisterEleveDocument(tiroir, confidentialite, roles, {
        orgAdmin,
        platformAdmin,
      })
    ) {
      return NextResponse.json(
        { error: "Tiroir ou confidentialité non autorisé pour votre rôle." },
        { status: 403 },
      );
    }

    const [doc] = await db
      .insert(eleveDocument)
      .values({
        etablissementId: etabId,
        eleveId: id,
        tiroir,
        title,
        mimeType: body.mimeType || null,
        s3Key: body.s3Key || null,
        fileUrl: body.fileUrl || null,
        anneeLabel: body.anneeLabel || null,
        confidentialite,
        source: body.source || "upload",
        createdByUserId: authUserId,
      })
      .returning();

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: authUserId,
      resourceType: "document",
      resourceId: doc.id,
      eleveId: id,
      action: "create",
      metadata: { tiroir, source: doc.source },
    });
    return NextResponse.json({ success: true, document: doc });
  }

  if (action === "request_document_access") {
    if (!sections.has("documents")) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    const documentId = String(body.documentId || "");
    if (!documentId) {
      return NextResponse.json({ error: "documentId requis." }, { status: 400 });
    }
    const [doc] = await db
      .select()
      .from(eleveDocument)
      .where(
        and(
          eq(eleveDocument.etablissementId, etabId),
          eq(eleveDocument.eleveId, id),
          eq(eleveDocument.id, documentId),
        ),
      )
      .limit(1);
    if (!doc) {
      return NextResponse.json({ error: "Document introuvable." }, { status: 404 });
    }

    const durationDays = Math.min(7, Math.max(1, Number(body.durationDays) || 1));
    const [created] = await db
      .insert(documentAccessRequest)
      .values({
        etablissementId: etabId,
        documentId,
        requesterUserId: authUserId,
        status: "pending",
        durationDays,
        note: body.note?.trim() || null,
      })
      .returning();

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: authUserId,
      resourceType: "document",
      resourceId: documentId,
      eleveId: id,
      action: "request",
      metadata: { requestId: created.id, durationDays },
    });
    return NextResponse.json({ success: true, request: created });
  }

  if (action === "decide_document_access") {
    if (!canDecideAccess(roles, { orgAdmin, platformAdmin })) {
      return NextResponse.json({ error: "Réservé à la direction." }, { status: 403 });
    }
    if (body.decision !== "approved" && body.decision !== "rejected") {
      return NextResponse.json({ error: "Décision invalide." }, { status: 400 });
    }
    const requestId = String(body.requestId || "");
    const [reqRow] = await db
      .select()
      .from(documentAccessRequest)
      .where(
        and(
          eq(documentAccessRequest.etablissementId, etabId),
          eq(documentAccessRequest.id, requestId),
          eq(documentAccessRequest.status, "pending"),
        ),
      )
      .limit(1);
    if (!reqRow) {
      return NextResponse.json({ error: "Demande introuvable." }, { status: 404 });
    }

    const now = new Date();
    let expiresAt: Date | null = null;
    if (body.decision === "approved") {
      const days = Math.min(7, Math.max(1, reqRow.durationDays || 1));
      expiresAt = new Date(now.getTime() + days * 24 * 60 * 60 * 1000);
    }

    const [updated] = await db
      .update(documentAccessRequest)
      .set({
        status: body.decision,
        decidedByUserId: authUserId,
        decidedAt: now,
        expiresAt,
      })
      .where(eq(documentAccessRequest.id, reqRow.id))
      .returning();

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: authUserId,
      resourceType: "document",
      resourceId: reqRow.documentId,
      eleveId: id,
      action: body.decision === "approved" ? "grant" : "deny",
      metadata: { requestId: reqRow.id },
    });
    return NextResponse.json({ success: true, request: updated });
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}
