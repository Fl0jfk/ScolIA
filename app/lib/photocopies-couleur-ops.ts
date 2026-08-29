import type { NotificationsConfig } from "@/app/lib/app-config-schemas";
import type { ModuleAccessConfig, ModuleAccessLookup } from "@/app/lib/module-access";
import { userHasPhotocopiesOpsFlag } from "@/app/lib/module-access";

function normEmail(value: string | undefined | null): string {
  return String(value || "").trim().toLowerCase();
}

function isEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

/**
 * Destinataires impressions (file ops) : liste multi + legacy `photocopiesOps` + env.
 * Source principale recommandée : Droits modules → flag réceptionnaire.
 */
export function resolvePhotocopiesOpsEmails(
  notifications?: Pick<NotificationsConfig, "photocopiesOps" | "photocopiesOpsEmails"> | null,
): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  const push = (raw: string | undefined | null) => {
    const e = String(raw || "").trim();
    if (!e || !isEmail(e)) return;
    const k = normEmail(e);
    if (seen.has(k)) return;
    seen.add(k);
    out.push(e);
  };

  if (Array.isArray(notifications?.photocopiesOpsEmails)) {
    for (const e of notifications.photocopiesOpsEmails) push(e);
  }
  push(notifications?.photocopiesOps);
  push(process.env.PHOTOCOPIES_COULEUR_OPS_EMAIL);

  return out;
}

export function isPhotocopiesOpsHandler(
  email: string | undefined | null,
  opsEmails: string[],
): boolean {
  const mine = normEmail(email);
  if (!mine) return false;
  return opsEmails.some((e) => normEmail(e) === mine);
}

/** E-mails legacy OU flag Droits modules. */
export function isPhotocopiesOpsHandlerResolved(opts: {
  email?: string | null;
  opsEmails: string[];
  moduleAccess?: ModuleAccessConfig | null;
  lookup?: ModuleAccessLookup | null;
}): boolean {
  if (isPhotocopiesOpsHandler(opts.email, opts.opsEmails)) return true;
  return userHasPhotocopiesOpsFlag(opts.moduleAccess, opts.lookup);
}

export function photocopiesOpsPendingCount(
  photocopies: Array<{ status: string }>,
): number {
  return photocopies.filter((p) => p.status === "ACCEPTEE").length;
}
