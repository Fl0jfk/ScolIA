import {
  canAccessIntranetPath,
  INTRANET_MODULES,
  rolesAllowModule,
} from "@/app/lib/intranet-modules";
import type { BrainToolCtx, BrainToolDefinition } from "@/app/lib/brain-ai/types";

/** Codes de refus d’accès (ne jamais contourner côté modèle / UI). */
export const BRAIN_PERMISSION_DENIED_CODES = new Set([
  "AUTH_REQUIRED",
  "MODULE_FORBIDDEN",
  "FORBIDDEN",
]);

export function isBrainPermissionDenied(code: string | undefined): boolean {
  return Boolean(code && BRAIN_PERMISSION_DENIED_CODES.has(code));
}

function getIntranetModuleById(moduleId: string) {
  return INTRANET_MODULES.find((m) => m.id === moduleId) ?? null;
}

function moduleLabel(tool: BrainToolDefinition): string | null {
  if (!tool.moduleId) return null;
  const mod = getIntranetModuleById(tool.moduleId);
  const name = mod?.dashboard?.name?.trim() || "";
  return name || null;
}

/**
 * Formulation unique pour les refus RBAC — claire, sans contournement possible.
 */
export function brainPermissionDeniedMessage(opts: {
  code: string;
  moduleTitle?: string | null;
  detail?: string | null;
}): string {
  if (opts.code === "AUTH_REQUIRED") {
    return "Connectez-vous pour utiliser ScolIA sur cette action.";
  }

  const detail = opts.detail?.trim();
  if (detail) {
    // Détail métier déjà précis (ex. validateur OGEC) — on l’encadre.
    if (/autoris|droit|restreint|habilit|réserv/i.test(detail)) {
      return `${detail.replace(/\.*\s*$/, "")}. ScolIA applique les mêmes droits que l’intranet.`;
    }
    return `${detail.replace(/\.*\s*$/, "")}. Vous n’avez pas les droits pour cette action — ScolIA ne peut pas la contourner.`;
  }

  const mod = opts.moduleTitle?.trim();
  if (mod) {
    return `Vous n’êtes pas autorisé à accéder à « ${mod} ». Cette action est restreinte selon votre profil — ScolIA respecte les mêmes droits que l’intranet.`;
  }

  return "Vous n’êtes pas autorisé à effectuer cette action. Elle est restreinte selon votre profil dans l’établissement — ScolIA ne peut pas contourner vos droits.";
}

export function assertToolPermissions(
  ctx: BrainToolCtx,
  tool: BrainToolDefinition,
): { ok: true } | { ok: false; error: string; code: string } {
  if (tool.requiresAuth && !ctx.userId) {
    return {
      ok: false,
      error: brainPermissionDeniedMessage({ code: "AUTH_REQUIRED" }),
      code: "AUTH_REQUIRED",
    };
  }

  const title = moduleLabel(tool);

  if (tool.pathPrefix) {
    if (!canAccessIntranetPath(tool.pathPrefix, ctx.roles, ctx.isOrgAdmin)) {
      return {
        ok: false,
        error: brainPermissionDeniedMessage({
          code: "MODULE_FORBIDDEN",
          moduleTitle: title,
        }),
        code: "MODULE_FORBIDDEN",
      };
    }
  } else if (tool.moduleId) {
    const mod = getIntranetModuleById(tool.moduleId);
    if (mod && !rolesAllowModule(ctx.roles, mod, ctx.isOrgAdmin)) {
      return {
        ok: false,
        error: brainPermissionDeniedMessage({
          code: "MODULE_FORBIDDEN",
          moduleTitle: title || mod.id,
        }),
        code: "MODULE_FORBIDDEN",
      };
    }
  }

  return { ok: true };
}

/** Normalise un refus handler (FORBIDDEN / MODULE_FORBIDDEN) vers la formulation standard. */
export function normalizeBrainDeniedResult(error: string, code?: string): {
  error: string;
  code: string;
} {
  const c = code && BRAIN_PERMISSION_DENIED_CODES.has(code) ? code : "FORBIDDEN";
  return {
    code: c,
    error: brainPermissionDeniedMessage({ code: c, detail: error }),
  };
}
