"use client";

import { useAppUser } from "@/app/hooks/useAppUser";

export function useIsOrgAdmin(): boolean {
  const { user } = useAppUser();
  if (!user) return false;
  if (user.orgAdmin) return true;
  if (user.roles.includes("admin")) return true;
  if (user.roles.includes("master")) return true;
  return false;
}
