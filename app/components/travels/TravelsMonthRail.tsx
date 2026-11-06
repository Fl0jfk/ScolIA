"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from "react";
import {
  buildTravelMonthRail,
  travelCardAnchorId,
  type TravelMonthRailEntry,
} from "@/app/lib/travels-month-rail";
import type { TravelsTrip } from "@/app/lib/travels-types";

/** Attribut posé sur chaque carte de la liste pour suivre le mois affiché. */
export const TRAVEL_RAIL_KEY_ATTR = "data-travel-rail-key";

/** Hauteur (px) sous laquelle une carte est considérée comme « à l'écran » pour le mois actif. */
const ACTIVE_THRESHOLD_PX = 140;

function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true
  );
}

/**
 * Frise des mois des séjours : remplace la barre de défilement sur la page Voyage.
 * - Bureau : colonne à droite de la liste, qui suit le défilement.
 * - Mobile : rangée de mois au-dessus de la liste, qui suit le défilement.
 * Un clic amène au premier séjour du mois ; le mois affiché à l'écran est surligné.
 */
export function useTravelMonthRail(trips: TravelsTrip[]) {
  const entries = useMemo(() => buildTravelMonthRail(trips), [trips]);
  const [activeKey, setActiveKey] = useState<string | null>(null);

  useEffect(() => {
    if (entries.length === 0) return;
    let frame = 0;
    const update = () => {
      frame = 0;
      const cards = document.querySelectorAll<HTMLElement>(`[${TRAVEL_RAIL_KEY_ATTR}]`);
      let key: string | null = null;
      for (const card of cards) {
        if (card.getBoundingClientRect().bottom > ACTIVE_THRESHOLD_PX) {
          key = card.getAttribute(TRAVEL_RAIL_KEY_ATTR);
          break;
        }
      }
      // Tout en bas de la page : dernier mois.
      if (
        window.innerHeight + window.scrollY >= document.documentElement.scrollHeight - 4 &&
        cards.length > 0
      ) {
        key = cards[cards.length - 1]!.getAttribute(TRAVEL_RAIL_KEY_ATTR);
      }
      setActiveKey(key ?? entries[0]!.key);
    };
    const onScroll = () => {
      if (!frame) frame = window.requestAnimationFrame(update);
    };
    update();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);
    return () => {
      if (frame) window.cancelAnimationFrame(frame);
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
    };
  }, [entries]);

  const goTo = useCallback((entry: TravelMonthRailEntry) => {
    const el = document.getElementById(travelCardAnchorId(entry.firstTripId));
    if (!el) return;
    el.scrollIntoView({ behavior: prefersReducedMotion() ? "auto" : "smooth", block: "start" });
    setActiveKey(entry.key);
  }, []);

  return { entries, activeKey, goTo };
}

type RailProps = ReturnType<typeof useTravelMonthRail>;

/**
 * Fait suivre `el` le défilement de la fenêtre, sans sortir de `boundary`.
 * Remplace `position: sticky`, inopérant ici : le conteneur global de l'appli
 * (`ScolaAmbientBackground`, `overflow-x-hidden`) bloque l'effet collant de ses enfants.
 */
