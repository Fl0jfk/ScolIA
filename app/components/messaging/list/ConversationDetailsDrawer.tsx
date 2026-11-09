"use client";

import { useEffect, useState } from "react";
import type {
  MessagingConversationDetailDto,
  MessagingPeer,
  MessagingPresenceStatus,
} from "@/app/lib/messaging/types";
import {
  IconCheck,
  IconPencil,
  IconPlus,
  IconSearch,
  IconVideoCall,
  IconX,
} from "@/app/components/messaging/MessagingIcons";
import { presenceLabel } from "@/app/components/messaging/MessagingPresenceBadge";
import ListAvatar from "./ListAvatar";
import { IconBell, IconBellOff, IconLogOut, IconPin } from "./ListIcons";
import { conversationAvatarSource, conversationTime, peerLetter } from "./listHelpers";

type Props = {
  detail: MessagingConversationDetailDto;
  meId: string;
  users: MessagingPeer[];
  statusOf: (id?: string | null) => MessagingPresenceStatus;
  statusForConversation: (c: MessagingConversationDetailDto) => MessagingPresenceStatus;
  onClose: () => void;
  onToggleMute: () => void;
  onRename: (title: string) => Promise<boolean>;
  onAddMembers: (userIds: string[]) => Promise<boolean>;
  onRemoveMember: (userId: string) => Promise<boolean>;
  onLeave: () => Promise<boolean>;
  onStartVideoCall?: () => void;
};

