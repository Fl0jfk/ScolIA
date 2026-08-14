import {
  canAccessIntranetPath,
  INTRANET_MODULES,
  rolesAllowModule,
} from "@/app/lib/intranet-modules";
import type { BrainToolCtx, BrainToolDefinition } from "@/app/lib/brain-ai/types";

function getIntranetModuleById(moduleId: string) {
  return INTRANET_MODULES.find((m) => m.id === moduleId) ?? null;
}

export function assertToolPermissions(
  ctx: BrainToolCtx,
  tool: BrainToolDefinition,
): { ok: true } | { ok: false; error: string; code: string } {
  if (tool.requiresAuth && !ctx.userId) {
    return {
      ok: false,
      error: "Connectez-vous pour utiliser cette action.",
      code: "AUTH_REQUIRED",
    };
  }

  if (tool.pathPrefix) {
    if (!canAccessIntranetPath(tool.pathPrefix, ctx.roles, ctx.isOrgAdmin)) {
      return {
        ok: false,
        error: "Vous n'avez pas accès à ce module pour cette action.",
        code: "MODULE_FORBIDDEN",
      };
    }
  } else if (tool.moduleId) {
    const mod = getIntranetModuleById(tool.moduleId);
    if (mod && !rolesAllowModule(ctx.roles, mod, ctx.isOrgAdmin)) {
      return {
        ok: false,
        error: "Vous n'avez pas accès à ce module pour cette action.",
        code: "MODULE_FORBIDDEN",
      };
    }
  }

  return { ok: true };
}
