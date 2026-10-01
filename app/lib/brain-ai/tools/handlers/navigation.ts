import "server-only";

import {
  findScoliaDestinationById,
  searchScoliaDestinations,
} from "@/app/lib/brain-ai/destination-catalog";
import type { BrainClientAction, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { canAccessIntranetPath } from "@/app/lib/intranet-modules";

function canOpenHref(ctx: BrainToolCtx, href: string): boolean {
  const path = href.split("?")[0] || href;
  return canAccessIntranetPath(path, ctx.roles, ctx.isOrgAdmin);
}

export async function handleResolveAndOpen(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }

  const destinationId = String(args.destinationId || "").trim();
  const query = String(args.query || args.q || "").trim();
  const hrefArg = String(args.href || "").trim();

  if (hrefArg.startsWith("/") && !hrefArg.startsWith("//")) {
    if (!canOpenHref(ctx, hrefArg)) {
      return { ok: false, error: "Accès refusé à cette page.", code: "FORBIDDEN" };
    }
    const action: BrainClientAction = {
      type: "open_route",
      href: hrefArg,
      label: String(args.label || "Ouvrir"),
    };
    return {
      ok: true,
      data: { clientActions: [action], href: hrefArg },
      summaryFr: `J’ouvre ${hrefArg}.`,
    };
  }

  if (destinationId) {
    const dest = findScoliaDestinationById(destinationId);
    if (!dest) {
      return { ok: false, error: `Destination inconnue : ${destinationId}` };
    }
    if (!canOpenHref(ctx, dest.href)) {
      return { ok: false, error: `Accès refusé à « ${dest.label} ».`, code: "FORBIDDEN" };
    }
    const action: BrainClientAction = {
      type: "open_route",
      href: dest.href,
      label: dest.label,
    };
    return {
      ok: true,
      data: { clientActions: [action], destination: dest },
      summaryFr: `J’ouvre « ${dest.label} ».`,
    };
  }

  if (!query) {
    const top = searchScoliaDestinations("", 12).filter((d) => canOpenHref(ctx, d.href));
    return {
      ok: false,
      needsChoices: true,
      tool: "resolve_and_open",
      field: "destinationId",
      promptFr: "Quelle page voulez-vous ouvrir ?",
      options: top.map((d) => ({ value: d.id, label: d.label })),
      draftArgs: {},
      selectionType: "single",
    };
  }

  const hits = searchScoliaDestinations(query, 10).filter((d) => canOpenHref(ctx, d.href));
  if (hits.length === 0) {
    return {
      ok: false,
      error: `Aucune page trouvée pour « ${query} ».`,
      code: "NOT_FOUND",
    };
  }
  if (hits.length === 1) {
    const dest = hits[0]!;
    const action: BrainClientAction = {
      type: "open_route",
      href: dest.href,
      label: dest.label,
    };
    return {
      ok: true,
      data: { clientActions: [action], destination: dest },
      summaryFr: `J’ouvre « ${dest.label} ».`,
    };
  }

  return {
    ok: false,
    needsChoices: true,
    tool: "resolve_and_open",
    field: "destinationId",
    promptFr: `Plusieurs pages correspondent à « ${query} ». Laquelle ouvrir ?`,
    options: hits.map((d) => ({ value: d.id, label: `${d.label} (${d.href})` })),
    draftArgs: { query },
    selectionType: "single",
  };
}

export async function handleListDestinations(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }
  const query = String(args.query || "").trim();
  const hits = searchScoliaDestinations(query, 20).filter((d) => canOpenHref(ctx, d.href));
  return {
    ok: true,
    data: {
      destinations: hits.map((d) => ({
        id: d.id,
        label: d.label,
        href: d.href,
        moduleId: d.moduleId,
      })),
    },
    summaryFr:
      hits.length === 0
        ? "Aucune destination accessible."
        : `${hits.length} page(s) : ${hits
            .slice(0, 6)
            .map((d) => d.label)
            .join(" · ")}.`,
  };
}
