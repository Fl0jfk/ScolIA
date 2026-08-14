"use client";

import dynamic from "next/dynamic";
import { useMemo } from "react";
import { usePathname } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { isPublicVisitorPath } from "@/app/lib/public-site-paths";

const ChatbotBubble = dynamic(() => import("./ChatbotBubble"), { ssr: false });
const ChatbotBubbleBienEtre = dynamic(() => import("./ChatbotBubbleBienEtre"), { ssr: false });

export default function ChatbotBubbleClient() {
  const pathname = usePathname();
  const { user, isLoaded } = useUser();
  const bienEtreMode = useMemo(() => {
    if (!isLoaded || !user) return false;
    return isEleveBienEtreProfile(rolesFromUserLike(user));
  }, [isLoaded, user]);

  if (isPublicVisitorPath(pathname)) return null;

  const path = (pathname ?? "").toLowerCase();
  if (path === "/scolia-ai" || path.startsWith("/scolia-ai/")) return null;

  if (bienEtreMode) return <ChatbotBubbleBienEtre />;
  return <ChatbotBubble />;
}
