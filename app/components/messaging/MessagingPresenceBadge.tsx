"use client";

import type { ReactNode } from "react";
import type { MessagingPresenceStatus } from "@/app/lib/messaging/types";

const RING: Record<MessagingPresenceStatus, string> = {
  online: "ring-emerald-500",
  away: "ring-amber-400",
  busy: "ring-orange-500",
  dnd: "ring-red-500",
  offline: "ring-slate-300",
};

const DOT: Record<MessagingPresenceStatus, string> = {
  online: "bg-emerald-500",
  away: "bg-amber-400",
  busy: "bg-orange-500",
  dnd: "bg-red-500",
  offline: "bg-slate-400",
};

const LABEL: Record<MessagingPresenceStatus, string> = {
  online: "En ligne",
  away: "Absent",
  busy: "Occupé",
  dnd: "Ne pas déranger",
  offline: "Hors ligne",
};

type Props = {
  status?: MessagingPresenceStatus | null;
  showRing?: boolean;
  dotClassName?: string;
  className?: string;
  children: ReactNode;
};

export default function MessagingPresenceBadge({
  status = "offline",
  showRing = true,
  dotClassName = "h-3.5 w-3.5",
  className = "",
  children,
}: Props) {
  const resolved: MessagingPresenceStatus = status ?? "offline";
  return (
    <span
      className={`relative inline-flex shrink-0 rounded-full ${
        showRing ? `ring-2 ring-offset-1 ring-offset-white ${RING[resolved]}` : ""
      } ${className}`}
      title={LABEL[resolved]}
    >
      {children}
      <span
        aria-hidden
        className={`absolute bottom-0 right-0 rounded-full border-2 border-white ${DOT[resolved]} ${dotClassName}`}
      />
      <span className="sr-only">{LABEL[resolved]}</span>
    </span>
  );
}

export function presenceLabel(status: MessagingPresenceStatus): string {
  return LABEL[status];
}

export { LABEL as PRESENCE_LABELS, DOT as PRESENCE_DOT_COLORS };
