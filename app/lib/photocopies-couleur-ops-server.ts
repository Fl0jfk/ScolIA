import "server-only";

import type { ModuleAccessConfig, ModuleAccessLookup } from "@/app/lib/module-access";
import { userHasPhotocopiesOpsFlag } from "@/app/lib/module-access";
import {
  isPhotocopiesOpsHandler,
  resolvePhotocopiesOpsEmails,
} from "@/app/lib/photocopies-couleur-ops";
import type { NotificationsConfig } from "@/app/lib/app-config-schemas";

/** E-mails legacy OU flag Droits modules (server-only). */
export function isPhotocopiesOpsHandlerResolved(opts: {
  email?: string | null;
  opsEmails?: string[];
  notifications?: Pick<NotificationsConfig, "photocopiesOps" | "photocopiesOpsEmails"> | null;
  moduleAccess?: ModuleAccessConfig | null;
  lookup?: ModuleAccessLookup | null;
}): boolean {
  const emails =
    opts.opsEmails ?? resolvePhotocopiesOpsEmails(opts.notifications ?? null);
  if (isPhotocopiesOpsHandler(opts.email, emails)) return true;
  return userHasPhotocopiesOpsFlag(opts.moduleAccess, opts.lookup);
}
