import { NextResponse } from "next/server";
import { after } from "next/server";
import { and, asc, desc, eq, inArray } from "drizzle-orm";
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
import { resolveCurrentEtablissementId, syncEleveScolariteFromEleveRow, ensureEleveFoyerFromParentContacts } from "@/app/lib/ent-core-db";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import {
  canRegisterEleveDocument,
  eleveDocCategoriesMetaForRoles,
  eleveDocTiroirsForRoles,
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
import { buildEleveDossierClassCatalog } from "@/app/lib/eleve-dossier-catalog";
import { buildEleveSyntheseSnapshot } from "@/app/lib/eleve-dossier-synthese";
import {
  countMidiFromGrille,
  parseEleveGrilleRepas,
} from "@/app/lib/eleve-grille-repas";
import { listMoyennesForEleve } from "@/app/lib/notes-saisie-db";
import { COMPETENCE_NIVEAUX, listCompetencesForEleve } from "@/app/lib/notes-competences-db";
import { listAbsencesForEleve } from "@/app/lib/vs-absences-db";
import { listSanctionsForEleve } from "@/app/lib/vs-sanctions-db";
import { listCarnetForEleve } from "@/app/lib/vs-carnet-db";
import { countFacturesEnRetardForEleve } from "@/app/lib/facturation-db";
import { listGroupesForEleve } from "@/app/lib/groupes-pedagogiques-db";
import { parisDateKey } from "@/app/lib/paris-time";
import { ensureEleveScolariteGrilleRepasColumn } from "@/app/lib/eleve-scolarite-schema";

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
  try {
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
  const { loadModuleAccess } = await import("@/app/lib/module-access-store");
  const { dossierSectionsForRolesWithAccess } = await import("@/app/lib/module-access");
  const access = await loadModuleAccess();
  const sections = [
    ...dossierSectionsForRolesWithAccess(
      roles,
      { orgAdmin, platformAdmin },
      access,
      { userId: authUserId, businessUserId },
    ),
  ];
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

  await ensureEleveScolariteGrilleRepasColumn();

  const needDocs = sections.includes("documents");
  const needNotes = sections.includes("notes");
  const needVs = sections.includes("vie_scolaire");
  const needScol = sections.includes("scolarite");
  const needFactu = sections.includes("facturation");
  const needFamille = sections.includes("famille");
  const canDecide = canDecideAccess(roles, { orgAdmin, platformAdmin });

  // Rattrapage hors chemin critique : sync scolarité en arrière-plan.
  after(async () => {
    try {
      await syncEleveScolariteFromEleveRow(etabId, {
        id: row.id,
        classe: row.classe,
        regime: row.regime,
      });
    } catch (e) {
      console.warn("[eleves/dossier] sync scolarité", e);
    }
  });

  const [
    scolarites,
    links,
    documentsRaw,
    sites,
    annees,
    pendingAccess,
    enCoursMaintenant,
    notesRaw,
    competencesRaw,
    groupesEleve,
    absencesRaw,
    sanctionsRaw,
    carnetRaw,
    financesSynthese,
  ] = await Promise.all([
    db
      .select()
      .from(eleveScolarite)
      .where(and(eq(eleveScolarite.etablissementId, etabId), eq(eleveScolarite.eleveId, id)))
      .orderBy(desc(eleveScolarite.createdAt)),
    needFamille
      ? db
          .select()
          .from(eleveFoyerLink)
          .where(and(eq(eleveFoyerLink.etablissementId, etabId), eq(eleveFoyerLink.eleveId, id)))
      : Promise.resolve([] as (typeof eleveFoyerLink.$inferSelect)[]),
    needDocs
      ? listEleveDocumentsForViewer({
          etablissementId: etabId,
          eleveId: id,
          userId: authUserId,
          roles,
          orgAdmin,
          platformAdmin,
        }).catch(() => [])
      : Promise.resolve([]),
    db
      .select({
        siteId: etablissementSite.siteId,
        label: etablissementSite.label,
        kind: etablissementSite.kind,
      })
      .from(etablissementSite)
      .where(eq(etablissementSite.etablissementId, etabId))
      .orderBy(asc(etablissementSite.label)),
    db
      .select({
        id: anneeScolaire.id,
        label: anneeScolaire.label,
        isCurrent: anneeScolaire.isCurrent,
      })
      .from(anneeScolaire)
      .where(eq(anneeScolaire.etablissementId, etabId))
      .orderBy(desc(anneeScolaire.label)),
    needDocs && canDecide
      ? db
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
      : Promise.resolve([]),
    (async () => {
      try {
        const cfg = await loadAppConfig();
        return resolveEleveLiveCourse({
          classe: row.classe,
          zone: cfg.identity.schoolHolidayZone ?? null,
        });
      } catch {
        try {
          return resolveEleveLiveCourse({ classe: row.classe });
        } catch {
          return {
            activity: null,
            reason: "pas_edt" as const,
            label: "Emploi du temps indisponible",
          };
        }
      }
    })(),
    needNotes
      ? listMoyennesForEleve(etabId, id).catch(() => [])
      : Promise.resolve([]),
    needNotes
      ? listCompetencesForEleve(etabId, id).catch(() => [])
      : Promise.resolve([]),
    needScol
      ? listGroupesForEleve(etabId, id).catch(() => [])
      : Promise.resolve([]),
    needVs
      ? listAbsencesForEleve(etabId, id, { limit: 40 }).catch(() => [])
      : Promise.resolve([]),
    needVs
      ? listSanctionsForEleve(etabId, id, { limit: 30 }).catch(() => [])
      : Promise.resolve([]),
    needVs
      ? listCarnetForEleve(etabId, id, { limit: 30 }).catch(() => [])
      : Promise.resolve([]),
    needFactu
      ? countFacturesEnRetardForEleve(etabId, id, parisDateKey(new Date()))
          .then((enRetard) => ({
            available: true as const,
            label: enRetard > 0 ? `${enRetard} facture(s) en retard` : "Facturation à jour",
            detail:
              enRetard > 0
                ? "Échéance dépassée — voir l’onglet Finances"
                : "Aucune facture émise en retard pour ce foyer.",
          }))
          .catch(() => undefined)
      : Promise.resolve(undefined),
  ]);

  // Foyers : 2 requêtes max au lieu de N+1
  const foyers: Array<{
    id: string;
    label: string;
    adresse: string | null;
    codePostal: string | null;
    ville: string | null;
    payeurEstFoyer: boolean;
    relation: string;
    responsables: Array<{
      id: string;
      nom: string;
      prenom: string;
      email: string | null;
      telephone: string | null;
      autoriteParentale: boolean;
      contactUrgence: boolean;
      payeur: boolean;
      rang: number;
    }>;
  }> = [];
  if (links.length) {
    const foyerIds = [...new Set(links.map((l) => l.foyerId))];
    const [foyerRows, respRows] = await Promise.all([
      db
        .select()
        .from(foyer)
        .where(and(eq(foyer.etablissementId, etabId), inArray(foyer.id, foyerIds))),
      db
        .select()
        .from(foyerResponsable)
        .where(
          and(eq(foyerResponsable.etablissementId, etabId), inArray(foyerResponsable.foyerId, foyerIds)),
        )
        .orderBy(asc(foyerResponsable.rang)),
    ]);
    const foyerById = new Map(foyerRows.map((f) => [f.id, f]));
    const respByFoyer = new Map<string, typeof respRows>();
    for (const r of respRows) {
      const list = respByFoyer.get(r.foyerId) || [];
      list.push(r);
      respByFoyer.set(r.foyerId, list);
    }
    for (const link of links) {
      const f = foyerById.get(link.foyerId);
      if (!f) continue;
      const responsables = respByFoyer.get(f.id) || [];
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
  }

  const documents = documentsRaw.map((d) => ({
    ...d,
    createdAt:
      d.createdAt instanceof Date ? d.createdAt.toISOString() : String(d.createdAt ?? ""),
  }));

  // Audit non bloquant
  after(() => {
    void recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: authUserId,
      resourceType: "fiche_eleve",
      resourceId: id,
      eleveId: id,
      action: "view",
    }).catch((auditErr) => console.error("[eleves/dossier] audit view", auditErr));
  });

  let catalog;
  try {
    catalog = await buildEleveDossierClassCatalog(sites);
  } catch (catalogErr) {
    console.error("[eleves/dossier] catalog", catalogErr);
    catalog = {
      sites: sites.map((s) => ({ siteId: s.siteId, label: s.label, kind: s.kind })),
      siteLabelById: new Map(sites.map((s) => [s.siteId, s.label])),
      classToSiteId: new Map<string, string>(),
      classOptions: [],
    };
  }
  const currentScolarite = scolarites[0] ?? null;
  const siteIdFromScolarite = currentScolarite?.siteId ?? null;

  const notesMoyennes = notesRaw.map((r) => ({
    matiereLibelle: r.matiereLibelle,
    moyenne: r.moyenne,
    nbNotes: r.nbNotes,
    periodeId: r.periodeId,
    periodeLibelle: r.periodeLibelle,
    periodeStatut: r.periodeStatut,
  }));

  const competences = competencesRaw.map((r) => ({
    domaineLibelle: r.domaineLibelle,
    itemLibelle: r.itemLibelle,
    niveau: r.niveau,
    niveauLabel: COMPETENCE_NIVEAUX.find((n) => n.code === r.niveau)?.label || "—",
    periodeId: r.periodeId,
  }));

  const absencesEleve = absencesRaw.map((r) => ({
    id: r.id,
    dateDebut: r.dateDebut,
    type: r.type,
    statut: r.statut,
    justifie: r.justifie,
    motif: r.motif,
  }));
  const sanctionsEleve = sanctionsRaw.map((r) => ({
    id: r.id,
    typeLibelle: r.typeLibelle,
    dateSanction: r.dateSanction,
    motif: r.motif,
    createdByNom: r.createdByNom,
  }));
  const carnetEleve = carnetRaw.map((r) => ({
    id: r.id,
    dateEntree: r.dateEntree,
    categorie: r.categorie,
    titre: r.titre,
    corps: r.corps,
    signeAt: r.signeAt ? r.signeAt.toISOString() : null,
    signeParNom: r.signeParNom,
    createdByNom: r.createdByNom,
  }));

  let absencesSynthese: {
    available: boolean;
    label: string;
    value: string;
    detail: string;
  } | undefined;
  if (needVs) {
    const aTraiter = absencesEleve.filter((a) => a.statut === "a_traiter").length;
    const totalAbs = absencesEleve.filter((a) => a.type === "absence").length;
    const totalRetards = absencesEleve.filter((a) => a.type === "retard").length;
    absencesSynthese = {
      available: true,
      label:
        totalAbs + totalRetards === 0
          ? "Aucune absence récente"
          : `${totalAbs} absence(s) · ${totalRetards} retard(s)`,
      value: String(totalAbs + totalRetards),
      detail:
        aTraiter > 0
          ? `${aTraiter} à traiter · ${sanctionsEleve.length} sanction(s) active(s)`
          : `${sanctionsEleve.length} sanction(s) active(s)`,
    };
  }

  const synthese = await buildEleveSyntheseSnapshot({
    eleve: {
      nom: row.nom,
      prenom: row.prenom,
      classe: row.classe,
      status: row.status,
      ine: row.ine,
      folderName: row.folderName,
      siteId: siteIdFromScolarite,
    },
    scolarite: currentScolarite
      ? {
          demiPension: currentScolarite.demiPension,
          repasParSemaine: currentScolarite.repasParSemaine,
          siteId: currentScolarite.siteId,
          grilleRepas: currentScolarite.grilleRepas,
        }
      : null,
    catalog,
    notesMoyennes,
    absences: absencesSynthese,
    finances: financesSynthese,
  });

  return NextResponse.json({
    eleve: profRestrictedView ? sanitizeEleveRowForProfViewer(row) : row,
    sections,
    scolarites,
    groupes: needScol ? groupesEleve : [],
    foyers: needFamille ? foyers : [],
    notes: needNotes ? notesMoyennes : [],
    competences: needNotes ? competences : [],
    absences: needVs ? absencesEleve : [],
    sanctions: needVs ? sanctionsEleve : [],
    carnet: needVs ? carnetEleve : [],
    documents,
    meta: {
      sites,
      annees,
      canEditStructure: canEditStructure(roles, { orgAdmin, platformAdmin }),
      canDecideAccess: canDecide,
      profRestrictedView,
      tiroirs: [...eleveDocTiroirsForRoles(roles, { orgAdmin, platformAdmin })],
      docCategories: eleveDocCategoriesMetaForRoles(roles, { orgAdmin, platformAdmin }),
    },
    pendingAccessRequests: pendingAccess,
    enCoursMaintenant,
    synthese,
  });
  } catch (error) {
    console.error("[eleves/dossier GET]", error);
    return NextResponse.json(
      {
        error: "Impossible de charger le dossier élève.",
        code: "DOSSIER_GET_ERROR",
        detail: error instanceof Error ? error.message : String(error),
      },
      { status: 500 },
    );
  }
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
  scolariteId?: string;
  grilleRepas?: unknown;
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
  const { loadModuleAccess } = await import("@/app/lib/module-access-store");
  const { dossierSectionsForRolesWithAccess } = await import("@/app/lib/module-access");
  const access = await loadModuleAccess();
  const sections = dossierSectionsForRolesWithAccess(
    roles,
    { orgAdmin, platformAdmin },
    access,
    { userId: authUserId, businessUserId },
  );
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

  if (action === "ensure_foyer_from_contacts") {
    if (!sections.has("famille") || !canEditStructure(roles, { orgAdmin, platformAdmin })) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    const created = await ensureEleveFoyerFromParentContacts(etabId, id, {
      nom: row.nom,
      prenom: row.prenom,
      parentEmail: row.parentEmail,
      parent1Email: row.parent1Email,
      parent2Email: row.parent2Email,
      parentPhone: row.parentPhone,
      parent1Phone: row.parent1Phone,
      parent2Phone: row.parent2Phone,
    });
    if (!created) {
      return NextResponse.json(
        { error: "Aucun contact importé exploitable, ou foyer déjà existant." },
        { status: 400 },
      );
    }
    return NextResponse.json({ success: true, created: true });
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
      payeur: body.payeurEstFoyer !== false ? false : Boolean(resp?.payeur),
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

  if (action === "update_grille_repas") {
    if (!sections.has("scolarite") || !canEditStructure(roles, { orgAdmin, platformAdmin })) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    const grille = parseEleveGrilleRepas(body.grilleRepas, { allowEmpty: true });
    if (!grille) {
      return NextResponse.json({ error: "Grille repas invalide." }, { status: 400 });
    }

    await ensureEleveScolariteGrilleRepasColumn();

    let scolariteId = String(body.scolariteId || "").trim();
    if (!scolariteId) {
      const [current] = await db
        .select({ id: eleveScolarite.id })
        .from(eleveScolarite)
        .where(
          and(
            eq(eleveScolarite.etablissementId, etabId),
            eq(eleveScolarite.eleveId, id),
            eq(eleveScolarite.statut, "en_cours"),
          ),
        )
        .limit(1);
      scolariteId = current?.id || "";
    }
    if (!scolariteId) {
      return NextResponse.json(
        { error: "Aucune scolarité en cours — créez-en une avant la grille repas." },
        { status: 400 },
      );
    }

    const midiCount = countMidiFromGrille(grille);
    const [updated] = await db
      .update(eleveScolarite)
      .set({
        grilleRepas: grille,
        repasParSemaine: midiCount,
        demiPension: midiCount > 0,
        updatedAt: new Date(),
      })
      .where(
        and(
          eq(eleveScolarite.etablissementId, etabId),
          eq(eleveScolarite.eleveId, id),
          eq(eleveScolarite.id, scolariteId),
        ),
      )
      .returning();

    if (!updated) {
      return NextResponse.json({ error: "Scolarité introuvable." }, { status: 404 });
    }

    await recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: authUserId,
      resourceType: "fiche_eleve",
      resourceId: id,
      eleveId: id,
      action: "update_grille_repas",
      metadata: { scolariteId, repasParSemaine: midiCount },
    });
    return NextResponse.json({ success: true, scolarite: updated });
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
