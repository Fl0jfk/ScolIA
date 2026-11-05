import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { edtCreneau, eleve } from "@/db/schema";
import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import {
  eleveAllowedByClassRestriction,
  professorClassRestriction,
} from "@/app/lib/brain-ai/core-read";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { listCahierTexte, listDistinctClasses } from "@/app/lib/cahier-texte-db";
import { canAccessPersonnelModule, canViewPersonnelDashboard } from "@/app/lib/personnel-types";
import {
  buildPortesOuvertesToolPayload,
  countRegistrationsBySlot,
  listPortesOuvertesRegistrations,
} from "@/app/lib/portes-ouvertes-db";
import { schoolWeekParity } from "@/app/lib/rh/planning-calendar";
import { readRhPlanning } from "@/app/lib/rh/planning-storage";
import { PLANNING_WEEKDAY_LABELS, type TeacherPlanningSlot } from "@/app/lib/rh/planning-types";
import {
  CARNET_CATEGORIES,
  listCarnetForEleve,
  searchElevesForCarnet,
} from "@/app/lib/vs-carnet-db";
import { listSanctionsForEleve, searchElevesForSanction } from "@/app/lib/vs-sanctions-db";

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

function etab(ctx: BrainToolCtx): string | null {
  return ctx.etablissementId?.trim() || null;
}

function clip(s: string | null | undefined, n: number): string {
  const t = (s || "").replace(/\s+/g, " ").trim();
  return t.length > n ? `${t.slice(0, n)}…` : t;
}

const norm = (s: string) => s.replace(/\s+/g, "").toLowerCase();

/** Classes d'un prof d'après l'EDT (même règle que core-read) ; null = pas de restriction. */
async function allowedClassesForCtx(ctx: BrainToolCtx): Promise<string[] | null> {
  const base = professorClassRestriction(ctx.roles, ctx.isOrgAdmin);
  if (base === null) return null;
  const hints = [ctx.name, [ctx.lastName, ctx.firstName].filter(Boolean).join(" ")]
    .map((s) => (s || "").trim().toLowerCase())
    .filter(Boolean);
  const id = etab(ctx);
  if (!hints.length || !id) return [];
  const rows = await getDb()
    .select({ classe: edtCreneau.classe, enseignantNom: edtCreneau.enseignantNom })
    .from(edtCreneau)
    .where(eq(edtCreneau.etablissementId, id));
  const out = new Set<string>();
  for (const r of rows) {
    const ens = (r.enseignantNom || "").trim().toLowerCase();
    if (!ens || !hints.some((h) => ens.includes(h) || h.includes(ens))) continue;
    if (r.classe?.trim()) out.add(r.classe.trim());
  }
  return [...out];
}

type EleveHit = { id: string; nom: string; prenom: string; classe: string | null };

/** Résout un élève (id explicite ou recherche) ; sinon renvoie un résultat d'erreur / de choix. */
async function resolveEleve(
  tool: string,
  args: Record<string, unknown>,
  search: (etabId: string, q: string) => Promise<EleveHit[]>,
  etabId: string,
): Promise<EleveHit | BrainToolResult> {
  const query = String(args.query || args.eleve || "").trim();
  const eleveId = String(args.eleveId || "").trim();
  if (!query && !eleveId) {
    return { ok: false, error: "Précisez l’élève (nom / prénom).", code: "INVALID_ARGS" };
  }
  // Id fourni (choix UI ou élève ouvert à l'écran) : lecture directe, limitée à l'établissement.
  // Jamais de repli silencieux sur un autre élève de la recherche.
  if (eleveId) {
    const [row] = await getDb()
      .select({ id: eleve.id, nom: eleve.nom, prenom: eleve.prenom, classe: eleve.classe })
      .from(eleve)
      .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
      .limit(1);
    if (!row) return { ok: false, error: "Élève introuvable.", code: "NOT_FOUND" };
    return row;
  }
  const hits = await search(etabId, query || String(args.queryName || ""));
  if (hits.length === 0) {
    return { ok: false, error: `Aucun élève trouvé pour « ${query} ».`, code: "NOT_FOUND" };
  }
  if (hits.length > 1) {
    return choicesResult(
      tool,
      "eleveId",
      `Plusieurs élèves pour « ${query} ». Lequel ?`,
      hits.map((h) => ({
        value: h.id,
        label: `${h.prenom} ${h.nom}${h.classe ? ` (${h.classe})` : ""}`,
      })),
      { query },
      "single",
    );
  }
  return hits[0];
}

