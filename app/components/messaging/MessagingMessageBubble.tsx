"use client";

import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  MessagingMessageDto,
  MessagingPeer,
  MessagingReceiptDetailDto,
  MessagingReceiptStatus,
} from "@/app/lib/messaging/types";
import {
  IconCheck,
  IconChecks,
  IconMoreHorizontal,
  IconPin,
  IconReply,
} from "./MessagingIcons";
import Avatar from "./conversation/Avatar";
import MessageMenu from "./conversation/MessageMenu";
import ReceiptPopover from "./conversation/ReceiptPopover";
import MessageBody from "./conversation/MessageBody";
import SeenBy from "./conversation/SeenBy";
import { FileCard, ImageThumb } from "./conversation/Attachments";
import type { Anchor } from "./conversation/FloatingLayer";
import { formatTime } from "./conversation/utils";

type Props = {
  message: MessagingMessageDto;
  isMine: boolean;
  currentUserId: string;
  kind: "dm" | "group";
  firstInGroup: boolean;
  /** Dernier de mes messages (affiche le libellé « Vu… » dessous). */
  isLastMine: boolean;
  highlighted: boolean;
  /** Arrivé après l'ouverture de la conversation : petite animation d'apparition. */
  isNew: boolean;
  /** Texte recherché dans la conversation (surlignage). */
  searchQuery: string;
  /** Résultat de recherche courant. */
  activeMatch: boolean;
  membersById: Map<string, MessagingPeer>;
  onReply: (message: MessagingMessageDto) => void;
  onForward: (message: MessagingMessageDto) => void;
  onEdit: (message: MessagingMessageDto) => void;
  /** Demande de suppression (la confirmation est gérée par le panneau). */
  onDelete: (message: MessagingMessageDto) => void;
  onReact: (message: MessagingMessageDto, emoji: string) => void;
  onPin: (message: MessagingMessageDto) => void;
  onCopy: (message: MessagingMessageDto) => void;
  onJumpTo: (messageId: string) => void;
  onOpenImage: (attachmentId: string) => void;
};

const STATUS_LABEL: Record<MessagingReceiptStatus, string> = {
  sent: "Envoyé",
  delivered: "Distribué",
  read: "Lu",
};

const UNAVAILABLE_PREVIEW = "Message non disponible";

function ReceiptTicks({ status }: { status: MessagingReceiptStatus }) {
  const color =
    status === "read"
      ? "text-sky-300"
      : status === "delivered"
        ? "text-white/75"
        : "text-white/60";
  return (
    <span className={`inline-flex items-center ${color}`}>
      {status === "sent" ? (
        <IconCheck className="h-3 w-3" />
      ) : (
        <IconChecks className="h-3.5 w-3.5" />
      )}
      <span className="sr-only">{STATUS_LABEL[status]}</span>
    </span>
  );
}

function reactionTooltip(
  userIds: string[],
  currentUserId: string,
  membersById: Map<string, MessagingPeer>,
): string {
  const names: string[] = [];
  if (userIds.includes(currentUserId)) names.push("Vous");
  let unknown = 0;
  for (const id of userIds) {
    if (id === currentUserId) continue;
    const p = membersById.get(id);
    if (p) names.push(p.name);
    else unknown += 1;
  }
  if (unknown > 0) names.push(`${unknown} autre${unknown > 1 ? "s" : ""}`);
  if (names.length <= 1) return names[0] ?? "";
  return `${names.slice(0, -1).join(", ")} et ${names[names.length - 1]}`;
}

