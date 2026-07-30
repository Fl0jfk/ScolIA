"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import type { TripReminder } from "@/app/lib/travels-trip-helpers";
import { TripAlert, TripButton } from "@/app/components/travels/TripDetailUI";

export function TripRemindersBanner({
  tripId,
  highlightReminderId,
}: {
  tripId: string;
  highlightReminderId?: string | null;
}) {
  const [reminders, setReminders] = useState<TripReminder[]>([]);
  const [sending, setSending] = useState<string | null>(null);

  useEffect(() => {
    fetch(`/api/travels/reminders?tripId=${encodeURIComponent(tripId)}`)
      .then((r) => r.json())
      .then((d) => setReminders(Array.isArray(d.reminders) ? d.reminders : []))
      .catch(() => setReminders([]));
  }, [tripId]);

  if (reminders.length === 0) return null;

  const top = reminders[0];
  const tone = top.severity === "urgent" ? "warning" : top.severity === "warning" ? "warning" : "info";

  const sendReminder = async (reminderId: string) => {
    setSending(reminderId);
    try {
      const res = await fetch("/api/travels/reminders", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, reminderId }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error);
      if (j.skipped) alert("Ce rappel a déjà été envoyé.");
      else alert("Rappel envoyé au créateur par e-mail.");
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur envoi rappel");
    } finally {
      setSending(null);
    }
  };

  return (
    <div id="trip-reminders" className={highlightReminderId ? "ring-2 ring-amber-400 rounded-2xl" : ""}>
    <TripAlert
      tone={tone}
      icon="⏰"
      title={`${reminders.length} rappel${reminders.length > 1 ? "s" : ""} actif${reminders.length > 1 ? "s" : ""}`}
      action={
        <TripButton
          variant="secondary"
          size="sm"
          disabled={!!sending}
          onClick={() => sendReminder(top.id)}
        >
          {sending ? "…" : "Notifier le créateur"}
        </TripButton>
      }
    >
      <ul className="text-xs space-y-1 mt-1">
        {reminders.slice(0, 3).map((r) => (
          <li
            key={r.id}
            className={
              highlightReminderId === r.id ? "font-bold text-amber-900 bg-amber-100/80 -mx-1 px-1 rounded" : ""
            }
          >
            • {r.label}
            {r.href ? (
              <>
                {" "}
                <Link href={r.href} className="underline font-semibold">
                  Ouvrir
                </Link>
              </>
            ) : null}
          </li>
        ))}
      </ul>
    </TripAlert>
    </div>
  );
}