function isResult(x: EleveHit | BrainToolResult): x is BrainToolResult {
  return "ok" in x;
}

// --- Cahier de textes -------------------------------------------------------

export async function handleGetCahierTexte(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabId = etab(ctx);
  if (!etabId) return { ok: false, error: "Établissement non résolu", code: "MISSING_ETABLISSEMENT" };

  const from = String(args.from || "").trim();
  const to = String(args.to || "").trim();
  if ((from && !ISO_DATE.test(from)) || (to && !ISO_DATE.test(to))) {
    return { ok: false, error: "Dates attendues au format AAAA-MM-JJ.", code: "INVALID_ARGS" };
  }

  let classe: string | null = null;
  const rawClasse = String(args.classe || "").trim();
  if (rawClasse) {
    const classes = await listDistinctClasses(etabId);
    classe = classes.find((c) => norm(c) === norm(rawClasse)) ?? null;
    if (!classe) {
      return {
        ok: false,
        error: `Classe « ${rawClasse} » inconnue. Classes : ${classes.slice(0, 30).join(", ")}.`,
        code: "NOT_FOUND",
      };
    }
  }

  const limit = Math.min(Math.max(Number(args.limit) || 12, 1), 30);
  const rows = await listCahierTexte(etabId, { classe, from: from || null, to: to || null, limit });
  const entries = rows.map((r) => ({
    date: r.dateSeance,
    classe: r.classe,
    matiere: r.matiereLibelle || null,
    enseignant: r.enseignantNom || null,
    contenu: clip(r.contenu, 400),
    travail: clip(r.travail, 300) || null,
    aRendreLe: r.aRendreLe || null,
  }));
  return {
    ok: true,
    data: { entries, count: entries.length, classe },
    summaryFr: entries.length
      ? `${entries.length} entrée(s) du cahier de textes.`
      : "Aucune entrée au cahier de textes pour ces critères.",
  };
}

// --- Mon planning -----------------------------------------------------------

function fmtSlots(slots: TeacherPlanningSlot[]) {
  return [...slots]
    .sort((a, b) => a.day - b.day || a.start.localeCompare(b.start))
    .map((s) => ({
      jour: PLANNING_WEEKDAY_LABELS[s.day],
      debut: s.start,
      fin: s.end,
      matiere: s.subject,
      classes: s.classes.join(", "),
      salle: s.room || null,
    }));
}

export async function handleGetMonPlanning(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  const roles = ctx.roles;
  const isTeacher = hasRole(roles, "professeur");
  // Même garde que GET /api/rh/planning.
  if (!canAccessPersonnelModule(roles) && !canViewPersonnelDashboard(roles) && !isTeacher) {
    return { ok: false, error: "Planning non disponible pour votre profil.", code: "FORBIDDEN" };
  }

  // Uniquement SON propre planning (jamais celui d'un autre).
  const doc = await readRhPlanning(isTeacher ? "teacher" : "staff", ctx.userId);
  const wanted = String(args.semaine || "").trim().toUpperCase();
  const courante = schoolWeekParity();

  if (doc.kind === "teacher") {
    const today = calendarDateKeyParis();
    const remplacements = doc.replacements
      .filter((r) => r.date >= today)
      .sort((a, b) => a.date.localeCompare(b.date))
      .slice(0, 10)
      .map((r) => ({
        date: r.date,
        debut: r.start,
        fin: r.end,
        matiere: r.subject,
        classes: r.classes.join(", "),
        salle: r.room || null,
      }));
    const data: Record<string, unknown> = { type: "professeur", semaineCourante: courante, remplacements };
    if (wanted !== "B") data.semaineA = fmtSlots(doc.weekA);
    if (wanted !== "A") data.semaineB = fmtSlots(doc.weekB);
    return { ok: true, data, summaryFr: `Votre emploi du temps (semaine ${courante} en cours).` };
  }

  const fixed = doc.fixedSlots
    .slice()
    .sort((a, b) => a.day - b.day || a.start.localeCompare(b.start))
    .map((s) => ({ jour: PLANNING_WEEKDAY_LABELS[s.day], debut: s.start, fin: s.end, libelle: s.label }));
  const rotations = doc.rotations.map((rot) => ({
    libelle: rot.label,
    du: rot.startDate || null,
    au: rot.endDate || null,
    creneaux: rot.slots
      .slice()
      .sort((a, b) => a.day - b.day || a.start.localeCompare(b.start))
      .map((s) => ({
        jour: PLANNING_WEEKDAY_LABELS[s.day],
        debut: s.start,
        fin: s.end,
        mission: s.mission,
        lieu: s.location || null,
      })),
  }));
  return {
    ok: true,
    data: { type: "personnel", mode: doc.mode, semaineCourante: courante, horairesFixes: fixed, rotations },
    summaryFr: "Votre planning.",
  };
}

