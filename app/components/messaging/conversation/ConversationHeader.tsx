"use client";

import type {
  MessagingConversationKind,
  MessagingPeer,
  MessagingPresenceStatus,
} from "@/app/lib/messaging/types";
import { IconInfo, IconMinus, IconSearch, IconUsers, IconVideoCall, IconX } from "../MessagingIcons";
import MessagingPresenceBadge, {
  PRESENCE_DOT_COLORS,
  PRESENCE_LABELS,
} from "../MessagingPresenceBadge";
import Avatar, { avatarTone } from "./Avatar";
import { firstName } from "./utils";

type Props = {
  title: string;
  peer: MessagingPeer | null;
  kind: MessagingConversationKind;
  memberCount: number;
  members: MessagingPeer[];
  presenceStatus: MessagingPresenceStatus;
  typingLabel: string | null;
  searchOpen: boolean;
  onToggleSearch: () => void;
  onStartVideoCall?: () => void;
  onOpenDetails?: () => void;
  onMinimize?: () => void;
  onClose?: () => void;
};

const iconBtn =
  "rounded-full p-2 text-[var(--dash-ink)] outline-none transition hover:bg-black/[0.06] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50";

export default function ConversationHeader({
  title,
  peer,
  kind,
  memberCount,
  members,
  presenceStatus,
  typingLabel,
  searchOpen,
  onToggleSearch,
  onStartVideoCall,
  onOpenDetails,
  onMinimize,
  onClose,
}: Props) {
  const isGroup = kind === "group";
  const name = peer?.name || title || "?";

  const namesPreview = members
    .slice(0, 3)
    .map((m) => firstName(m.name))
    .join(", ");

  const subtitle = typingLabel ? (
    <span className="italic text-[var(--dash-ink)]">{typingLabel}</span>
  ) : isGroup ? (
    <>
      {onOpenDetails ? (
        <button
          type="button"
          onClick={onOpenDetails}
          className="rounded font-medium text-[var(--dash-mid)] underline-offset-2 outline-none hover:text-[var(--dash-ink)] hover:underline focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50"
          aria-label={`${memberCount} membres — voir les détails`}
        >
          {memberCount} membre{memberCount > 1 ? "s" : ""}
        </button>
      ) : (
        <span>
          {memberCount} membre{memberCount > 1 ? "s" : ""}
        </span>
      )}
      {namesPreview ? <span className="text-neutral-500"> · {namesPreview}{members.length > 3 ? "…" : ""}</span> : null}
    </>
  ) : (
    <span className="inline-flex items-center gap-1.5">
      <span aria-hidden className={`h-2 w-2 rounded-full ${PRESENCE_DOT_COLORS[presenceStatus]}`} />
      {PRESENCE_LABELS[presenceStatus]}
    </span>
  );

  return (
    <header className="flex items-center gap-2.5 border-b border-black/5 bg-[#eceeea]/60 px-3.5 py-2.5">
      {isGroup ? (
        <span
          className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full"
          style={avatarTone(title)}
          aria-hidden
        >
          <IconUsers className="h-[18px] w-[18px]" />
        </span>
      ) : (
        <MessagingPresenceBadge status={presenceStatus} showRing={false} dotClassName="h-3 w-3">
          <Avatar name={name} imageUrl={peer?.imageUrl} size={40} />
        </MessagingPresenceBadge>
      )}
      <div className="min-w-0 flex-1">
        <h2 className="truncate text-sm font-semibold tracking-tight text-[var(--dash-ink)]">{title}</h2>
        <p className="truncate text-[11px] text-[var(--dash-mid)]" aria-live="polite">
          {subtitle}
        </p>
      </div>
      <button
        type="button"
        title="Rechercher (Ctrl+F)"
        aria-label="Rechercher dans la conversation"
        aria-pressed={searchOpen}
        className={`${iconBtn} ${searchOpen ? "bg-black/[0.08]" : ""}`}
        onClick={onToggleSearch}
      >
        <IconSearch className="h-4 w-4" />
      </button>
      {onStartVideoCall ? (
        <button
          type="button"
          title="Visio"
          aria-label="Démarrer une visio"
          className={iconBtn}
          onClick={onStartVideoCall}
        >
          <IconVideoCall className="h-4 w-4" />
        </button>
      ) : null}
      {onOpenDetails ? (
        <button
          type="button"
          title="Informations"
          aria-label="Informations sur la conversation"
          className={iconBtn}
          onClick={onOpenDetails}
        >
          <IconInfo className="h-4 w-4" />
        </button>
      ) : null}
      {onMinimize ? (
        <button type="button" aria-label="Réduire" className={`${iconBtn} text-neutral-600`} onClick={onMinimize}>
          <IconMinus className="h-4 w-4" />
        </button>
      ) : null}
      {onClose ? (
        <button type="button" aria-label="Fermer" className={`${iconBtn} text-neutral-600`} onClick={onClose}>
          <IconX className="h-4 w-4" />
        </button>
      ) : null}
    </header>
  );
}