function useFollowScroll(
  boundaryRef: RefObject<HTMLElement | null>,
  elRef: RefObject<HTMLElement | null>,
  offsetTop: number,
  enabled: boolean,
) {
  useEffect(() => {
    if (!enabled) return;
    let frame = 0;
    const update = () => {
      frame = 0;
      const boundary = boundaryRef.current;
      const el = elRef.current;
      if (!boundary || !el) return;
      // Élément masqué (ex. version mobile sur grand écran) : rien à faire.
      if (el.offsetParent === null) {
        el.style.transform = "";
        return;
      }
      const boundaryRect = boundary.getBoundingClientRect();
      const maxShift = Math.max(0, boundary.offsetHeight - el.offsetHeight);
      const shift = Math.min(Math.max(offsetTop - boundaryRect.top, 0), maxShift);
      el.style.transform = shift > 0 ? `translateY(${Math.round(shift)}px)` : "";
    };
    const schedule = () => {
      if (!frame) frame = window.requestAnimationFrame(update);
    };
    update();
    window.addEventListener("scroll", schedule, { passive: true });
    window.addEventListener("resize", schedule);
    // La liste change de hauteur (images chargées, filtres) : recalcul.
    const observer =
      typeof ResizeObserver !== "undefined" ? new ResizeObserver(schedule) : null;
    if (observer && boundaryRef.current) observer.observe(boundaryRef.current);
    return () => {
      if (frame) window.cancelAnimationFrame(frame);
      window.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
      observer?.disconnect();
      if (elRef.current) elRef.current.style.transform = "";
    };
  }, [boundaryRef, elRef, offsetTop, enabled]);
}

const todayLabel = () =>
  new Date().toLocaleDateString("fr-FR", { day: "numeric", month: "short" });