// --- Sanctions / carnet d'un élève -----------------------------------------

export async function handleGetEleveSanctions(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabId = etab(ctx);
  if (!etabId) return { ok: false, error: "Établissement non résolu", code: "MISSING_ETABLISSEMENT" };

  const hit = await resolveEleve("get_eleve_sanctions", args, searchElevesForSanction, etabId);
  if (isResult(hit)) return hit;

  // Module vs-sanctions : pas de restriction de classe côté route (rôles vie scolaire / direction) ;
  // on l'applique malgré tout si le rôle n'est qu'enseignant.
  const allowed = await allowedClassesForCtx(ctx);
  if (!eleveAllowedByClassRestriction(hit.classe, allowed)) {
    return { ok: false, error: "Cet élève n’est pas dans vos classes.", code: "FORBIDDEN" };
  }

  const rows = await listSanctionsForEleve(etabId, hit.id, { limit: 20 });
  return {
    ok: true,
    data: {
      eleve: { nom: hit.nom, prenom: hit.prenom, classe: hit.classe },
      sanctions: rows.map((s) => ({
        date: s.dateSanction,
        type: s.typeLibelle,
        gravite: s.gravite,
        motif: clip(s.motif, 200) || null,
      })),
    },
    summaryFr: `${rows.length} sanction(s) active(s) pour ${hit.prenom} ${hit.nom}.`,
  };
}

export async function handleGetEleveCarnet(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabId = etab(ctx);
  if (!etabId) return { ok: false, error: "Établissement non résolu", code: "MISSING_ETABLISSEMENT" };

  const hit = await resolveEleve("get_eleve_carnet", args, searchElevesForCarnet, etabId);
  if (isResult(hit)) return hit;

  // Le module carnet est ouvert aux professeurs : restriction à leurs classes.
  const allowed = await allowedClassesForCtx(ctx);
  if (!eleveAllowedByClassRestriction(hit.classe, allowed)) {
    return { ok: false, error: "Cet élève n’est pas dans vos classes.", code: "FORBIDDEN" };
  }

  const rows = await listCarnetForEleve(etabId, hit.id, { limit: 15 });
  const label = (id: string) => CARNET_CATEGORIES.find((c) => c.id === id)?.label ?? id;
  return {
    ok: true,
    data: {
      eleve: { nom: hit.nom, prenom: hit.prenom, classe: hit.classe },
      entrees: rows.map((e) => ({
        date: e.dateEntree,
        categorie: label(e.categorie),
        titre: e.titre,
        texte: clip(e.corps, 300),
        parAuteur: e.createdByNom || null,
        luParFamille: Boolean(e.signeAt),
      })),
    },
    summaryFr: `${rows.length} entrée(s) du carnet de liaison de ${hit.prenom} ${hit.nom}.`,
  };
}

// --- Événements (portes ouvertes) ------------------------------------------

export async function handleListEvenements(
  ctx: BrainToolCtx,
  _args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etabId = etab(ctx);
  if (!etabId) return { ok: false, error: "Établissement non résolu", code: "MISSING_ETABLISSEMENT" };

  const [payload, regs] = await Promise.all([
    buildPortesOuvertesToolPayload(etabId),
    listPortesOuvertesRegistrations(etabId),
  ]);
  const counts = countRegistrationsBySlot(regs);
  const horizon = Date.now() - 24 * 3600 * 1000;
  const slots = payload.slots
    .filter((s) => {
      const t = Date.parse(s.startAt);
      return Number.isNaN(t) || t >= horizon;
    })
    .sort((a, b) => a.startAt.localeCompare(b.startAt))
    .slice(0, 25)
    .map((s) => ({
      creneau: s.label,
      debut: s.startAt,
      fin: s.endAt,
      cycle: s.cycle || null,
      inscrits: counts[s.id] || 0,
      places: s.maxPlaces ?? null,
      equipe: payload.staff
        .filter((st) => st.slotId === s.id)
        .map((st) => `${st.displayName} (${st.role})`),
    }));

  return {
    ok: true,
    data: {
      portesOuvertes: {
        titre: payload.title,
        adresse: payload.address || null,
        creneauxAVenir: slots,
      },
    },
    summaryFr: slots.length
      ? `${slots.length} créneau(x) de portes ouvertes à venir.`
      : "Aucun créneau de portes ouvertes à venir.",
  };
}