function MessagingMessageBubble({
  message,
  isMine,
  currentUserId,
  kind,
  firstInGroup,
  isLastMine,
  highlighted,
  isNew,
  searchQuery,
  activeMatch,
  membersById,
  onReply,
  onForward,
  onEdit,
  onDelete,
  onReact,
  onPin,
  onCopy,
  onJumpTo,
  onOpenImage,
}: Props) {
  const [menu, setMenu] = useState<Anchor | null>(null);
  const [receipts, setReceipts] = useState<Anchor | null>(null);
  const [tapped, setTapped] = useState(false);
  const [receiptDetail, setReceiptDetail] = useState<MessagingReceiptDetailDto | null>(null);
  const pressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const deleted = Boolean(message.deletedAt);
  const status = message.receiptStatus;
  const mentionedMe = !isMine && message.mentionUserIds.includes(currentUserId);
  const groupReadCount = message.receiptSummary?.readCount ?? 0;

  const mentionNames = useMemo(
    () =>
      message.mentionUserIds
        .map((id) => membersById.get(id)?.name)
        .filter((n): n is string => Boolean(n)),
    [message.mentionUserIds, membersById],
  );

  const reactionGroups = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const r of message.reactions) {
      const list = map.get(r.emoji) ?? [];
      list.push(r.userId);
      map.set(r.emoji, list);
    }
    return [...map.entries()];
  }, [message.reactions]);

  // Détail des accusés, chargé seulement sur le dernier de mes messages lu :
  // « Vu à 14:32 » (DM) et petits avatars « vu par » (groupes).
  const wantReceiptDetail =
    isLastMine &&
    !deleted &&
    ((kind === "dm" && status === "read") || (kind === "group" && groupReadCount > 0));
  useEffect(() => {
    if (!wantReceiptDetail) return;
    let cancelled = false;
    fetch(`/api/messaging/messages/${message.id}/receipts`, { cache: "no-store" })
      .then(async (res) => (res.ok ? ((await res.json()) as MessagingReceiptDetailDto) : null))
      .then((d) => {
        if (!cancelled) setReceiptDetail(d);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [wantReceiptDetail, message.id, status, groupReadCount]);

  const dmReadAt = receiptDetail?.recipients.find((r) => r.readAt)?.readAt ?? null;
  const readers = useMemo(
    () =>
      kind === "group" && wantReceiptDetail
        ? (receiptDetail?.recipients.filter((r) => r.readAt).map((r) => r.user) ?? [])
        : [],
    [kind, wantReceiptDetail, receiptDetail],
  );

  const openMenuAt = useCallback((x: number, y: number, height?: number) => {
    setTapped(false);
    setMenu({ x, y, height });
  }, []);

  const closeMenu = useCallback(() => setMenu(null), []);
  const closeReceipts = useCallback(() => setReceipts(null), []);

  const openReceiptsFrom = (el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    setReceipts({ x: r.left - 180, y: r.top - 8, height: 0 });
  };

  const clearPress = () => {
    if (pressTimer.current) clearTimeout(pressTimer.current);
    pressTimer.current = null;
  };

  const hasText = Boolean(message.body);
  const canEdit = message.type === "text" && !deleted;

  const receiptLabel = (() => {
    if (!isLastMine || deleted || !status) return null;
    if (kind === "group" && message.receiptSummary && message.receiptSummary.recipientCount > 0) {
      const { readCount, recipientCount, deliveredCount } = message.receiptSummary;
      if (readCount > 0) return { text: `Vu par ${readCount}/${recipientCount}`, read: true };
      if (deliveredCount > 0) return { text: "Distribué", read: false };
      return { text: "Envoyé", read: false };
    }
    if (status === "read") {
      return { text: dmReadAt ? `Vu à ${formatTime(dmReadAt)}` : "Vu", read: true };
    }
    return { text: STATUS_LABEL[status], read: false };
  })();

  const showAuthor = kind === "group" && !isMine && firstInGroup;
  const showAvatarCol = kind === "group" && !isMine;

  const subtle = isMine ? "text-white/70" : "text-neutral-500";

  const images = message.attachments.filter((a) => a.mime.startsWith("image/") && a.url);
  const audios = message.attachments.filter((a) => a.mime.startsWith("audio/") && a.url);
  const videos = message.attachments.filter((a) => a.mime.startsWith("video/") && a.url);
  const files = message.attachments.filter(
    (a) =>
      !a.mime.startsWith("image/") && !a.mime.startsWith("audio/") && !a.mime.startsWith("video/"),
  );

  const quoteUnavailable = !message.replyToId || message.replyPreview === UNAVAILABLE_PREVIEW;

  const barBtn =
    "rounded-full p-1 text-[var(--dash-mid)] outline-none hover:bg-black/[0.06] hover:text-[var(--dash-ink)] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50";

  return (
    <div
      id={`msg-${message.id}`}
      className={`flex gap-2 ${isMine ? "flex-row-reverse" : ""} ${
        firstInGroup ? "mt-3" : "mt-0.5"
      } ${isNew ? "scolia-msg-in" : ""}`}
    >
      {showAvatarCol ? (
        <div className="w-7 shrink-0">
          {firstInGroup ? (
            <Avatar name={message.senderName} imageUrl={message.senderImageUrl} size={28} />
          ) : null}
        </div>
      ) : null}

      <div className={`flex min-w-0 max-w-[85%] flex-col ${isMine ? "items-end" : "items-start"}`}>
        {showAuthor ? (
          <p className="mb-0.5 ml-1 max-w-full truncate text-[11px] font-semibold text-[var(--dash-mid)]">
            {message.senderName}
          </p>
        ) : null}

        {message.replyPreview ? (
          <button
            type="button"
            disabled={quoteUnavailable}
            onClick={() => message.replyToId && onJumpTo(message.replyToId)}
            aria-label="Aller au message cité"
            className={`mb-0.5 max-w-full truncate rounded-2xl border-l-2 px-2.5 py-1 text-left text-[11px] text-[var(--dash-mid)] outline-none transition focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/40 enabled:cursor-pointer enabled:hover:bg-black/[0.06] ${
              isMine
                ? "border-[var(--dash-ink)]/40 bg-[var(--dash-ink)]/5"
                : "border-black/15 bg-black/[0.03]"
            } ${quoteUnavailable ? "italic" : ""}`}
          >
            {message.replyPreview}
          </button>
        ) : null}

        <div className="group/bubble relative max-w-full">
          <div
            onContextMenu={(e) => {
              if (deleted) return;
              e.preventDefault();
              openMenuAt(e.clientX, e.clientY);
            }}
            onTouchStart={(e) => {
              if (deleted) return;
              const t = e.touches[0];
              clearPress();
              pressTimer.current = setTimeout(() => openMenuAt(t.clientX, t.clientY), 450);
            }}
            onTouchMove={clearPress}
            onTouchEnd={clearPress}
            onTouchCancel={clearPress}
            onClick={(e) => {
              if ((e.target as HTMLElement).closest("a,button,audio,video")) return;
              setTapped((v) => !v);
            }}
            title={new Date(message.createdAt).toLocaleString("fr-FR", {
              dateStyle: "full",
              timeStyle: "short",
            })}
            className={`relative rounded-[1.25rem] px-3.5 py-2 text-sm shadow-[0_1px_0_rgba(0,0,0,0.03)] transition-shadow ${
              isMine
                ? "rounded-br-md bg-[var(--dash-ink)] text-white"
                : "rounded-bl-md border border-black/6 bg-white text-[var(--dash-ink)]"
            } ${mentionedMe ? "border-l-[3px] border-l-[var(--dash-lime)]" : ""} ${
              highlighted ? "ring-2 ring-[var(--dash-lime)] ring-offset-2" : ""
            } ${activeMatch && !highlighted ? "ring-2 ring-amber-400 ring-offset-2" : ""}`}
          >
            {deleted ? (
              <p className={`italic ${isMine ? "text-white/70" : "text-neutral-500"}`}>
                Message supprimé
              </p>
            ) : (
              <>
                {message.forwardedFromId ? (
                  <p className={`mb-1 text-[10px] uppercase tracking-wide ${subtle}`}>Transféré</p>
                ) : null}

                {images.length > 0 ? (
                  <div className="mb-1.5 flex flex-wrap gap-1.5">
                    {images.map((a) => (
                      <ImageThumb key={a.id} attachment={a} onOpen={onOpenImage} />
                    ))}
                  </div>
                ) : null}

                {audios.map((a) => (
                  <audio key={a.id} controls className="mb-2 max-w-full" src={a.url ?? undefined}>
                    <track kind="captions" />
                  </audio>
                ))}

                {videos.map((a) => (
                  <video
                    key={a.id}
                    controls
                    className="mb-2 max-h-56 max-w-full rounded-xl"
                    src={a.url ?? undefined}
                  >
                    <track kind="captions" />
                  </video>
                ))}

                {files.map((a) => (
                  <FileCard key={a.id} attachment={a} isMine={isMine} />
                ))}

                {message.body ? (
                  <MessageBody
                    body={message.body}
                    mentionNames={mentionNames}
                    isMine={isMine}
                    query={searchQuery}
                  />
                ) : null}
              </>
            )}

            <div className={`mt-0.5 flex items-center justify-end gap-1 text-[10px] ${subtle}`}>
              {message.pinnedAt && !deleted ? (
                <IconPin className="h-2.5 w-2.5" aria-label="Épinglé" />
              ) : null}
              {message.editedAt && !deleted ? <span>(modifié)</span> : null}
              <time dateTime={message.createdAt}>{formatTime(message.createdAt)}</time>
              {isMine && status && !deleted ? (
                <button
                  type="button"
                  aria-label={`${STATUS_LABEL[status]} — voir le détail`}
                  className="rounded p-0.5 outline-none focus-visible:ring-2 focus-visible:ring-white/70"
                  onClick={(e) => {
                    e.stopPropagation();
                    openReceiptsFrom(e.currentTarget);
                  }}
                >
                  <ReceiptTicks status={status} />
                </button>
              ) : null}
            </div>
          </div>

          {!deleted ? (
            <div
              className={`pointer-events-none absolute -top-3.5 right-1 z-10 flex items-center gap-0.5 rounded-full border border-black/8 bg-white p-0.5 opacity-0 shadow-[0_4px_14px_-6px_rgba(0,0,0,0.35)] transition-opacity duration-100 focus-within:pointer-events-auto focus-within:opacity-100 group-hover/bubble:pointer-events-auto group-hover/bubble:opacity-100 ${
                tapped ? "pointer-events-auto opacity-100" : ""
              }`}
            >
              <button
                type="button"
                aria-label="Réagir avec 👍"
                className="hidden rounded-full px-1.5 py-0.5 text-sm outline-none hover:bg-black/[0.06] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 [@media(hover:hover)]:inline-block"
                onClick={() => onReact(message, "👍")}
              >
                👍
              </button>
              <button
                type="button"
                aria-label="Répondre"
                title="Répondre"
                className={`hidden ${barBtn} [@media(hover:hover)]:inline-block`}
                onClick={() => onReply(message)}
              >
                <IconReply className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                aria-label="Plus d’actions"
                title="Plus d’actions"
                aria-haspopup="menu"
                className={barBtn}
                onClick={(e) => {
                  const r = e.currentTarget.getBoundingClientRect();
                  openMenuAt(r.left, r.top, r.height);
                }}
              >
                <IconMoreHorizontal className="h-3.5 w-3.5" />
              </button>
            </div>
          ) : null}
        </div>

        {reactionGroups.length > 0 && !deleted ? (
          <div className={`mt-1 flex flex-wrap gap-1 ${isMine ? "justify-end" : ""}`}>
            {reactionGroups.map(([emoji, userIds]) => {
              const mine = userIds.includes(currentUserId);
              const tip = reactionTooltip(userIds, currentUserId, membersById);
              return (
                <span key={emoji} className="group/pill relative">
                  <button
                    type="button"
                    onClick={() => onReact(message, emoji)}
                    aria-pressed={mine}
                    aria-label={`${emoji} ${userIds.length} — ${tip}`}
                    className={`rounded-full border px-1.5 py-0.5 text-xs outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 ${
                      mine
                        ? "border-[var(--dash-ink)]/50 bg-[var(--dash-lime)]/40 font-semibold"
                        : "border-black/8 bg-white hover:bg-black/[0.04]"
                    }`}
                  >
                    {emoji} {userIds.length}
                  </button>
                  {tip ? (
                    <span
                      role="tooltip"
                      className={`pointer-events-none absolute bottom-full z-20 mb-1 hidden w-max max-w-[14rem] rounded-lg bg-[var(--dash-ink)] px-2 py-1 text-[11px] leading-snug text-white shadow-lg group-hover/pill:block group-focus-within/pill:block ${
                        isMine ? "right-0" : "left-0"
                      }`}
                    >
                      {tip}
                    </span>
                  ) : null}
                </span>
              );
            })}
          </div>
        ) : null}

        {receiptLabel ? (
          <div className="mt-1 flex items-center gap-1.5">
            {readers.length > 0 ? <SeenBy readers={readers} /> : null}
            <button
              type="button"
              className={`rounded px-1 text-[11px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/40 ${
                receiptLabel.read ? "font-semibold text-sky-700" : "text-neutral-500"
              }`}
              onClick={(e) => openReceiptsFrom(e.currentTarget)}
              aria-label={`${receiptLabel.text} — voir le détail`}
            >
              {receiptLabel.text}
            </button>
          </div>
        ) : null}
      </div>

      {menu ? (
        <MessageMenu
          anchor={menu}
          onClose={closeMenu}
          isMine={isMine}
          canEdit={canEdit}
          hasText={hasText}
          pinned={Boolean(message.pinnedAt)}
          onReact={(emoji) => onReact(message, emoji)}
          onReply={() => onReply(message)}
          onForward={() => onForward(message)}
          onCopy={() => onCopy(message)}
          onPin={() => onPin(message)}
          onEdit={() => onEdit(message)}
          onDelete={() => onDelete(message)}
        />
      ) : null}
      {receipts ? (
        <ReceiptPopover messageId={message.id} anchor={receipts} onClose={closeReceipts} />
      ) : null}
    </div>
  );
}

export default memo(MessagingMessageBubble);
