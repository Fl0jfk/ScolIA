"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { WeekSheetEvent } from "@/app/lib/dashboard-week-sheet-types";

type Props = {
  event: WeekSheetEvent;
  top: number;
  height: number;
  left: string;
  width: string;
};

export default function WeekSheetEventBlock({ event, top, height, left, width }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [hover, setHover] = useState(false);
  const [tipPos, setTipPos] = useState<{ x: number; y: number; above: boolean; align: "center" | "start" | "end" }>(
    { x: 0, y: 0, above: false, align: "center" },
  );
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  const updateTipPos = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const tipH = 160;
    const tipWidth = Math.min(320, Math.max(200, window.innerWidth - 32));
    const edgeGap = 8;
    const isTablet = window.innerWidth < 1024;
    const above = rect.bottom + tipH + 12 > window.innerHeight;

    let align: "center" | "start" | "end" = "center";
    let x = rect.left + rect.width / 2;

    if (event.day === "mon" && isTablet) {
      // Tablette : ouvrir vers la droite, sans coller le bord écran.
      align = "start";
      x = Math.min(rect.right + 10, window.innerWidth - tipWidth - edgeGap);
    } else if (event.day === "mon") {
      // Bureau : ouvrir vers la droite depuis le bord gauche du créneau.
      align = "start";
      x = Math.max(edgeGap, rect.left);
    } else {
      const nearLeftEdge = rect.left < tipWidth * 0.5 + edgeGap;
      const nearRightEdge = rect.right > window.innerWidth - tipWidth * 0.5 - edgeGap;
      align = nearLeftEdge ? "start" : nearRightEdge ? "end" : "center";
      x =
        align === "start"
          ? Math.max(edgeGap, rect.left)
          : align === "end"
            ? Math.min(window.innerWidth - edgeGap, rect.right)
            : Math.min(
                Math.max(rect.left + rect.width / 2, tipWidth / 2 + edgeGap),
                window.innerWidth - tipWidth / 2 - edgeGap,
              );
    }

    setTipPos({
      x,
      y: above ? rect.top - 8 : rect.bottom + 8,
      above,
      align,
    });
  }, [event.day]);

  const onEnter = () => {
    updateTipPos();
    setHover(true);
  };

  const onLeave = () => setHover(false);

  useEffect(() => {
    if (!hover) return;
    const onScroll = () => updateTipPos();
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onScroll);
    return () => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onScroll);
    };
  }, [hover, updateTipPos]);

  const tooltip =
    hover && mounted
      ? createPortal(
          <div
            className="pointer-events-none fixed z-[9999] w-[min(20rem,calc(100vw-2rem))] rounded-xl border border-[color:var(--dash-border)] bg-white px-3 py-2.5 text-left shadow-2xl shadow-slate-900/20"
            style={{
              left: tipPos.x,
              top: tipPos.y,
              transform:
                tipPos.align === "start"
                  ? `translate(0, ${tipPos.above ? "calc(-100% - 4px)" : "4px"})`
                  : tipPos.align === "end"
                    ? `translate(-100%, ${tipPos.above ? "calc(-100% - 4px)" : "4px"})`
                    : `translate(-50%, ${tipPos.above ? "calc(-100% - 4px)" : "4px"})`,
            }}
            role="tooltip"
          >
            <p className="text-[11px] font-black text-[var(--dash-primary)]">
              {event.startTime}
              {event.endTime ? ` – ${event.endTime}` : ""}
            </p>
            <p className="mt-1 whitespace-pre-wrap break-words text-[12px] font-semibold leading-snug text-[#14231A]">
              {event.title}
            </p>
            {event.location ? (
              <p className="mt-1 whitespace-pre-wrap break-words text-[11px] text-stone-600">
                {event.location}
              </p>
            ) : null}
          </div>,
          document.body,
        )
      : null;

  const showDetails = height >= 34;
  const showTitle = height >= 22;

  return (
    <>
      <div
        ref={ref}
        className="absolute z-[1] flex flex-col overflow-hidden rounded-md border border-[color:var(--dash-border)]/80 bg-[color:var(--dash-soft-muted)]/95 px-1.5 py-1 shadow-sm hover:z-[50]"
        style={{ top, height, left, width }}
        onMouseEnter={onEnter}
        onMouseLeave={onLeave}
        onFocus={onEnter}
        onBlur={onLeave}
        tabIndex={0}
      >
        <p className="shrink-0 whitespace-pre-wrap break-words text-[10px] font-bold leading-tight text-[#14231A] sm:text-[11px]">
          <span className="text-[var(--dash-mid)]">
            {event.startTime}
            {event.endTime ? ` – ${event.endTime}` : ""}
          </span>
          {showTitle ? <span className="text-[#14231A]"> {event.title}</span> : null}
        </p>
        <div className="min-h-0 flex-1 overflow-hidden leading-[13px] sm:leading-[14px]">
          {!showTitle ? (
            <p className="whitespace-pre-wrap break-words text-[10px] font-semibold text-[#14231A] sm:text-[11px]">
              {event.title}
            </p>
          ) : null}
          {showDetails && event.location ? (
            <p className="mt-0.5 whitespace-pre-wrap break-words text-[9px] text-stone-500 sm:text-[10px]">
              {event.location}
            </p>
          ) : null}
        </div>
      </div>
      {tooltip}
    </>
  );
}