export default function ConversationDetailsDrawer({
  detail,
  meId,
  users,
  statusOf,
  statusForConversation,
  onClose,
  onToggleMute,
  onRename,
  onAddMembers,
  onRemoveMember,
  onLeave,
  onStartVideoCall,
}: Props) {
  const isGroup = detail.kind === "group";
  const [renaming, setRenaming] = useState(false);
  const [title, setTitle] = useState(detail.title);
  const [adding, setAdding] = useState(false);
  const [addQuery, setAddQuery] = useState("");
  const [toAdd, setToAdd] = useState<string[]>([]);
  const [confirmLeave, setConfirmLeave] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setTitle(detail.title);
  }, [detail.title]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const memberIds = new Set(detail.members.map((m) => m.id));
  const candidates = users
    .filter((u) => !memberIds.has(u.id) && u.id !== meId)
    .filter((u) => {
      const q = addQuery.trim().toLowerCase();
      return !q || u.name.toLowerCase().includes(q) || u.email.toLowerCase().includes(q);
    })
    .slice(0, 40);

  const run = async (fn: () => Promise<boolean>) => {
    setBusy(true);
    try {
      return await fn();
    } finally {
      setBusy(false);
    }
  };

  const av = conversationAvatarSource(detail);
  const section = "text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--dash-mid)]";

  return (
    <div className="fixed inset-0 z-[155] flex justify-end bg-black/30 backdrop-blur-[1px]" onClick={onClose}>
      <aside
        role="dialog"
        aria-label="Détails de la conversation"
        onClick={(e) => e.stopPropagation()}
        className="flex h-full w-full max-w-sm flex-col overflow-hidden bg-white shadow-2xl sm:rounded-l-[1.75rem]"
      >
        <div className="flex items-center justify-between border-b border-black/5 px-4 py-3">
          <h3 className="text-sm font-semibold text-[var(--dash-ink)]">Détails</h3>
          <button
            type="button"
            aria-label="Fermer"
            onClick={onClose}
            className="rounded-full p-1.5 text-neutral-500 hover:bg-[#eceeea]"
          >
            <IconX className="h-4 w-4" />
          </button>
        </div>

        <div className="min-h-0 flex-1 space-y-5 overflow-y-auto p-4">
          <div className="flex flex-col items-center text-center">
            <ListAvatar
              imageUrl={av.imageUrl}
              letter={av.letter}
              name={av.name}
              status={statusForConversation(detail)}
              size="h-20 w-20"
            />
            {renaming ? (
              <form
                className="mt-3 flex w-full items-center gap-2"
                onSubmit={async (e) => {
                  e.preventDefault();
                  const t = title.trim();
                  if (t.length < 2 || t === detail.title) {
                    setRenaming(false);
                    return;
                  }
                  const ok = await run(() => onRename(t));
                  if (ok) setRenaming(false);
                }}
              >
                <input
                  autoFocus
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  maxLength={80}
                  className="min-w-0 flex-1 rounded-2xl border border-black/10 bg-[#eceeea]/50 px-3 py-2 text-sm outline-none focus:border-[var(--dash-ink)]"
                />
                <button
                  type="submit"
                  disabled={busy}
                  aria-label="Valider"
                  className="rounded-full bg-[var(--dash-ink)] p-2 text-white disabled:opacity-50"
                >
                  <IconCheck className="h-4 w-4" />
                </button>
              </form>
            ) : (
              <div className="mt-3 flex items-center gap-1.5">
                <p className="text-base font-semibold text-[var(--dash-ink)]">{detail.title}</p>
                {isGroup && detail.canManage ? (
                  <button
                    type="button"
                    aria-label="Renommer le groupe"
                    onClick={() => setRenaming(true)}
                    className="rounded-full p-1.5 text-neutral-500 hover:bg-[#eceeea]"
                  >
                    <IconPencil className="h-3.5 w-3.5" />
                  </button>
                ) : null}
              </div>
            )}
            <p className="mt-0.5 text-xs text-neutral-500">
              {isGroup
                ? `Groupe · ${detail.members.length} membre${detail.members.length > 1 ? "s" : ""}`
                : detail.peer?.email ?? "Discussion privée"}
            </p>
          </div>

          <div className="flex gap-2">
            <button
              type="button"
              onClick={onToggleMute}
              className="flex flex-1 items-center justify-center gap-1.5 rounded-2xl bg-[#eceeea] py-2.5 text-xs font-semibold text-[var(--dash-ink)] transition hover:bg-[#e2e5e0]"
            >
              {detail.muted ? <IconBell className="h-4 w-4" /> : <IconBellOff className="h-4 w-4" />}
              {detail.muted ? "Réactiver" : "Sourdine"}
            </button>
            {!isGroup && onStartVideoCall ? (
              <button
                type="button"
                onClick={() => {
                  onClose();
                  onStartVideoCall();
                }}
                className="flex flex-1 items-center justify-center gap-1.5 rounded-2xl bg-[var(--dash-ink)] py-2.5 text-xs font-semibold text-white transition hover:opacity-90"
              >
                <IconVideoCall className="h-4 w-4" />
                Visio
              </button>
            ) : null}
          </div>

          {isGroup ? (
            <div>
              <div className="mb-1.5 flex items-center justify-between">
                <p className={section}>Membres ({detail.members.length})</p>
                {detail.canManage ? (
                  <button
                    type="button"
                    onClick={() => setAdding((v) => !v)}
                    className="inline-flex items-center gap-1 rounded-full bg-[#eceeea] px-2.5 py-1 text-[11px] font-semibold text-[var(--dash-ink)]"
                  >
                    <IconPlus className="h-3 w-3" />
                    Ajouter
                  </button>
                ) : null}
              </div>

              {adding && detail.canManage ? (
                <div className="mb-3 rounded-2xl border border-black/8 p-2">
                  <div className="relative">
                    <IconSearch className="pointer-events-none absolute left-2.5 top-2.5 h-3.5 w-3.5 text-neutral-400" />
                    <input
                      value={addQuery}
                      onChange={(e) => setAddQuery(e.target.value)}
                      placeholder="Chercher un collègue…"
                      className="w-full rounded-xl border border-black/6 bg-white py-2 pl-8 pr-2 text-sm outline-none focus:border-[var(--dash-ink)]"
                    />
                  </div>
                  <div className="mt-1.5 max-h-44 overflow-y-auto">
                    {candidates.length === 0 ? (
                      <p className="px-2 py-3 text-xs text-neutral-500">Aucun collègue à ajouter.</p>
                    ) : (
                      candidates.map((u) => {
                        const checked = toAdd.includes(u.id);
                        return (
                          <label
                            key={u.id}
                            className={`flex cursor-pointer items-center gap-2 rounded-xl px-2 py-1.5 ${
                              checked ? "bg-[#eceeea]" : "hover:bg-black/[0.03]"
                            }`}
                          >
                            <input
                              type="checkbox"
                              checked={checked}
                              onChange={() =>
                                setToAdd((prev) =>
                                  checked ? prev.filter((id) => id !== u.id) : [...prev, u.id],
                                )
                              }
                            />
                            <span className="truncate text-sm">{u.name}</span>
                          </label>
                        );
                      })
                    )}
                  </div>
                  <button
                    type="button"
                    disabled={busy || toAdd.length === 0}
                    onClick={async () => {
                      const ok = await run(() => onAddMembers(toAdd));
                      if (ok) {
                        setToAdd([]);
                        setAddQuery("");
                        setAdding(false);
                      }
                    }}
                    className="mt-2 w-full rounded-xl bg-[var(--dash-ink)] py-2 text-xs font-semibold text-white disabled:opacity-40"
                  >
                    {toAdd.length > 0 ? `Ajouter ${toAdd.length} personne${toAdd.length > 1 ? "s" : ""}` : "Ajouter"}
                  </button>
                </div>
              ) : null}

              <ul className="space-y-0.5">
                {detail.members.map((m) => {
                  const status = statusOf(m.id);
                  const isMe = m.id === meId;
                  return (
                    <li key={m.id} className="flex items-center gap-2.5 rounded-2xl px-1.5 py-1.5">
                      <ListAvatar
                        imageUrl={m.imageUrl}
                        letter={peerLetter(m)}
                        name={m.name}
                        status={status}
                        size="h-9 w-9"
                      />
                      <div className="min-w-0 flex-1">
                        <p className="truncate text-sm font-medium text-[var(--dash-ink)]">
                          {m.name}
                          {isMe ? <span className="text-neutral-400"> (vous)</span> : null}
                          {m.id === detail.createdById ? (
                            <span className="ml-1.5 rounded-full bg-[#eceeea] px-1.5 py-0.5 text-[9px] font-semibold text-[var(--dash-mid)]">
                              Admin
                            </span>
                          ) : null}
                        </p>
                        <p className="text-[11px] text-neutral-500">{presenceLabel(status)}</p>
                      </div>
                      {detail.canManage && !isMe ? (
                        <button
                          type="button"
                          disabled={busy}
                          aria-label={`Retirer ${m.name}`}
                          onClick={() => void run(() => onRemoveMember(m.id))}
                          className="rounded-full p-1.5 text-neutral-400 hover:bg-red-50 hover:text-red-600 disabled:opacity-40"
                        >
                          <IconX className="h-4 w-4" />
                        </button>
                      ) : null}
                    </li>
                  );
                })}
              </ul>
            </div>
          ) : null}

          <div>
            <p className={`${section} mb-1.5 flex items-center gap-1`}>
              <IconPin className="h-3 w-3" /> Messages épinglés ({detail.pinnedMessages.length})
            </p>
            {detail.pinnedMessages.length === 0 ? (
              <p className="text-xs text-neutral-500">Aucun message épinglé.</p>
            ) : (
              <ul className="space-y-1.5">
                {detail.pinnedMessages.map((m) => (
                  <li key={m.id} className="rounded-2xl bg-[#eceeea]/70 px-3 py-2">
                    <p className="flex justify-between gap-2 text-[11px] font-semibold text-[var(--dash-ink)]">
                      <span className="truncate">{m.senderName}</span>
                      <span className="shrink-0 font-normal text-neutral-400">
                        {conversationTime(m.createdAt)}
                      </span>
                    </p>
                    <p className="mt-0.5 line-clamp-3 text-xs text-neutral-600">
                      {m.body?.trim() || (m.attachments.length > 0 ? "\u{1F4CE} Pièce jointe" : "—")}
                    </p>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        {isGroup ? (
          <div className="border-t border-black/5 p-3">
            {confirmLeave ? (
              <div>
                <p className="text-xs leading-relaxed text-neutral-600">
                  Quitter « {detail.title} » ? Vous ne recevrez plus ses messages et ne pourrez
                  revenir que si un membre vous réajoute.
                </p>
                <div className="mt-2 flex gap-2">
                  <button
                    type="button"
                    className="flex-1 rounded-xl bg-[#eceeea] py-2 text-xs font-semibold text-[var(--dash-ink)]"
                    onClick={() => setConfirmLeave(false)}
                  >
                    Annuler
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    className="flex-1 rounded-xl bg-red-600 py-2 text-xs font-semibold text-white disabled:opacity-50"
                    onClick={() => void run(onLeave)}
                  >
                    Quitter
                  </button>
                </div>
              </div>
            ) : (
              <button
                type="button"
                onClick={() => setConfirmLeave(true)}
                className="flex w-full items-center justify-center gap-2 rounded-2xl py-2.5 text-xs font-semibold text-red-600 transition hover:bg-red-50"
              >
                <IconLogOut className="h-4 w-4" />
                Quitter le groupe
              </button>
            )}
          </div>
        ) : null}
      </aside>
    </div>
  );
}
