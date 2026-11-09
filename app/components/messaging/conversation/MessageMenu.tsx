"use client";

import { useEffect, useRef, type ReactNode } from "react";
import { REACTION_EMOJIS } from "@/app/lib/messaging/constants";
import {
  IconCopy,
  IconForward,
  IconPencil,
  IconPin,
  IconReply,
  IconTrash,
} from "../MessagingIcons";
import FloatingLayer, { type Anchor } from "./FloatingLayer";

type Props = {
  anchor: Anchor;
  onClose: () => void;
  isMine: boolean;
  canEdit: boolean;
  hasText: boolean;
  pinned: boolean;
  onReact: (emoji: string) => void;
  onReply: () => void;
  onForward: () => void;
  onCopy: () => void;
  onPin: () => void;
  onEdit: () => void;
  onDelete: () => void;
};

function Item({
  icon,
  children,
  onClick,
  danger,
}: {
  icon: ReactNode;
  children: ReactNode;
  onClick: () => void;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onClick}
      className={`flex w-full items-center gap-2.5 rounded-xl px-2.5 py-1.5 text-left text-sm outline-none transition focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 ${
        danger
          ? "text-rose-600 hover:bg-rose-50 focus-visible:bg-rose-50"
          : "text-[var(--dash-ink)] hover:bg-black/[0.05] focus-visible:bg-black/[0.05]"
      }`}
    >
      <span className="shrink-0 opacity-70">{icon}</span>
      {children}
    </button>
  );
}

export default function MessageMenu({
  anchor,
  onClose,
  isMine,
  canEdit,
  hasText,
  pinned,
  onReact,
  onReply,
  onForward,
  onCopy,
  onPin,
  onEdit,
  onDelete,
}: Props) {
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const first = rootRef.current?.querySelector<HTMLElement>("button");
    first?.focus();
  }, []);

  const run = (fn: () => void) => () => {
    onClose();
    fn();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const nodes = Array.from(rootRef.current?.querySelectorAll<HTMLElement>("button") ?? []);
    const idx = nodes.indexOf(document.activeElement as HTMLElement);
    const next = e.key === "ArrowDown" ? (idx + 1) % nodes.length : (idx - 1 + nodes.length) % nodes.length;
    nodes[next]?.focus();
  };

  return (
    <FloatingLayer
      anchor={anchor}
      onClose={onClose}
      label="Actions du message"
      className="w-56 rounded-2xl border border-black/8 bg-white p-1.5 shadow-[0_16px_40px_-12px_rgba(15,23,42,0.45)]"
    >
      <div ref={rootRef} onKeyDown={onKeyDown}>
        <div className="mb-1 flex justify-between gap-0.5 border-b border-black/5 px-1 pb-1.5">
          {REACTION_EMOJIS.map((emoji) => (
            <button
              key={emoji}
              type="button"
              aria-label={`Réagir avec ${emoji}`}
              className="rounded-lg px-1 py-0.5 text-lg outline-none transition hover:scale-125 focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50"
              onClick={run(() => onReact(emoji))}
            >
              {emoji}
            </button>
          ))}
        </div>
        <Item icon={<IconReply className="h-4 w-4" />} onClick={run(onReply)}>
          Répondre
        </Item>
        <Item icon={<IconForward className="h-4 w-4" />} onClick={run(onForward)}>
          Transférer
        </Item>
        {hasText ? (
          <Item icon={<IconCopy className="h-4 w-4" />} onClick={run(onCopy)}>
            Copier le texte
          </Item>
        ) : null}
        <Item icon={<IconPin className="h-4 w-4" />} onClick={run(onPin)}>
          {pinned ? "Désépingler" : "Épingler"}
        </Item>
        {isMine && canEdit ? (
          <Item icon={<IconPencil className="h-4 w-4" />} onClick={run(onEdit)}>
            Modifier
          </Item>
        ) : null}
        {isMine ? (
          <Item icon={<IconTrash className="h-4 w-4" />} onClick={run(onDelete)} danger>
            Supprimer
          </Item>
        ) : null}
      </div>
    </FloatingLayer>
  );
}
