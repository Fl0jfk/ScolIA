import { loadAppConfig, looksLikeLaProvidenceTenant } from "@/app/lib/app-config";
import { countElevesRegistry } from "@/app/lib/eleves-registry";
import { rolesAllowModule, INTRANET_MODULES } from "@/app/lib/intranet-modules";
import { getTenantSecrets } from "@/app/lib/tenant-registry";
import { getTenant } from "@/app/lib/tenant-context";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

export async function handleOcrModuleStatus(ctx: BrainToolCtx): Promise<BrainToolResult> {
  const mod = INTRANET_MODULES.find((m) => m.id === "agent-ia-ocr");
  const hasAdminRole = mod ? rolesAllowModule(ctx.roles, mod, ctx.isOrgAdmin) : false;

  let studentCount = 0;
  try {
    studentCount = await countElevesRegistry();
  } catch {
    studentCount = 0;
  }
  const hasStudentLists = studentCount > 0;

  let oneDriveConfigured = false;
  try {
    const config = await loadAppConfig();
    const tenant = await getTenant();
    const secrets = await getTenantSecrets(tenant.slug);
    const msClientId =
      secrets?.microsoft?.clientId || process.env.NEXT_PUBLIC_CLIENT_ID?.trim() || "";
    const msTenantId =
      secrets?.microsoft?.tenantId || process.env.NEXT_PUBLIC_TENANT_ID?.trim() || "";
    const flag = config.integrations.microsoftOneDrive?.enabled === true;
    oneDriveConfigured =
      (flag || looksLikeLaProvidenceTenant(config.identity)) && Boolean(msClientId && msTenantId);
  } catch {
    oneDriveConfigured = false;
  }

  const ready = hasAdminRole && hasStudentLists && oneDriveConfigured;
  const ctas: Array<{ label: string; href: string }> = [];
  if (!hasAdminRole) {
    ctas.push({ label: "Accéder au dashboard", href: "/dashboard" });
  } else {
    ctas.push({ label: "Ouvrir Ajout de documents IA", href: "/agentIAOCR" });
  }

  const blockers: string[] = [];
  if (!hasAdminRole) blockers.push("rôle insuffisant (administratif / direction)");
  if (!hasStudentLists) blockers.push("listes élèves absentes ou vides");
  if (!oneDriveConfigured) blockers.push("OneDrive / Microsoft non configuré");

  return {
    ok: true,
    data: {
      hasAdminRole,
      hasStudentLists,
      studentCount,
      oneDriveConfigured,
      /** Connexion MSAL est côté navigateur : on indique seulement si la config est prête. */
      oneDriveConnected: null,
      configured: ready,
      blockers,
      ctas,
      message: ready
        ? "Le module OCR est prêt. Ouvrez /agentIAOCR et connectez OneDrive si besoin."
        : `Préflight OCR incomplet : ${blockers.join(" ; ")}.`,
    },
    summaryFr: ready
      ? "OCR prêt (rôles OK, listes élèves présentes, OneDrive configuré)."
      : `OCR non prêt : ${blockers.join(" ; ")}.`,
  };
}
