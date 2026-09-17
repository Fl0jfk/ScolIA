import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

/** Direction / admin / secrétariat — paramétrage RDV inscriptions. */
export function canManageRdvInscription(roles: string[]): boolean {
  const allowed = new Set<string>([...INTRANET_DIRECTION_SLUGS, "admin", "administratif", "accueil"]);
  return roles.some((r) => allowed.has(r));
}
