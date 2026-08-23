"use client";

import { useEffect, useRef } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { clearLastPortalTenant } from "@/app/lib/tenant-portal-client";

/** Oublie le dernier établissement quand la session se termine. */
export default function PortalMemoryOnSignOut() {
  const { isLoaded, isSignedIn } = useSessionUser();
  const wasSignedIn = useRef(false);

  useEffect(() => {
    if (!isLoaded) return;
    if (wasSignedIn.current && !isSignedIn) {
      clearLastPortalTenant();
    }
    wasSignedIn.current = isSignedIn;
  }, [isLoaded, isSignedIn]);

  return null;
}
