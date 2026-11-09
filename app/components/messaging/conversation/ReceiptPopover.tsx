"use client";

import { useEffect, useState } from "react";
import type { MessagingReceiptDetailDto } from "@/app/lib/messaging/types";
import FloatingLayer, { type Anchor } from "./FloatingLayer";
import Avatar from "./Avatar";
import { formatTime } from "./utils";

type Props = {
  messageId: string;
  anchor: Anchor;
  onClose: () => void;
};

export default function ReceiptPopover({ messageId, anchor, onClose }: Props) {
  const [data, setData] = useState<MessagingReceiptDetailDto | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    fetch(`/api/messaging/messages/${messageId}/receipts`, { cache: "no-store" })
      .then(async (res) => {
        if (!res.ok) throw new Error("receipts");
        return (await res.json()) as MessagingReceiptDetailDto;
      })
      .then((d) => {
        if (!cancelled) setData(d);
      })
      .catch(() => {
        if (!cancelled) setError(true);
      });
    return () => {
      cancelled = true;
    };
  }, [messageId]);

  return (
    <FloatingLayer
      anchor={anchor}
      onClose={onClose}
      label="Vu par"
      role="dialog"
      className="w-64 rounded-2xl border border-black/8 bg-white p-2 shadow-[0_12px_32px_-12px_rgba(0,0,0,0.4)]"
    >
      <p className="px-2 pb-1 pt-0.5 text-[11px] font-semibold uppercase tracking-wide text-[var(--dash-mid)]">
        Vu par
      </p>
      {error ? (
        <p className="px-2 py-2 text-xs text-rose-600">Impossible de charger les accusés.</p>
      ) : !data ? (
        <p className="px-2 py-2 text-xs text-neutral-400">Chargement…</p>
      ) : data.recipients.length === 0 ? (
        <p className="px-2 py-2 text-xs text-neutral-400">Aucun destinataire.</p>
      ) : (
        <ul className="max-h-60 overflow-y-auto">
          {data.recipients.map((r) => (
            <li key={r.user.id} className="flex items-center gap-2 rounded-xl px-2 py-1.5">
              <Avatar name={r.user.name} imageUrl={r.user.imageUrl} size={24} />
              <span className="min-w-0 flex-1 truncate text-xs font-medium text-[var(--dash-ink)]">
                {r.user.name}
              </span>
              <span
                className={`shrink-0 text-[11px] ${r.readAt ? "font-semibold text-sky-600" : "text-neutral-500"}`}
              >
                {r.readAt
                  ? `Lu à ${formatTime(r.readAt)}`
                  : r.deliveredAt
                    ? `Distribué à ${formatTime(r.deliveredAt)}`
                    : "Pas encore reçu"}
              </span>
            </li>
          ))}
        </ul>
      )}
    </FloatingLayer>
  );
}
