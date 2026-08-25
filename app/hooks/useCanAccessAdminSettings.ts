"use client";

import { useAppUser } from "@/app/hooks/useAppUser";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

/** Accès page / API Paramètres : org-admin ou rôle direction. */
export function useCanAccessAdminSettings(): boolean {
  const isOrgAdmin = useIsOrgAdmin();
  const { isLoaded, user } = useAppUser();
  if (!isLoaded || !user) return false;
  if (isOrgAdmin) return true;
  return INTRANET_DIRECTION_SLUGS.some((slug) => user.roles.includes(slug));
}
