import "server-only";

import { and, eq, ilike, or } from "drizzle-orm";
import type { BrainClientAction, BrainCta, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { canOpenEleveDossierDetail } from "@/app/lib/accueil-access";
import { listEleveLatestAccompagnementByKind } from "@/app/lib/eleve-dossier-access";
import { eleveDocumentFileProxyPath } from "@/app/lib/eleve-document-file";
import { canManageElevePreinscriptions } from "@/app/lib/eleve-dossier-scope";
import type { AccompagnementKind } from "@/app/lib/eleve-pap";
import { ACCOMPAGNEMENT_KINDS, accompagnementKindDef } from "@/app/lib/eleve-pap";
import { canonicalRegimeLabel, classifyRegime } from "@/app/lib/eleve-regime";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { eleve, eleveScolarite } from "@/db/schema";

function fold(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

function parseDocumentKind(raw: string): AccompagnementKind | null {
  const s = raw.trim().toLowerCase();
  if (!s) return null;
  if (s === "pap" || s === "pai" || s === "pps" || s === "gevasco") return s;
  const hit = ACCOMPAGNEMENT_KINDS.find(
    (k) => k.kind === s || k.code.toLowerCase() === s || k.label.toLowerCase().includes(s),
  );
  return hit?.kind ?? null;
}

function canViewDossiers(ctx: BrainToolCtx): boolean {
  return canOpenEleveDossierDetail({
    roles: ctx.roles,
    orgAdmin: ctx.isOrgAdmin,
    platformAdmin: false,
  });
}

function canEditDossiers(ctx: BrainToolCtx): boolean {
  return canManageElevePreinscriptions({
    roles: ctx.roles,
    orgAdmin: ctx.isOrgAdmin,
    platformAdmin: false,
  });
}

async function requireEtabId(): Promise<string | null> {
  if (!isDatabaseConfigured()) return null;
  return resolveCurrentEtablissementId();
}

async function searchElevesByQuery(
  etablissementId: string,
  query: string,
  limit = 8,
): Promise<
  Array<{
    id: string;
    nom: string;
    prenom: string;
    classe: string | null;
    regime: string | null;
  }>
> {
  const db = getDb();
  const q = query.trim();
  if (!q) return [];
  const parts = q.split(/\s+/).filter(Boolean);
  const patterns = parts.map((p) => `%${p}%`);

  const nameConds = patterns.flatMap((p) => [
    ilike(eleve.nom, p),
    ilike(eleve.prenom, p),
  ]);

  const rows = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      regime: eleve.regime,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), or(...nameConds)))
    .orderBy(eleve.nom, eleve.prenom)
    .limit(40);

  const foldedQuery = fold(q);
  const scored = rows
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
    .sort((a, b) => b.score - a.score);

  return scored.slice(0, limit).map((x) => x.r);
}

export async function handleSearchEleves(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  if (!canViewDossiers(ctx)) {
    return { ok: false, error: "Accès dossiers élèves refusé.", code: "FORBIDDEN" };
  }
  const etabId = await requireEtabId();
  if (!etabId) return { ok: false, error: "Base indisponible.", code: "DB" };

  const query = String(args.query || args.q || args.name || "").trim();
  if (!query) {
    return { ok: false, error: "Indiquez un nom / prénom d’élève." };
  }
  const items = await searchElevesByQuery(etabId, query);
  return {
    ok: true,
    data: {
      eleves: items.map((e) => ({
        id: e.id,
        label: `${e.prenom} ${e.nom}${e.classe ? ` (${e.classe})` : ""}`,
        nom: e.nom,
        prenom: e.prenom,
        classe: e.classe,
        regime: e.regime,
        dossierHref: `/eleves/dossier/${e.id}`,
        inscriptionHref: `/eleves/dossier/${e.id}/inscription`,
      })),
    },
    summaryFr:
      items.length === 0
        ? `Aucun élève pour « ${query} ».`
        : `${items.length} élève(s) : ${items
            .slice(0, 5)
            .map((e) => `${e.prenom} ${e.nom}`)
            .join(" · ")}.`,
  };
}

