import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { edtCreneau, eleve, eleveScolarite, etablissement } from "@/db/schema";
import {
  eleveAllowedByClassRestriction,
  envelope,
  explainOccupancyTag,
  professorClassRestriction,
  shapeEleveForRoles,
} from "@/app/lib/brain-ai/core-read";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { parseEleveGrilleRepas } from "@/app/lib/eleve-grille-repas";
import { listSitesFromDb } from "@/app/lib/ent-core-db";
import { searchElevesRegistry, loadElevesRegistry } from "@/app/lib/eleves-registry";
import { getPresenceJour } from "@/app/lib/occupancy";
import { getTravelFromDb } from "@/app/lib/travel-db";
import {
  jourSemaineFromIsoDate,
  listEdtCreneauxForJour,
} from "@/app/lib/vs-calendrier-db";

function requireEtab(ctx: BrainToolCtx): string | BrainToolResult {
  const id = ctx.etablissementId?.trim() || "";
  if (!id) {
    return { ok: false, error: "Établissement non résolu", code: "MISSING_ETABLISSEMENT" };
  }
  return id;
}

async function resolveAllowedClasses(ctx: BrainToolCtx): Promise<string[] | null> {
  const base = professorClassRestriction(ctx.roles, ctx.isOrgAdmin);
  if (base === null) return null;

  const nameHints = [ctx.name, [ctx.lastName, ctx.firstName].filter(Boolean).join(" ")]
    .map((s) => (s || "").trim().toLowerCase())
    .filter(Boolean);

  if (nameHints.length === 0 || !ctx.etablissementId) return base;

  const db = getDb();
  const rows = await db
    .select({ classe: edtCreneau.classe, enseignantNom: edtCreneau.enseignantNom })
    .from(edtCreneau)
    .where(eq(edtCreneau.etablissementId, ctx.etablissementId));

  const classes = new Set<string>();
  for (const r of rows) {
    const ens = (r.enseignantNom || "").trim().toLowerCase();
    if (!ens) continue;
    if (!nameHints.some((h) => ens.includes(h) || h.includes(ens))) continue;
    if (r.classe?.trim()) classes.add(r.classe.trim());
  }
  return [...classes];
}

export async function handleGetTenantContext(
  ctx: BrainToolCtx,
  _args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabOrErr = requireEtab(ctx);
  if (typeof etabOrErr !== "string") return etabOrErr;
  const etablissementId = etabOrErr;

  const db = getDb();
  const [row] = await db
    .select({
      id: etablissement.id,
      slug: etablissement.slug,
      name: etablissement.name,
    })
    .from(etablissement)
    .where(eq(etablissement.id, etablissementId))
    .limit(1);

  const sites = await listSitesFromDb(etablissementId).catch(() => []);

  const facts = envelope({
    etablissementId,
    slug: row?.slug ?? ctx.tenantSlug ?? null,
    name: row?.name ?? null,
    sites: sites.map((s) => ({
      id: s.id,
      label: s.label,
      kind: s.kind,
    })),
    hasInternatSite: sites.some((s) => /internat/i.test(String(s.kind || s.label || ""))),
  });

  return {
    ok: true,
    data: facts,
    summaryFr: `Tenant ${facts.facts.slug || etablissementId} — ${sites.length} site(s).`,
  };
}

