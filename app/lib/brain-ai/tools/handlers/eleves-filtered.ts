import "server-only";

import type { BrainClientAction, BrainCta, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { canOpenEleveDossierDetail } from "@/app/lib/accueil-access";
import { listEleveLatestAccompagnementByKind } from "@/app/lib/eleve-dossier-access";
import { eleveDocumentFileProxyPath } from "@/app/lib/eleve-document-file";
import { listElevesDossierFromDb } from "@/app/lib/eleve-dossier-prof";
import type { AccompagnementKind } from "@/app/lib/eleve-pap";
import { ACCOMPAGNEMENT_KINDS, accompagnementKindDef } from "@/app/lib/eleve-pap";
import { schoolClassesMatch, resolveSchoolClassQuery } from "@/app/lib/school-classes-catalog";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { choicesResult } from "@/app/lib/brain-ai/choice-options";

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

type EleveDocLink = {
  kind: AccompagnementKind;
  documentId: string;
  fileHref: string;
};

/**
 * Liste des élèves filtrés (classe + accompagnement PAP/PAI/PPS…).
 * CTAs : ouverture directe du document (aperçu) + lien fiche élève.
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
  });

  let resolvedClasse = classe;
  if (classe) {
    const known = [
      ...new Set(eleves.map((e) => String(e.classe || "").trim()).filter(Boolean)),
    ];
    const resolved = resolveSchoolClassQuery(classe, known);
    if (resolved.match) {
      resolvedClasse = resolved.match;
    } else if (resolved.ambiguous.length > 0) {
      return choicesResult(
        "list_eleves_filtered",
        "classe",
        `Plusieurs classes correspondent à « ${classe} ». Laquelle ?`,
        resolved.ambiguous
          .sort((a, b) => a.localeCompare(b, "fr", { numeric: true }))
          .slice(0, 30)
          .map((c) => ({ value: c, label: c })),
        { accompagnement: kindRaw || kind || "any" },
      );
    } else if (resolved.fold && /^[3-6]E$/.test(resolved.fold)) {
      resolvedClasse = classe;
    }
    eleves = eleves.filter((e) => schoolClassesMatch(e.classe, resolvedClasse));
  }

  const accompagnementByEleve = await listEleveLatestAccompagnementByKind({
    etablissementId: etabId,
    eleveIds: eleves.map((e) => e.id),
  });

  const kindOrder = ACCOMPAGNEMENT_KINDS.map((k) => k.kind);
  let rows = eleves.map((e) => {
    const items = accompagnementByEleve.get(e.id) ?? [];
    const kinds = kindOrder.filter((k) => items.some((i) => i.kind === k));
    const documents: EleveDocLink[] = items.map((i) => ({
      kind: i.kind,
      documentId: i.documentId,
      fileHref: eleveDocumentFileProxyPath(e.id, i.documentId),
    }));
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
      documents,
    };
  });

  if (kind && kind !== "any") {
    rows = rows.filter((r) => r.accompagnementKinds.includes(kind));
  }

  const limit = Math.min(Math.max(Number(args.limit) || 40, 1), 80);
  const sliced = rows.slice(0, limit);
  const focusKind: AccompagnementKind | null = kind && kind !== "any" ? kind : null;
  const focusCode = focusKind ? accompagnementKindDef(focusKind).code : null;

  const ctas: BrainCta[] = [];
  for (const r of sliced.slice(0, 12)) {
    if (focusKind && focusCode) {
      const doc = r.documents.find((d) => d.kind === focusKind);
      if (doc) {
        ctas.push({
          label: `${focusCode} · ${r.prenom} ${r.nom}`,
          href: doc.fileHref,
          preview: true,
        });
      } else {
        ctas.push({
          label: `Fiche · ${r.prenom} ${r.nom}`,
          href: r.dossierHref,
        });
      }
      continue;
    }

    for (const doc of r.documents.slice(0, 2)) {
      const code = accompagnementKindDef(doc.kind).code;
      ctas.push({
        label: `${code} · ${r.prenom} ${r.nom}`,
        href: doc.fileHref,
        preview: true,
      });
    }
    if (r.documents.length === 0) {
      ctas.push({
        label: `Fiche · ${r.prenom} ${r.nom}`,
        href: r.dossierHref,
      });
    }
  }

  const clientActions: BrainClientAction[] = [];
  if (sliced.length === 1) {
    const alone = sliced[0]!;
    const doc =
      (focusKind ? alone.documents.find((d) => d.kind === focusKind) : null) ??
      alone.documents[0] ??
      null;
    if (doc && focusKind) {
      clientActions.push({
        type: "open_url_modal",
        href: doc.fileHref,
        title: `${accompagnementKindDef(doc.kind).code} — ${alone.prenom} ${alone.nom}`,
      });
    } else {
      clientActions.push({ type: "open_eleve_dossier", eleveId: alone.id, subView: "dossier" });
    }
  }

  const filterLabel = [
    resolvedClasse ? `classe ${resolvedClasse}` : null,
    focusCode,
  ]
    .filter(Boolean)
    .join(" · ");

  const listPreview = sliced
    .slice(0, 12)
    .map((e) => {
      if (focusCode) {
        return `• ${e.prenom} ${e.nom}${e.classe ? ` (${e.classe})` : ""}`;
      }
      const codes = e.documents.map((d) => accompagnementKindDef(d.kind).code).join(", ");
      return codes
        ? `• ${e.prenom} ${e.nom}${e.classe ? ` · ${e.classe}` : ""} — ${codes}`
        : `• ${e.label}`;
    })
    .join("\n");

  return {
    ok: true,
    data: {
      total: rows.length,
      eleves: sliced,
      ctas: ctas.slice(0, 20),
      clientActions,
    },
    summaryFr:
      sliced.length === 0
        ? `Aucun élève${filterLabel ? ` (${filterLabel})` : ""}.`
        : `${rows.length} élève(s)${filterLabel ? ` — ${filterLabel}` : ""} :\n${listPreview}${
            rows.length > 12 ? "\n…" : ""
          }`,
  };
}