/** Colonne verticale (écrans larges). */
export function TravelsMonthRailVertical({ entries, activeKey, goTo }: RailProps) {
  const trackRef = useRef<HTMLDivElement>(null);
  const navRef = useRef<HTMLElement>(null);
  useFollowScroll(trackRef, navRef, 24, entries.length >= 2);

  // Frise plus haute que l'écran : garde le mois actif visible dans la colonne.
  useEffect(() => {
    const nav = navRef.current;
    if (!nav || !activeKey || nav.scrollHeight <= nav.clientHeight) return;
    const item = nav.querySelector<HTMLElement>(`[data-rail-item="${CSS.escape(activeKey)}"]`);
    if (!item) return;
    const top = item.offsetTop - nav.clientHeight / 2 + item.offsetHeight / 2;
    nav.scrollTo({ top, behavior: prefersReducedMotion() ? "auto" : "smooth" });
  }, [activeKey]);

  if (entries.length < 2) return null;
  let lastHeader = "";
  return (
    <div ref={trackRef} className="h-full">
    <nav
      ref={navRef}
      aria-label="Aller aux séjours d'un mois"
      className="relative max-h-[calc(100vh-3rem)] overflow-y-auto rounded-2xl bg-white/90 p-3 shadow-sm ring-1 ring-black/5 will-change-transform"
    >
      <ol className="relative space-y-0.5 before:absolute before:bottom-2 before:left-[9.5px] before:top-2 before:w-px before:bg-slate-200">
        {entries.map((entry, index) => {
          const sectionTitle =
            entry.section === "upcoming" ? `À venir · auj. ${todayLabel()}` : "Terminés";
          const yearTitle = entry.year != null ? String(entry.year) : "";
          const header = `${entry.section}|${yearTitle}`;
          const showSection = !lastHeader.startsWith(`${entry.section}|`);
          const showYear = header !== lastHeader && yearTitle !== "";
          lastHeader = header;
          const active = entry.key === activeKey;
          const past = entry.section === "past";
          return (
            <li key={entry.key} data-rail-item={entry.key}>
              {showSection ? (
                <p className={`relative mb-1 pl-7 text-[10px] font-bold uppercase tracking-[0.14em] text-slate-400 ${index > 0 ? "mt-3" : ""}`}>
                  {sectionTitle}
                </p>
              ) : null}
              {showYear ? (
                <p className="relative mb-0.5 mt-1.5 pl-7 text-[11px] font-black text-slate-500">
                  {yearTitle}
                </p>
              ) : null}
              <button
                type="button"
                onClick={() => goTo(entry)}
                aria-current={active ? "true" : undefined}
                title={`${entry.count} séjour${entry.count > 1 ? "s" : ""} — ${entry.shortLabel}`}
                className={`group relative flex w-full items-center gap-2 rounded-xl py-1.5 pl-1 pr-2 text-left text-sm transition-colors cursor-pointer ${
                  active ? "bg-slate-900 text-white" : "hover:bg-slate-100"
                } ${past && !active ? "text-slate-400" : ""}`}
              >
                <span
                  aria-hidden
                  className={`relative z-[1] h-3 w-3 shrink-0 rounded-full ring-2 ${
                    active
                      ? "bg-[var(--dash-lime,#d4ff37)] ring-white"
                      : entry.isCurrentMonth
                        ? "bg-indigo-500 ring-white"
                        : past
                          ? "bg-slate-300 ring-white"
                          : "bg-white ring-slate-400"
                  }`}
                />
                <span className="ml-1 min-w-0 flex-1 truncate font-semibold">
                  {entry.label}
                  {entry.isCurrentMonth ? (
                    <span className={`ml-1 text-[10px] font-bold ${active ? "text-white/70" : "text-indigo-500"}`}>
                      ce mois-ci
                    </span>
                  ) : null}
                </span>
                <span
                  className={`shrink-0 rounded-full px-1.5 text-[11px] font-bold tabular-nums ${
                    active ? "bg-white/20 text-white" : "bg-slate-100 text-slate-600"
                  }`}
                >
                  {entry.count}
                </span>
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
    </div>
  );
}

/**
 * Rangée horizontale (mobile / tablette).
 * `boundaryRef` : bloc contenant la rangée et la liste ; la rangée le suit sans en sortir.
 */
export function TravelsMonthRailHorizontal({
  entries,
  activeKey,
  goTo,
  boundaryRef,
}: RailProps & { boundaryRef: RefObject<HTMLElement | null> }) {
  const scrollerRef = useRef<HTMLDivElement>(null);
  const navRef = useRef<HTMLElement>(null);
  useFollowScroll(boundaryRef, navRef, 0, entries.length >= 2);

  // Garde le mois actif visible dans la rangée (sans faire défiler la page).
  useEffect(() => {
    const scroller = scrollerRef.current;
    if (!scroller || !activeKey) return;
    const chip = scroller.querySelector<HTMLElement>(`[data-rail-chip="${CSS.escape(activeKey)}"]`);
    if (!chip) return;
    const left = chip.offsetLeft - scroller.clientWidth / 2 + chip.clientWidth / 2;
    scroller.scrollTo({ left, behavior: prefersReducedMotion() ? "auto" : "smooth" });
  }, [activeKey]);

  if (entries.length < 2) return null;
  return (
    <nav
      ref={navRef}
      aria-label="Aller aux séjours d'un mois"
      className="relative z-20 -mx-4 mb-4 border-b border-black/5 bg-[color:var(--dash-surface,#f4f5f3)] px-4 py-2 shadow-sm will-change-transform sm:-mx-6 sm:px-6"
    >
      <div ref={scrollerRef} className="relative flex gap-2 overflow-x-auto">
        {entries.map((entry, i) => {
          const active = entry.key === activeKey;
          const past = entry.section === "past";
          const firstPast = past && entries[i - 1]?.section !== "past";
          return (
            <div key={entry.key} className="flex shrink-0 items-center gap-2">
              {firstPast ? (
                <span className="text-[10px] font-bold uppercase tracking-[0.12em] text-slate-400">
                  Terminés
                </span>
              ) : null}
              <button
                type="button"
                data-rail-chip={entry.key}
                onClick={() => goTo(entry)}
                aria-current={active ? "true" : undefined}
                className={`whitespace-nowrap rounded-full border px-3 py-1.5 text-xs font-bold transition-colors cursor-pointer ${
                  active
                    ? "border-slate-900 bg-slate-900 text-white"
                    : past
                      ? "border-slate-200 bg-white text-slate-400"
                      : entry.isCurrentMonth
                        ? "border-indigo-300 bg-indigo-50 text-indigo-700"
                        : "border-slate-200 bg-white text-slate-700"
                }`}
              >
                {entry.shortLabel} · {entry.count}
              </button>
            </div>
          );
        })}
      </div>
    </nav>
  );
}