export async function handleOpenEleveDossier(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  if (!canViewDossiers(ctx)) {
    return { ok: false, error: "Accès dossiers élèves refusé.", code: "FORBIDDEN" };
  }
  const etabId = await requireEtabId();
  if (!etabId) return { ok: false, error: "Base indisponible.", code: "DB" };

  const subView =
    String(args.subView || args.view || "").trim() === "inscription"
      ? "inscription"
      : "dossier";
  const documentKind = parseDocumentKind(
    String(args.documentKind || args.accompagnement || args.kind || args.document || "").trim(),
  );

  let eleveId = String(args.eleveId || args.id || "").trim();
  const query = String(args.query || args.q || args.name || "").trim();

  if (!eleveId && query) {
    const hits = await searchElevesByQuery(etabId, query);
    if (hits.length === 0) {
      return { ok: false, error: `Aucun élève pour « ${query} ».` };
    }
    if (hits.length > 1) {
      return {
        ok: false,
        needsChoices: true,
        tool: "open_eleve_dossier",
        field: "eleveId",
        promptFr: "Quel élève voulez-vous ouvrir ?",
        options: hits.map((e) => ({
          value: e.id,
          label: `${e.prenom} ${e.nom}${e.classe ? ` — ${e.classe}` : ""}`,
        })),
        draftArgs: {
          subView,
          query,
          ...(documentKind ? { documentKind } : {}),
        },
        selectionType: "single",
      };
    }
    eleveId = hits[0]!.id;
  }

  if (!eleveId) {
    return {
      ok: false,
      needsChoices: true,
      tool: "open_eleve_dossier",
      field: "query",
      promptFr: documentKind
        ? `Nom de l’élève dont ouvrir le ${accompagnementKindDef(documentKind).code} ?`
        : "Nom de l’élève à ouvrir ?",
      options: [],
      draftArgs: {
        subView,
        ...(documentKind ? { documentKind } : {}),
      },
      selectionType: "text",
    };
  }

  const db = getDb();
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
  if (!row) return { ok: false, error: "Élève introuvable.", code: "NOT_FOUND" };

  if (documentKind) {
    const byEleve = await listEleveLatestAccompagnementByKind({
      etablissementId: etabId,
      eleveIds: [row.id],
    });
    const doc = (byEleve.get(row.id) ?? []).find((d) => d.kind === documentKind);
    const code = accompagnementKindDef(documentKind).code;
    if (!doc) {
      return {
        ok: false,
        error: `Aucun ${code} trouvé pour ${row.prenom} ${row.nom}.`,
        code: "NOT_FOUND",
      };
    }
    const fileHref = eleveDocumentFileProxyPath(row.id, doc.documentId);
    const dossierHref = `/eleves/dossier/${row.id}`;
    const clientActions: BrainClientAction[] = [
      {
        type: "open_url_modal",
        href: fileHref,
        title: `${code} — ${row.prenom} ${row.nom}`,
      },
    ];
    const ctas: BrainCta[] = [
      { label: `${code} · ${row.prenom} ${row.nom}`, href: fileHref, preview: true },
      { label: `Fiche · ${row.prenom} ${row.nom}`, href: dossierHref },
    ];
    return {
      ok: true,
      data: {
        clientActions,
        ctas,
        eleve: row,
        documentKind,
        fileHref,
        href: dossierHref,
      },
      summaryFr: `J’ouvre le ${code} de ${row.prenom} ${row.nom}.`,
    };
  }

  const action: BrainClientAction = {
    type: "open_eleve_dossier",
    eleveId: row.id,
    subView,
  };
  const label =
    subView === "inscription"
      ? `documents d’inscription de ${row.prenom} ${row.nom}`
      : `dossier de ${row.prenom} ${row.nom}`;

  return {
    ok: true,
    data: {
      clientActions: [action],
      eleve: row,
      href:
        subView === "inscription"
          ? `/eleves/dossier/${row.id}/inscription`
          : `/eleves/dossier/${row.id}`,
    },
    summaryFr: `J’ouvre les ${label}.`,
  };
}

