"use client";

import { useAppUser } from "@/app/hooks/useAppUser";

export function useIsPlatformMaster(): boolean {
  const { isLoaded, user } = useAppUser();
  if (!isLoaded || !user) return false;
  if (user.platformAdmin) return true;
  return user.roles.includes("master");
}
