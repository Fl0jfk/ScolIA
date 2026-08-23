"use client";

import dynamic from "next/dynamic";
import { useEffect, useState } from "react";
import { usePathname } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { isPublicVisitorPath } from "@/app/lib/public-site-paths";
import { canUseTeamsChatOverlay } from "@/app/lib/teams-chat/roles";
import type { TeamsChatStatus } from "@/app/lib/teams-chat/types";

/** UI masquée jusqu’à activation explicite (voir app/lib/teams-chat/flag.ts). */
const UI_ENABLED =
  process.env.NEXT_PUBLIC_TEAMS_CHAT_OVERLAY === "1" ||
  process.env.NEXT_PUBLIC_TEAMS_CHAT_OVERLAY === "true";

const TeamsChatOverlay = dynamic(() => import("./TeamsChatOverlay"), { ssr: false });

export default function TeamsChatOverlayClient() {
  const pathname = usePathname();
  const { user, isLoaded, isSignedIn } = useSessionUser();
  const [status, setStatus] = useState<TeamsChatStatus | null>(null);

  const skip =
    !UI_ENABLED ||
    !isLoaded ||
    !isSignedIn ||
    isPublicVisitorPath(pathname) ||
    (pathname ?? "").toLowerCase().startsWith("/scolia-ai") ||
    (user ? isEleveBienEtreProfile(rolesFromUserLike(user)) : false) ||
    (user ? !canUseTeamsChatOverlay(rolesFromUserLike(user)) : true);

  useEffect(() => {
    if (skip) {
      setStatus(null);
      return;
    }
    let cancelled = false;
    fetch("/api/teams-chat/status")
      .then((r) => r.json() as Promise<TeamsChatStatus>)
      .then((data) => {
        if (!cancelled) setStatus(data);
      })
      .catch(() => {
        if (!cancelled) setStatus({ enabled: false, allowed: false, linked: false });
      });
    return () => {
      cancelled = true;
    };
  }, [skip]);

  if (skip || !status?.enabled || !status.allowed) return null;
  return <TeamsChatOverlay initialStatus={status} />;
}