function regimeTargetFromArgs(args: Record<string, unknown>): {
  kind: "interne" | "demi_pension" | "externe";
  label: string;
  demiPension: boolean;
} | null {
  const raw = String(args.regime || args.target || args.value || "").trim();
  if (!raw) return null;
  const kind = classifyRegime(raw);
  if (kind === "inconnu") {
    const f = fold(raw);
    if (f.includes("interne")) {
      return {
        kind: "interne",
        label: canonicalRegimeLabel("3") || "Interne dans l’établissement",
        demiPension: true,
      };
    }
    if (f.includes("demi") || f === "dp") {
      return {
        kind: "demi_pension",
        label: canonicalRegimeLabel("2") || "Demi-pensionnaire dans l’établissement",
        demiPension: true,
      };
    }
    if (f.includes("externe")) {
      return {
        kind: "externe",
        label: canonicalRegimeLabel("0") || "Externe libre",
        demiPension: false,
      };
    }
    return null;
  }
  return {
    kind,
    label:
      canonicalRegimeLabel(kind === "interne" ? "3" : kind === "demi_pension" ? "2" : "0") ||
      raw,
    demiPension: kind !== "externe",
  };
}

export async function handleUpdateEleveRegime(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  if (!canEditDossiers(ctx)) {
    return { ok: false, error: "Modification régime réservée à la direction / admin.", code: "FORBIDDEN" };
  }
  const etabId = await requireEtabId();
  if (!etabId) return { ok: false, error: "Base indisponible.", code: "DB" };

  const target = regimeTargetFromArgs(args);
  if (!target) {
    return {
      ok: false,
      needsChoices: true,
      tool: "update_eleve_regime",
      field: "regime",
      promptFr: "Nouveau régime ?",
      options: [
        { value: "interne", label: "Interne" },
        { value: "demi_pension", label: "Demi-pensionnaire" },
        { value: "externe", label: "Externe" },
      ],
      draftArgs: {
        eleveId: args.eleveId,
        query: args.query || args.name,
      },
      selectionType: "single",
    };
  }

  let eleveId = String(args.eleveId || "").trim();
  const query = String(args.query || args.q || args.name || "").trim();
  if (!eleveId && query) {
    const hits = await searchElevesByQuery(etabId, query);
    if (hits.length === 0) return { ok: false, error: `Aucun élève pour « ${query} ».` };
    if (hits.length > 1) {
      return {
        ok: false,
        needsChoices: true,
        tool: "update_eleve_regime",
        field: "eleveId",
        promptFr: "Quel élève ?",
        options: hits.map((e) => ({
          value: e.id,
          label: `${e.prenom} ${e.nom}${e.classe ? ` — ${e.classe}` : ""}${
            e.regime ? ` (${e.regime})` : ""
          }`,
        })),
        draftArgs: { regime: target.kind, query },
        selectionType: "single",
      };
    }
    eleveId = hits[0]!.id;
  }
  if (!eleveId) {
    return {
      ok: false,
      needsChoices: true,
      tool: "update_eleve_regime",
      field: "query",
      promptFr: "Nom de l’élève ?",
      options: [],
      draftArgs: { regime: target.kind },
      selectionType: "text",
    };
  }

  const db = getDb();
  const [row] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      regime: eleve.regime,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!row) return { ok: false, error: "Élève introuvable.", code: "NOT_FOUND" };

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "update_eleve_regime",
      args: {
        eleveId: row.id,
        regime: target.kind,
        query: `${row.prenom} ${row.nom}`,
      },
      summaryFr: `Passer ${row.prenom} ${row.nom}${row.classe ? ` (${row.classe})` : ""} de « ${
        row.regime || "non renseigné"
      } » à « ${target.label} ».${
        target.kind === "interne"
          ? " (Le dossier s’ouvrira ensuite — affectation chambre internat à vérifier si besoin.)"
          : ""
      }`,
    };
  }

  await db
    .update(eleve)
    .set({ regime: target.label, updatedAt: new Date() })
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, row.id)));

  await db
    .update(eleveScolarite)
    .set({ demiPension: target.demiPension, updatedAt: new Date() })
    .where(
      and(
        eq(eleveScolarite.etablissementId, etabId),
        eq(eleveScolarite.eleveId, row.id),
        eq(eleveScolarite.statut, "en_cours"),
      ),
    );

  const openAction: BrainClientAction = {
    type: "open_eleve_dossier",
    eleveId: row.id,
    subView: "dossier",
  };

  return {
    ok: true,
    data: {
      eleveId: row.id,
      regime: target.label,
      kind: target.kind,
      clientActions: [openAction],
      note:
        target.kind === "interne"
          ? "Régime mis à jour. Vérifiez l’affectation internat si l’élève doit dormir sur place."
          : undefined,
    },
    summaryFr: `Régime de ${row.prenom} ${row.nom} mis à jour → ${target.label}.`,
  };
}
