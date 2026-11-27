"use client";

import { useEffect, useState } from "react";
import Link from "@/app/components/ui/AppLink";
import type { SantaHomeCard, SantaHomeStatus } from "@/app/lib/secret-santa-accueil";

const longDate = (iso: string) =>
  new Date(iso).toLocaleString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    hour: "2-digit",
    minute: "2-digit",
  });

const TEXTS: Record<SantaHomeStatus, { line: (card: SantaHomeCard) => string; action: string; urgent: boolean }> = {
  answer: {
    line: (c) => `${c.organizerName} vous invite. Répondez avant le ${longDate(c.revealAt)}.`,
    action: "Je réponds",
    urgent: true,
  },
  waiting: {
    line: (c) => `Vous participez. Tirage le ${longDate(c.revealAt)}.`,
    action: "Voir le compte à rebours",
    urgent: false,
  },
  reveal: { line: () => "C'est l'heure du tirage !", action: "Découvrir", urgent: true },
  changed: { line: () => "Du changement : rouvrez votre cadeau avant tout achat.", action: "Rouvrir", urgent: true },
  opened: { line: () => "Vous avez ouvert votre cadeau. Gardez le secret 🤫", action: "Voir", urgent: false },
};

export function SantaHomeList({ cards }: { cards: SantaHomeCard[] }) {
  return (
    <section aria-label="Secret Santa" className="flex flex-col gap-3">
      {cards.map((card) => {
        const text = TEXTS[card.status];
        const meeting = [card.eventAt ? longDate(card.eventAt) : "", card.eventPlace].filter(Boolean).join(", ");
        return (
          <Link
            key={card.id}
            href={`/secret-santa/${card.id}`}
            className="flex flex-col gap-3 rounded-[1.75rem] bg-gradient-to-r from-red-800 to-red-600 px-5 py-4 text-white shadow-md transition hover:brightness-110 sm:flex-row sm:items-center"
          >
            <span className="text-4xl" aria-hidden>
              🎅
            </span>
            <span className="min-w-0 flex-1">
              <span className="block text-xs font-black uppercase tracking-[0.2em] text-amber-200">Secret Santa</span>
              <span className="block break-words text-lg font-black">{card.title}</span>
              <span className="block text-sm text-white/90">{text.line(card)}</span>
              {meeting && <span className="block text-sm text-white/75">Rendez-vous : {meeting}</span>}
            </span>
            <span
              className={`shrink-0 self-start rounded-2xl px-4 py-2 text-sm font-black sm:self-center ${
                text.urgent ? "bg-amber-300 text-red-900" : "bg-white/15"
              }`}
            >
              {text.action} →
            </span>
          </Link>
        );
      })}
    </section>
  );
}

/** N'affiche rien pour une personne qui n'est invitée à aucun Secret Santa en cours. */
export default function HomeSecretSanta() {
  const [cards, setCards] = useState<SantaHomeCard[]>([]);

  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        const res = await fetch("/api/secret-santa/accueil", { cache: "no-store" });
        const data = res.ok ? await res.json() : { cards: [] };
        if (alive) setCards(Array.isArray(data.cards) ? data.cards : []);
      } catch {
        /* l'accueil reste utilisable sans l'encadré */
      }
    };
    void load();
    const onVisible = () => document.visibilityState === "visible" && void load();
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      alive = false;
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, []);

  return cards.length ? <SantaHomeList cards={cards} /> : null;
}
