import "server-only";

import { and, eq } from "drizzle-orm";
import { declareAccueilAbsence } from "@/app/lib/accueil-absences-db";
import type { AccueilPeriodMode } from "@/app/lib/accueil-absences-types";
import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import {
  buildDateQuickOptions,
  wizardStep,
  WIZARD_DATE_OTHER,
} from "@/app/lib/brain-ai/wizard";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { eleve } from "@/db/schema";

function fold(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

async function searchEleves(
  etablissementId: string,
  query: string,
  limit = 8,
): Promise<Array<{ id: string; nom: string; prenom: string; classe: string | null }>> {
  const db = getDb();
  const parts = query.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return [];
  const { ilike, or } = await import("drizzle-orm");
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
    .where(and(eq(eleve.etablissementId, etablissementId), or(...nameConds)))
    .orderBy(eleve.nom, eleve.prenom)
    .limit(40);

  const foldedQuery = fold(query);
  return rows
    .map((r) => {
      const full = fold(`${r.prenom} ${r.nom}`);
      const rev = fold(`${r.nom} ${r.prenom}`);
      let score = 0;
      if (full === foldedQuery || rev === foldedQuery) score += 20;
      if (full.includes(foldedQuery) || rev.includes(foldedQuery)) score += 10;
      for (const p of parts) {
        const fp = fold(p);
        if (fold(r.nom).includes(fp) || fold(r.prenom).includes(fp)) score += 3;
      }
      return { r, score };
    })
    .filter((x) => x.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map((x) => x.r);
}

function addDays(dateKey: string, days: number): string {
  const [y, m, d] = dateKey.split("-").map(Number);
  const next = new Date(Date.UTC(y!, m! - 1, d! + days, 12, 0, 0));
  return next.toISOString().slice(0, 10);
}

/**
 * Déclaration absence / retard élève à l’accueil (jour J, plage multi-jours, ou heures).
 */
export async function handleCreateAccueilAbsence(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }
  if (!isDatabaseConfigured()) {
    return { ok: false, error: "Base indisponible.", code: "DB" };
  }
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return { ok: false, error: "Établissement introuvable." };

  const natureRaw = String(args.eleveNature || args.nature || "absence").trim().toLowerCase();
  const eleveNature = natureRaw === "retard" ? "retard" : "absence";

  let subjectId = String(args.subjectId || args.eleveId || "").trim();
  const query = String(args.query || args.q || args.name || "").trim();

  if (!subjectId && query) {
    const hits = await searchEleves(etabId, query);
    if (hits.length === 0) {
      return { ok: false, error: `Aucun élève pour « ${query} ».` };
    }
    if (hits.length > 1) {
      return {
        ok: false,
        needsChoices: true,
        tool: "create_accueil_absence",
        field: "subjectId",
        promptFr: "Quel élève est concerné ?",
        options: hits.map((e) => ({
          value: e.id,
          label: `${e.prenom} ${e.nom}${e.classe ? ` — ${e.classe}` : ""}`,
        })),
        draftArgs: { ...args, query, eleveNature },
        selectionType: "single",
      };
    }
    subjectId = hits[0]!.id;
  }

  if (!subjectId) {
    return choicesResult(
      "create_accueil_absence",
      "query",
      "Nom de l’élève absent / en retard ?",
      [],
      { ...args, eleveNature },
      "text",
    );
  }

  const today = calendarDateKeyParis();
  let startDate = String(args.startDate || args.date || "").trim();
  let endDate = String(args.endDate || "").trim();
  let mode = String(args.mode || "").trim() as AccueilPeriodMode | "";

  if (!mode) {
    const daysHint = Number(args.days || args.nbJours || 0);
    if (Number.isFinite(daysHint) && daysHint > 1) {
      mode = "multi_day";
      if (!startDate) startDate = today;
      if (!endDate) endDate = addDays(startDate, Math.round(daysHint) - 1);
    } else if (eleveNature === "retard") {
      mode = "hours";
    }
  }

  if (!startDate) {
    const quick = buildDateQuickOptions(today);
    return choicesResult(
      "create_accueil_absence",
      "startDate",
      wizardStep(1, 4, "Date de début de l’absence ?"),
      [
        ...quick.map((o) => ({ value: o.value, label: o.label })),
        { value: WIZARD_DATE_OTHER, label: "Autre date…" },
      ],
      { ...args, subjectId, eleveNature, mode: mode || undefined },
      "date",
    );
  }
  if (startDate === WIZARD_DATE_OTHER) {
    return choicesResult(
      "create_accueil_absence",
      "startDate",
      "Indiquez la date (AAAA-MM-JJ) :",
      [],
      { ...args, subjectId, eleveNature, mode: mode || undefined },
      "date",
    );
  }

  if (!mode) {
    return choicesResult(
      "create_accueil_absence",
      "mode",
      wizardStep(2, 4, "Quelle durée ?"),
      [
        { value: "today", label: "Aujourd’hui uniquement" },
        { value: "multi_day", label: "Plusieurs jours" },
        { value: "hours", label: "Créneau horaires" },
      ],
      { ...args, subjectId, eleveNature, startDate },
    );
  }

  if (mode === "multi_day" && !endDate) {
    const daysHint = Number(args.days || args.nbJours || 0);
    if (Number.isFinite(daysHint) && daysHint > 1) {
      endDate = addDays(startDate, Math.round(daysHint) - 1);
    } else {
      return choicesResult(
        "create_accueil_absence",
        "endDate",
        wizardStep(3, 4, "Date de fin ?"),
        [],
        { ...args, subjectId, eleveNature, startDate, mode },
        "date",
      );
    }
  }

  if (mode === "today") {
    endDate = startDate;
  }
  if (mode === "hours") {
    endDate = startDate;
  }

  const startTime = String(args.startTime || "").trim() || null;
  const endTime = String(args.endTime || "").trim() || null;
  if (mode === "hours" && (!startTime || !endTime)) {
    return choicesResult(
      "create_accueil_absence",
      "startTime",
      "Heure de début (HH:MM) ?",
      [
        { value: "08:00", label: "08:00" },
        { value: "09:00", label: "09:00" },
        { value: "10:00", label: "10:00" },
        { value: "13:30", label: "13:30" },
      ],
      { ...args, subjectId, eleveNature, startDate, endDate, mode, endTime: endTime || "08:30" },
      "text",
    );
  }

  const motif = String(args.motif || "").trim() || "Absence déclarée à l’accueil";
  const canalRaw = String(args.canal || "telephone").trim();
  const canal =
    canalRaw === "physique" || canalRaw === "mail" ? canalRaw : "telephone";

  const db = getDb();
  const [row] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, subjectId)))
    .limit(1);
  if (!row) return { ok: false, error: "Élève introuvable." };

  const periodLabel =
    mode === "multi_day"
      ? `du ${startDate} au ${endDate}`
      : mode === "hours"
        ? `le ${startDate} de ${startTime} à ${endTime}`
        : `le ${startDate}`;

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_accueil_absence",
      args: {
        subjectId,
        eleveNature,
        mode,
        startDate,
        endDate,
        startTime,
        endTime,
        motif,
        canal,
      },
      summaryFr:
        `Déclarer ${eleveNature === "retard" ? "un retard" : "une absence"} pour ` +
        `${row.prenom} ${row.nom}${row.classe ? ` (${row.classe})` : ""} ${periodLabel}.` +
        `\nMotif : ${motif} · Canal : ${canal}.`,
    };
  }

  const created = await declareAccueilAbsence(etabId, {
    kind: "eleve",
    subjectId,
    mode,
    startDate,
    endDate: endDate || startDate,
    startTime,
    endTime,
    motif,
    canal,
    eleveNature,
    actor: {
      userId: ctx.userId,
      name:
        [ctx.firstName, ctx.lastName].filter(Boolean).join(" ") ||
        ctx.name ||
        "Accueil",
      email: ctx.email || "",
      roles: ctx.roles,
    },
  });

  return {
    ok: true,
    data: {
      ...created,
      clientActions: [
        { type: "open_route" as const, href: "/vie-scolaire/absences", label: "Voir le board absences" },
      ],
      ctas: [{ label: "Board absences accueil", href: "/vie-scolaire/absences" }],
    },
    summaryFr:
      `${eleveNature === "retard" ? "Retard" : "Absence"} déclaré(e) pour ${created.displayName} ${periodLabel}.`,
  };
}
