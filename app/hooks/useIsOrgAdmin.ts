"use client";

import { useAppUser } from "@/app/hooks/useAppUser";

export function useIsOrgAdmin(): boolean {
  const { isLoaded, user } = useAppUser();
  if (!isLoaded || !user) return false;
  if (user.orgAdmin) return true;
  if (user.platformAdmin) return true;
  if (user.roles.includes("admin")) return true;
  if (user.roles.includes("master")) return true;
  return false;
}
