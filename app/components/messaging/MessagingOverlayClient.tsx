"use client";

import dynamic from "next/dynamic";
import { useMemo } from "react";
import { usePathname } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { isPublicVisitorPath } from "@/app/lib/public-site-paths";

const MessagingOverlay = dynamic(() => import("./MessagingOverlay"), { ssr: false });

export default function MessagingOverlayClient() {
  const pathname = usePathname();
  const { user, isLoaded, isSignedIn } = useSessionUser();

  const skip = useMemo(() => {
    if (!isLoaded || !isSignedIn || !user) return true;
    if (isPublicVisitorPath(pathname)) return true;
    const path = (pathname ?? "").toLowerCase();
    if (path === "/scolia-ai" || path.startsWith("/scolia-ai/")) return true;
    if (path === "/messagerie" || path.startsWith("/messagerie/")) return true;
    if (isEleveBienEtreProfile(rolesFromUserLike(user))) return true;
    const roles = rolesFromUserLike(user);
    if (roles.includes("parent") && roles.length === 1) return true;
    if (roles.includes("eleve") && !roles.some((r) => r !== "eleve")) return true;
    return false;
  }, [isLoaded, isSignedIn, user, pathname]);

  if (skip) return null;
  return <MessagingOverlay />;
}
