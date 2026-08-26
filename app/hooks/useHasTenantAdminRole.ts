"use client";

import { useAppUser } from "@/app/hooks/useAppUser";

/** Rôle `admin` établissement (ou master / platform admin) — pas la direction seule. */
export function useHasTenantAdminRole(): boolean {
  const { isLoaded, user } = useAppUser();
  if (!isLoaded || !user) return false;
  if (user.platformAdmin) return true;
  if (user.roles.includes("admin")) return true;
  if (user.roles.includes("master")) return true;
  return false;
}
