import "server-only";

import type { BrainClientAction, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { canOpenEleveDossierDetail } from "@/app/lib/accueil-access";
import { listEleveLatestAccompagnementByKind } from "@/app/lib/eleve-dossier-access";
import { listElevesDossierFromDb } from "@/app/lib/eleve-dossier-prof";
import type { AccompagnementKind } from "@/app/lib/eleve-pap";
import { ACCOMPAGNEMENT_KINDS } from "@/app/lib/eleve-pap";
import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getDb, isDatabaseConfigured } from "@/db/index";

function canView(ctx: BrainToolCtx): boolean {
  return canOpenEleveDossierDetail({
    roles: ctx.roles,
    orgAdmin: ctx.isOrgAdmin,
    platformAdmin: false,
  });
}

function parseAccompKind(raw: string): AccompagnementKind | "any" | null {
  const s = raw.trim().toLowerCase();
  if (!s || s === "all" || s === "tous") return "any";
  if (s === "pap") return "pap";
  if (s === "pai") return "pai";
  if (s === "pps") return "pps";
  if (s === "gevasco") return "gevasco";
  const hit = ACCOMPAGNEMENT_KINDS.find(
    (k) => k.kind === s || k.code.toLowerCase() === s || k.label.toLowerCase().includes(s),
  );
  return hit?.kind ?? null;
}

/**
 * Liste des élèves filtrés (classe + accompagnement PAP/PAI/PPS…).
 */
export async function handleListElevesFiltered(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  if (!canView(ctx)) {
    return { ok: false, error: "Accès dossiers élèves refusé.", code: "FORBIDDEN" };
  }
  if (!isDatabaseConfigured()) {
    return { ok: false, error: "Base indisponible.", code: "DB" };
  }
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return { ok: false, error: "Établissement introuvable." };

  const classe = String(args.classe || args.className || "").trim();
  const kindRaw = String(args.accompagnement || args.kind || args.has || "").trim();
  const kind = kindRaw ? parseAccompKind(kindRaw) : "any";
  if (kindRaw && kind === null) {
    return {
      ok: false,
      needsChoices: true,
      tool: "list_eleves_filtered",
      field: "accompagnement",
      promptFr: "Quel type d’accompagnement ?",
      options: [
        { value: "pap", label: "PAP" },
        { value: "pai", label: "PAI" },
        { value: "pps", label: "PPS" },
        { value: "gevasco", label: "GEVASCO" },
        { value: "any", label: "Tous (avec ou sans)" },
      ],
      draftArgs: { classe },
      selectionType: "single",
    };
  }

  if (!classe && kind === "any") {
    return {
      ok: false,
      needsChoices: true,
      tool: "list_eleves_filtered",
      field: "classe",
      promptFr: "Pour quelle classe ? (ex. 6ème A)",
      options: [],
      draftArgs: { accompagnement: kindRaw || "any" },
      selectionType: "text",
    };
  }

  void getDb();
  let eleves = await listElevesDossierFromDb(etabId, {
    status: "inscrit",
    classe: classe || undefined,
  });

  if (classe) {
    eleves = eleves.filter((e) => schoolClassesMatch(e.classe, classe));
  }

  const accompagnementByEleve = await listEleveLatestAccompagnementByKind({
    etablissementId: etabId,
    eleveIds: eleves.map((e) => e.id),
  });

  const kindOrder = ACCOMPAGNEMENT_KINDS.map((k) => k.kind);
  let rows = eleves.map((e) => {
    const items = accompagnementByEleve.get(e.id) ?? [];
    const kinds = kindOrder.filter((k) => items.some((i) => i.kind === k));
    return {
      id: e.id,
      nom: e.nom,
      prenom: e.prenom,
      classe: e.classe,
      accompagnementKinds: kinds,
      hasPap: kinds.includes("pap"),
      hasPai: kinds.includes("pai"),
      hasPps: kinds.includes("pps"),
      label: `${e.prenom} ${e.nom}${e.classe ? ` (${e.classe})` : ""}`,
      dossierHref: `/eleves/dossier/${e.id}`,
    };
  });

  if (kind && kind !== "any") {
    rows = rows.filter((r) => r.accompagnementKinds.includes(kind));
  }

  const limit = Math.min(Math.max(Number(args.limit) || 40, 1), 80);
  const sliced = rows.slice(0, limit);

  const ctas = sliced.slice(0, 12).map((r) => ({
    label: r.label,
    href: r.dossierHref,
  }));

  const clientActions: BrainClientAction[] =
    sliced.length === 1
      ? [{ type: "open_eleve_dossier", eleveId: sliced[0]!.id, subView: "dossier" }]
      : [];

  const filterLabel = [
    classe ? `classe ${classe}` : null,
    kind && kind !== "any" ? kind.toUpperCase() : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return {
    ok: true,
    data: {
      total: rows.length,
      eleves: sliced,
      ctas,
      clientActions,
    },
    summaryFr:
      sliced.length === 0
        ? `Aucun élève${filterLabel ? ` (${filterLabel})` : ""}.`
        : `${rows.length} élève(s)${filterLabel ? ` — ${filterLabel}` : ""} : ${sliced
            .slice(0, 8)
            .map((e) => e.label)
            .join(" · ")}${rows.length > 8 ? "…" : ""}.`,
  };
}
