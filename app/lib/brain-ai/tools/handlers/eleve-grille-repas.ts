import "server-only";

import { and, eq } from "drizzle-orm";
import {
  countMidiFromGrille,
  emptyGrilleRepas,
  MEAL_DAY_ORDER,
  type EleveGrilleRepas,
  type MealDayKey,
} from "@/app/lib/eleve-grille-repas";
import { canManageElevePreinscriptions } from "@/app/lib/eleve-dossier-scope";
import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import type { BrainClientAction, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { eleve, eleveScolarite } from "@/db/schema";

function canEdit(ctx: BrainToolCtx): boolean {
  return canManageElevePreinscriptions({
    roles: ctx.roles,
    orgAdmin: ctx.isOrgAdmin,
    platformAdmin: false,
  });
}

function fold(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

async function resolveEleveId(
  etabId: string,
  args: Record<string, unknown>,
  tool: string,
): Promise<
  | { ok: true; id: string; nom: string; prenom: string; classe: string | null }
  | { ok: false; result: BrainToolResult }
> {
  let eleveId = String(args.eleveId || "").trim();
  const query = String(args.query || args.q || args.name || "").trim();
  const db = getDb();

  if (!eleveId && query) {
    const { ilike, or } = await import("drizzle-orm");
    const parts = query.split(/\s+/).filter(Boolean);
    const patterns = parts.map((p) => `%${p}%`);
    const nameConds = patterns.flatMap((p) => [ilike(eleve.nom, p), ilike(eleve.prenom, p)]);
    const rows = await db
      .select({
        id: eleve.id,
        nom: eleve.nom,
        prenom: eleve.prenom,
        classe: eleve.classe,
      })
      .from(eleve)
      .where(and(eq(eleve.etablissementId, etabId), or(...nameConds)))
      .orderBy(eleve.nom, eleve.prenom)
      .limit(40);
    const folded = fold(query);
    const scored = rows
      .map((r) => {
        const full = fold(`${r.prenom} ${r.nom}`);
        let score = full.includes(folded) ? 10 : 0;
        for (const p of parts) {
          if (fold(r.nom).includes(fold(p)) || fold(r.prenom).includes(fold(p))) score += 3;
        }
        return { r, score };
      })
      .filter((x) => x.score > 0)
      .sort((a, b) => b.score - a.score);
    if (scored.length === 0) {
      return { ok: false, result: { ok: false, error: `Aucun élève pour « ${query} ».` } };
    }
    if (scored.length > 1) {
      return {
        ok: false,
        result: {
          ok: false,
          needsChoices: true,
          tool,
          field: "eleveId",
          promptFr: "Quel élève ?",
          options: scored.slice(0, 10).map((x) => ({
            value: x.r.id,
            label: `${x.r.prenom} ${x.r.nom}${x.r.classe ? ` — ${x.r.classe}` : ""}`,
          })),
          draftArgs: { ...args, query },
          selectionType: "single",
        },
      };
    }
    eleveId = scored[0]!.r.id;
  }

  if (!eleveId) {
    return {
      ok: false,
      result: choicesResult(tool, "query", "Nom de l’élève ?", [], { ...args }, "text"),
    };
  }

  const [row] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!row) return { ok: false, result: { ok: false, error: "Élève introuvable." } };
  return { ok: true, ...row };
}

function grilleMidiDays(count: number): EleveGrilleRepas {
  const g = emptyGrilleRepas();
  const n = Math.min(Math.max(Math.round(count), 0), 5);
  for (let i = 0; i < n; i += 1) {
    g[MEAL_DAY_ORDER[i]!.key].midi = true;
  }
  return g;
}

function grilleFromDayKeys(keys: MealDayKey[], soir = false): EleveGrilleRepas {
  const g = emptyGrilleRepas();
  for (const k of keys) {
    g[k].midi = true;
    g[k].soir = soir;
  }
  return g;
}

function parseDayKeys(raw: string): MealDayKey[] {
  const map: Record<string, MealDayKey> = {
    lun: "lun",
    lundi: "lun",
    mar: "mar",
    mardi: "mar",
    mer: "mer",
    mercredi: "mer",
    jeu: "jeu",
    jeudi: "jeu",
    ven: "ven",
    vendredi: "ven",
  };
  const out: MealDayKey[] = [];
  for (const tok of raw.toLowerCase().split(/[\s,;+/]+/).filter(Boolean)) {
    const k = map[tok];
    if (k && !out.includes(k)) out.push(k);
  }
  return out;
}

/**
 * Met à jour la grille repas (midi / soir) d’un élève.
 * Raccourcis : repasParSemaine=3 | days=lun,mar,jeu | preset=semaine_midi
 */
export async function handleUpdateEleveGrilleRepas(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  if (!canEdit(ctx)) {
    return { ok: false, error: "Modification réservée direction / admin.", code: "FORBIDDEN" };
  }
  if (!isDatabaseConfigured()) return { ok: false, error: "Base indisponible.", code: "DB" };
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return { ok: false, error: "Établissement introuvable." };

  const resolved = await resolveEleveId(etabId, args, "update_eleve_grille_repas");
  if (!resolved.ok) return resolved.result;

  let grille: EleveGrilleRepas | null = null;
  const preset = String(args.preset || "").trim().toLowerCase();
  const daysRaw = String(args.days || args.jours || "").trim();
  const repasN = Number(args.repasParSemaine ?? args.nbRepas ?? args.repas ?? NaN);
  const soir = Boolean(args.soir);
  const presetAsNumber = Number(preset);

  if (preset === "semaine_midi" || preset === "dp5" || preset === "demi_pension") {
    grille = grilleMidiDays(5);
  } else if (preset === "externe" || preset === "zero") {
    grille = emptyGrilleRepas();
  } else if (preset === "interne_soir" || preset === "interne") {
    grille = grilleFromDayKeys(["lun", "mar", "mer", "jeu"], true);
    for (const k of ["lun", "mar", "mer", "jeu", "ven"] as MealDayKey[]) {
      grille[k].midi = true;
    }
  } else if (Number.isFinite(presetAsNumber) && preset !== "") {
    grille = grilleMidiDays(presetAsNumber);
  } else if (daysRaw) {
    const keys = parseDayKeys(daysRaw);
    if (keys.length === 0) {
      return {
        ok: false,
        error: "Jours non reconnus (ex. lun,mar,jeu).",
      };
    }
    grille = grilleFromDayKeys(keys, soir);
  } else if (Number.isFinite(repasN)) {
    grille = grilleMidiDays(repasN);
  } else {
    return choicesResult(
      "update_eleve_grille_repas",
      "preset",
      `Combien de repas midi pour ${resolved.prenom} ${resolved.nom} ?`,
      [
        { value: "3", label: "3 midi / semaine (lun–mer)" },
        { value: "4", label: "4 midi / semaine" },
        { value: "5", label: "5 midi (demi-pension complète)" },
        { value: "0", label: "Aucun (externe)" },
      ],
      { eleveId: resolved.id, query: `${resolved.prenom} ${resolved.nom}` },
    );
  }

  if (!grille) {
    return { ok: false, error: "Grille repas non déterminée." };
  }

  const midiCount = countMidiFromGrille(grille);
  const summaryDays = MEAL_DAY_ORDER.filter((d) => grille![d.key].midi)
    .map((d) => d.label)
    .join(", ");

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "update_eleve_grille_repas",
      args: {
        eleveId: resolved.id,
        repasParSemaine: midiCount,
        days: MEAL_DAY_ORDER.filter((d) => grille![d.key].midi)
          .map((d) => d.key)
          .join(","),
        soir,
      },
      summaryFr:
        `Grille repas de ${resolved.prenom} ${resolved.nom}` +
        `${resolved.classe ? ` (${resolved.classe})` : ""} → ${midiCount} midi` +
        (summaryDays ? ` (${summaryDays})` : "") +
        (soir ? " + soirs" : "") +
        ".",
    };
  }

  const db = getDb();
  const { ensureEleveScolariteGrilleRepasColumn } = await import(
    "@/app/lib/eleve-scolarite-schema"
  );
  await ensureEleveScolariteGrilleRepasColumn();

  const [current] = await db
    .select({ id: eleveScolarite.id })
    .from(eleveScolarite)
    .where(
      and(
        eq(eleveScolarite.etablissementId, etabId),
        eq(eleveScolarite.eleveId, resolved.id),
        eq(eleveScolarite.statut, "en_cours"),
      ),
    )
    .limit(1);
  if (!current) {
    return {
      ok: false,
      error: "Aucune scolarité en cours — créez-en une avant la grille repas.",
    };
  }

  await db
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
        eq(eleveScolarite.eleveId, resolved.id),
        eq(eleveScolarite.id, current.id),
      ),
    );

  const openAction: BrainClientAction = {
    type: "open_eleve_dossier",
    eleveId: resolved.id,
    subView: "dossier",
  };

  return {
    ok: true,
    data: {
      eleveId: resolved.id,
      repasParSemaine: midiCount,
      clientActions: [openAction],
    },
    summaryFr: `Grille repas mise à jour pour ${resolved.prenom} ${resolved.nom} (${midiCount} midi).`,
  };
}
