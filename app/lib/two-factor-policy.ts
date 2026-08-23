import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

/** Rôles / flags qui obligent l’activation 2FA (TOTP). */
export function roleRequiresTwoFactor(opts: {
  platformAdmin: boolean;
  orgAdmin: boolean;
  roles: string[];
}): boolean {
  if (opts.platformAdmin || opts.orgAdmin) return true;
  if (opts.roles.includes("admin")) return true;
  return INTRANET_DIRECTION_SLUGS.some((slug) => opts.roles.includes(slug));
}