export async function handleSearchEleves(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabOrErr = requireEtab(ctx);
  if (typeof etabOrErr !== "string") return etabOrErr;

  const query = typeof args.query === "string" ? args.query.trim() : "";
  if (!query) {
    return { ok: false, error: "query requise (nom / prénom / INE / classe)", code: "MISSING_QUERY" };
  }

  const limit = Math.min(Math.max(Number(args.limit) || 12, 1), 30);
  const allowedClasses = await resolveAllowedClasses(ctx);
  const hits = await searchElevesRegistry(query, limit * 3);
  const shaped = hits
    .filter((e) => eleveAllowedByClassRestriction(e.classe, allowedClasses))
    .slice(0, limit)
    .map((e) => shapeEleveForRoles(e, ctx.roles, ctx.isOrgAdmin));

  if (shaped.length === 0) {
    return {
      ok: true,
      data: envelope({ eleves: [], query }, "partial"),
      summaryFr: `Aucun élève pour « ${query} » (droits / classe).`,
    };
  }

  if (shaped.length > 1 && /thomas|dupont|martin/i.test(query) === false) {
    // Homonymes / multi-match → choices pour l’IA
  }

  const needsChoices = shaped.length > 1;
  if (needsChoices) {
    return {
      ok: false,
      needsChoices: true,
      tool: "get_eleve",
      field: "eleveId",
      promptFr: `Plusieurs élèves pour « ${query} ». Lequel ?`,
      options: shaped.map((e) => ({
        value: e.id || e.ine || `${e.nom}|${e.prenom}`,
        label: `${e.prenom} ${e.nom}${e.classe ? ` (${e.classe})` : ""}`,
      })),
      draftArgs: { query },
      selectionType: "single",
    };
  }

  return {
    ok: true,
    data: envelope({ eleves: shaped, query }),
    summaryFr: `1 élève : ${shaped[0].prenom} ${shaped[0].nom}${shaped[0].classe ? ` (${shaped[0].classe})` : ""}.`,
  };
}

export async function handleGetEleve(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabOrErr = requireEtab(ctx);
  if (typeof etabOrErr !== "string") return etabOrErr;

  const eleveId = typeof args.eleveId === "string" ? args.eleveId.trim() : "";
  const query = typeof args.query === "string" ? args.query.trim() : "";

  let found: Awaited<ReturnType<typeof loadElevesRegistry>>[number] | undefined;
  const all = await loadElevesRegistry();
  if (eleveId) {
    found = all.find((e) => e.id === eleveId || e.ine === eleveId);
  } else if (query) {
    const hits = await searchElevesRegistry(query, 8);
    if (hits.length > 1) {
      return handleSearchEleves(ctx, { query, limit: 8 });
    }
    found = hits[0];
  }

  if (!found) {
    return { ok: false, error: "Élève introuvable", code: "ELEVE_NOT_FOUND" };
  }

  const allowedClasses = await resolveAllowedClasses(ctx);
  if (!eleveAllowedByClassRestriction(found.classe, allowedClasses)) {
    return {
      ok: false,
      error: "Élève hors de vos classes — accès refusé.",
      code: "ELEVE_CLASS_FORBIDDEN",
    };
  }

  const shaped = shapeEleveForRoles(found, ctx.roles, ctx.isOrgAdmin);
  return {
    ok: true,
    data: envelope(shaped),
    summaryFr: `${shaped.prenom} ${shaped.nom}${shaped.classe ? ` — ${shaped.classe}` : ""}${shaped.regime ? ` · ${shaped.regime}` : ""}.`,
  };
}

