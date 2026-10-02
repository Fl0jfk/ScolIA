import "server-only";

import type {
  BrainClientAction,
  BrainCta,
  BrainDocCatalog,
  BrainDocCatalogGroup,
  BrainDocCatalogItem,
  BrainToolCtx,
  BrainToolResult,
} from "@/app/lib/brain-ai/types";
import { canOpenEleveDossierDetail } from "@/app/lib/accueil-access";
import { listEleveLatestAccompagnementByKind } from "@/app/lib/eleve-dossier-access";
import { eleveDocumentFileProxyPath } from "@/app/lib/eleve-document-file";
import { listElevesDossierFromDb } from "@/app/lib/eleve-dossier-prof";
import type { AccompagnementKind } from "@/app/lib/eleve-pap";
import { ACCOMPAGNEMENT_KINDS, accompagnementKindDef } from "@/app/lib/eleve-pap";
import {
  detectSchoolPoleQuery,
  schoolClassBelongsToPole,
  schoolClassesMatch,
  schoolPoleLabel,
  resolveSchoolClassQuery,
} from "@/app/lib/school-classes-catalog";
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

function formatDocYear(anneeLabel: string | null, createdAt: Date): string {
  const label = String(anneeLabel || "").trim();
  if (label) return label;
  const y = createdAt.getFullYear();
  if (!Number.isFinite(y) || y < 2000) return "PDF";
  return `${y}-${y + 1}`;
}

function compareClasseLabel(a: string, b: string): number {
  return a.localeCompare(b, "fr", { numeric: true, sensitivity: "base" });
}

function compareEleveName(
  a: { nom: string; prenom: string },
  b: { nom: string; prenom: string },
): number {
  const byNom = a.nom.localeCompare(b.nom, "fr", { sensitivity: "base" });
  if (byNom !== 0) return byNom;
  return a.prenom.localeCompare(b.prenom, "fr", { sensitivity: "base" });
}

type EleveDocLink = {
  kind: AccompagnementKind;
  documentId: string;
  fileHref: string;
  anneeLabel: string | null;
  createdAt: Date;
};

/**
 * Liste des élèves filtrés (classe / pôle + accompagnement PAP/PAI/PPS…).
 * Renvoie un catalogue groupé par classe (cartes PDF cliquables).
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

  const classe = String(args.classe || args.className || args.pole || "").trim();
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
      promptFr: "Pour quelle classe ou quel pôle ? (ex. 6ème A, Collège)",
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
  let resolvedPole = detectSchoolPoleQuery(classe);
  let filterScopeLabel = "";

  if (classe) {
    if (resolvedPole) {
      eleves = eleves.filter((e) => schoolClassBelongsToPole(e.classe, resolvedPole!));
      filterScopeLabel = schoolPoleLabel(resolvedPole);
    } else {
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
            .sort(compareClasseLabel)
            .slice(0, 30)
            .map((c) => ({ value: c, label: c })),
          { accompagnement: kindRaw || kind || "any" },
        );
      } else if (resolved.fold && /^[3-6]E$/.test(resolved.fold)) {
        resolvedClasse = classe;
      }
      eleves = eleves.filter((e) => schoolClassesMatch(e.classe, resolvedClasse));
      filterScopeLabel = `classe ${resolvedClasse}`;
    }
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
      anneeLabel: i.anneeLabel,
      createdAt: i.createdAt,
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

  rows.sort((a, b) => {
    const byClasse = compareClasseLabel(String(a.classe || "—"), String(b.classe || "—"));
    if (byClasse !== 0) return byClasse;
    return compareEleveName(a, b);
  });

  const focusKind: AccompagnementKind | null = kind && kind !== "any" ? kind : null;
  const focusCode = focusKind ? accompagnementKindDef(focusKind).code : null;

  // Pas de plafond artificiel : le catalogue UI gère le volume (groupé + scroll).
  const catalogItemsByClasse = new Map<string, BrainDocCatalogItem[]>();

  for (const r of rows) {
    const classeKey = String(r.classe || "").trim() || "Sans classe";
    const docs =
      focusKind != null
        ? r.documents.filter((d) => d.kind === focusKind)
        : r.documents;

    const bucket = catalogItemsByClasse.get(classeKey) ?? [];
    if (docs.length === 0) {
      bucket.push({
        title: `${r.prenom} ${r.nom}`,
        subtitle: "Fiche élève",
        href: r.dossierHref,
        dossierHref: r.dossierHref,
      });
    } else {
      for (const doc of docs) {
        const code = accompagnementKindDef(doc.kind).code;
        bucket.push({
          title: `${code} · ${r.prenom} ${r.nom}`,
          subtitle: formatDocYear(doc.anneeLabel, doc.createdAt),
          href: doc.fileHref,
          preview: true,
          dossierHref: r.dossierHref,
          ext: "pdf",
        });
      }
    }
    catalogItemsByClasse.set(classeKey, bucket);
  }

  const groups: BrainDocCatalogGroup[] = [...catalogItemsByClasse.entries()]
    .sort(([a], [b]) => compareClasseLabel(a, b))
    .map(([title, items]) => ({
      title,
      count: items.length,
      items,
    }));

  const totalDocs = groups.reduce((acc, g) => acc + g.count, 0);
  const catalogTitle = [
    totalDocs > 0
      ? `${totalDocs} ${focusCode ? focusCode : "document(s)"}`
      : `Aucun ${focusCode || "document"}`,
    filterScopeLabel || null,
  ]
    .filter(Boolean)
    .join(" — ");

  const docCatalog: BrainDocCatalog = {
    title: catalogTitle,
    kindLabel: focusCode || undefined,
    total: totalDocs,
    groups,
  };

  // CTAs plats : uniquement pour les petites listes (repli UI / historique léger).
  const ctas: BrainCta[] = [];
  if (totalDocs > 0 && totalDocs <= 8) {
    for (const g of groups) {
      for (const item of g.items) {
        ctas.push({
          label: item.title,
          href: item.href,
          ...(item.preview ? { preview: true as const } : {}),
          ...(item.subtitle ? { subtitle: item.subtitle } : {}),
          group: g.title,
        });
      }
    }
  }

  const clientActions: BrainClientAction[] = [];
  if (rows.length === 1) {
    const alone = rows[0]!;
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

  const classSummary =
    groups.length > 1
      ? groups.map((g) => `${g.title} (${g.count})`).join(" · ")
      : groups[0]
        ? `${groups[0].title} — ${groups[0].count} élève(s)`
        : "";

  const summaryFr =
    totalDocs === 0
      ? `Aucun élève${filterScopeLabel ? ` (${filterScopeLabel})` : ""}${focusCode ? ` avec ${focusCode}` : ""}.`
      : [
          `${totalDocs} ${focusCode || "document(s)"}${filterScopeLabel ? ` — ${filterScopeLabel}` : ""}.`,
          classSummary ? `Par classe : ${classSummary}.` : "",
          "Ouvrez un document ci-dessous (aperçu PDF).",
        ]
          .filter(Boolean)
          .join("\n");

  return {
    ok: true,
    data: {
      total: rows.length,
      eleves: rows.map((r) => ({
        id: r.id,
        nom: r.nom,
        prenom: r.prenom,
        classe: r.classe,
        accompagnementKinds: r.accompagnementKinds,
        hasPap: r.hasPap,
        hasPai: r.hasPai,
        hasPps: r.hasPps,
        label: r.label,
        dossierHref: r.dossierHref,
      })),
      docCatalog,
      ...(ctas.length ? { ctas } : {}),
      clientActions,
    },
    summaryFr,
  };
}