export async function handleGetPresenceJour(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabOrErr = requireEtab(ctx);
  if (typeof etabOrErr !== "string") return etabOrErr;
  const etablissementId = etabOrErr;

  const date =
    typeof args.date === "string" && /^\d{4}-\d{2}-\d{2}$/.test(args.date.trim())
      ? args.date.trim()
      : calendarDateKeyParis();
  const eleveId = typeof args.eleveId === "string" ? args.eleveId.trim() : "";
  const classe = typeof args.classe === "string" ? args.classe.trim() : "";
  const query = typeof args.query === "string" ? args.query.trim() : "";

  let resolvedEleveId = eleveId;
  if (!resolvedEleveId && query) {
    const hits = await searchElevesRegistry(query, 5);
    const allowedClasses = await resolveAllowedClasses(ctx);
    const filtered = hits.filter((e) => eleveAllowedByClassRestriction(e.classe, allowedClasses));
    if (filtered.length > 1) {
      return handleSearchEleves(ctx, { query, limit: 5 });
    }
    if (filtered.length === 1 && filtered[0].id) {
      resolvedEleveId = filtered[0].id;
    } else if (filtered.length === 1) {
      return {
        ok: false,
        error: "Élève sans id Postgres — impossible de lire occupancy.",
        code: "ELEVE_ID_MISSING",
      };
    }
  }

  if (!resolvedEleveId && !classe) {
    return {
      ok: false,
      error: "Indiquez eleveId, query (nom) ou classe.",
      code: "MISSING_TARGET",
    };
  }

  if (resolvedEleveId) {
    const all = await loadElevesRegistry();
    const one = all.find((e) => e.id === resolvedEleveId);
    const allowedClasses = await resolveAllowedClasses(ctx);
    if (one && !eleveAllowedByClassRestriction(one.classe, allowedClasses)) {
      return {
        ok: false,
        error: "Élève hors de vos classes — présence refusée.",
        code: "ELEVE_CLASS_FORBIDDEN",
      };
    }
  }

  const result = await getPresenceJour({
    etablissementId,
    date,
    ...(resolvedEleveId ? { eleveId: resolvedEleveId } : {}),
    ...(classe && !resolvedEleveId ? { classe } : {}),
  });

  const facts = result.facts.map((f) => ({
    eleveId: f.eleveId,
    date: f.date,
    tag: f.tag,
    source: f.source,
    explanationFr: explainOccupancyTag(f.tag),
    detail: f.detail ?? null,
    coverage: f.coverage,
    confidenceTag: f.confidenceTag,
  }));

  const summary =
    facts.length === 0
      ? `Aucune présence calculée pour ${date}.`
      : facts.length === 1
        ? `${facts[0].explanationFr} (source ${facts[0].source}).`
        : `${facts.length} élève(s) le ${date} — ex. ${facts[0].tag}.`;

  return {
    ok: true,
    data: envelope(
      {
        date,
        facts,
        unmatchedParticipants: result.unmatchedParticipants,
      },
      result.coverage,
      "defined",
    ),
    summaryFr: summary,
  };
}

export async function handleGetEdt(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabOrErr = requireEtab(ctx);
  if (typeof etabOrErr !== "string") return etabOrErr;
  const etablissementId = etabOrErr;

  const date =
    typeof args.date === "string" && /^\d{4}-\d{2}-\d{2}$/.test(args.date.trim())
      ? args.date.trim()
      : calendarDateKeyParis();
  const classe = typeof args.classe === "string" ? args.classe.trim() : "";
  const jour = jourSemaineFromIsoDate(date);

  const rows = await listEdtCreneauxForJour(
    etablissementId,
    jour,
    classe ? { classe } : undefined,
  );

  const facts = envelope(
    {
      date,
      jourSemaine: jour,
      source: "edt_creneau",
      sourceNoteFr:
        "Grille ScolIA (`edt_creneau`). Non opposable STS tant que la source officielle n’est pas tranchée.",
      creneaux: rows.map((r) => ({
        id: r.id,
        heureDebut: r.heureDebut,
        heureFin: r.heureFin,
        classe: r.classe,
        groupeId: r.groupeId,
        enseignantNom: r.enseignantNom,
        salle: r.salle,
        matiere: r.matiereLibelle,
        semaine: r.semaine,
      })),
    },
    rows.length > 0 ? "complete" : "partial",
    "deduced",
  );

  return {
    ok: true,
    data: facts,
    summaryFr:
      rows.length === 0
        ? `Aucun créneau EDT le ${date}${classe ? ` pour ${classe}` : ""}.`
        : `${rows.length} créneau(x) le ${date}${classe ? ` (${classe})` : ""}.`,
  };
}

export async function handleGetVoyage(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabOrErr = requireEtab(ctx);
  if (typeof etabOrErr !== "string") return etabOrErr;
  const etablissementId = etabOrErr;

  const tripId = typeof args.tripId === "string" ? args.tripId.trim() : "";
  if (!tripId) {
    return { ok: false, error: "tripId requis", code: "MISSING_TRIP" };
  }

  const trip = await getTravelFromDb(etablissementId, tripId);
  if (!trip) {
    return { ok: false, error: "Voyage introuvable", code: "TRIP_NOT_FOUND" };
  }

  // Accueil : pas de fiche détail travels (défini architecture).
  if (ctx.roles.includes("accueil") && !ctx.isOrgAdmin && ctx.roles.length === 1) {
    return {
      ok: true,
      data: envelope(
        {
          id: trip.id,
          title: trip.data?.title ?? null,
          status: trip.status,
          detailForbidden: true,
        },
        "partial",
      ),
      summaryFr: `Séjour « ${trip.data?.title || tripId} » — détail fiche non accessible à l’accueil.`,
    };
  }

  const participants = Array.isArray(trip.data?.participantEleves)
    ? trip.data.participantEleves.map((p) => ({
        eleveId: p.eleveId ?? null,
        ine: p.ine,
        nom: p.nom,
        prenom: p.prenom,
        snapshotClasse: p.classe ?? null,
        panierRepas: p.panierRepas === true,
      }))
    : [];

  return {
    ok: true,
    data: envelope({
      id: trip.id,
      status: trip.status,
      type: trip.type,
      title: trip.data?.title ?? null,
      destination: trip.data?.destination ?? null,
      startDate: trip.data?.startDate ?? trip.data?.date ?? null,
      endDate: trip.data?.endDate ?? trip.data?.startDate ?? trip.data?.date ?? null,
      classes: trip.data?.classes ?? null,
      listeElevesStatus: trip.data?.listeElevesStatus ?? null,
      participants,
      participantLinkedCount: participants.filter((p) => p.eleveId).length,
    }),
    summaryFr: `Séjour « ${trip.data?.title || tripId} » (${trip.status}) — ${participants.length} participant(s).`,
  };
}

export async function handleGetGrilleRepas(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabOrErr = requireEtab(ctx);
  if (typeof etabOrErr !== "string") return etabOrErr;
  const etablissementId = etabOrErr;

  const eleveId = typeof args.eleveId === "string" ? args.eleveId.trim() : "";
  if (!eleveId) {
    return { ok: false, error: "eleveId requis", code: "MISSING_ELEVE" };
  }

  const db = getDb();
  const [scol] = await db
    .select({
      grilleRepas: eleveScolarite.grilleRepas,
      classe: eleveScolarite.classe,
    })
    .from(eleveScolarite)
    .where(
      and(
        eq(eleveScolarite.etablissementId, etablissementId),
        eq(eleveScolarite.eleveId, eleveId),
        eq(eleveScolarite.statut, "en_cours"),
      ),
    )
    .limit(1);

  const [plat] = await db
    .select({ regime: eleve.regime, classe: eleve.classe })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);

  if (!plat && !scol) {
    return { ok: false, error: "Élève introuvable", code: "ELEVE_NOT_FOUND" };
  }

  const grille = parseEleveGrilleRepas(scol?.grilleRepas, { allowEmpty: true });

  return {
    ok: true,
    data: envelope(
      {
        eleveId,
        classe: scol?.classe ?? plat?.classe ?? null,
        regime: plat?.regime ?? null,
        grille,
        opsSelf: null,
        opsNoteFr: "Prévision self / menus = unavailable (module resto ops absent).",
      },
      grille ? "partial" : "unavailable",
      grille ? "defined" : "unknown",
    ),
    summaryFr: grille
      ? `Grille repas connue (droit élève) — ops cantine unavailable.`
      : `Pas de grille repas scolarité — coverage unavailable.`,
  };
}
