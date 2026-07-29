"use client";

import { useEffect, useState } from "react";
import {
  weekDayColumnClass,
  weekDayLabelClass,
} from "@/app/components/week-sheet/weekDayStyles";
import WeekSheetEventBlock from "@/app/components/week-sheet/WeekSheetEventBlock";
import {
  WEEK_DAYS,
  type WeekDayKey,
  type WeekSheetEvent,
} from "@/app/lib/dashboard-week-sheet-types";
import {
  formatMinutesAsHour,
  getWeekSheetDisplayBounds,
  getWeekSheetDisplayHours,
  getWeekSheetGridPixelHeight,
  layoutDayEvents,
} from "@/app/lib/dashboard-week-sheet-time";
import { todaySchoolWeekDayIndex } from "@/app/lib/dashboard-week";

function eventsForDay(events: WeekSheetEvent[], day: WeekDayKey) {
  return events.filter((ev) => ev.day === day);
}

const PX_PER_HOUR = 40;

const DAY_HEADER_CLASS = "shrink-0 px-1 pt-1.5 pb-1 sm:pt-1";

export default function WeekSheetHourGrid({
  events,
  compact,
}: {
  events: WeekSheetEvent[];
  compact?: boolean;
}) {
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const media = window.matchMedia("(max-width: 639px)");
    const update = () => setIsMobile(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  const pxPerHour = compact ? 34 : PX_PER_HOUR;
  const pxPerMinute = pxPerHour / 60;
  const minEventPx = compact ? 20 : 24;
  const { startMin, endMin } = getWeekSheetDisplayBounds();
  const hours = getWeekSheetDisplayHours();
  const totalHeight = getWeekSheetGridPixelHeight(events, pxPerMinute, minEventPx);
  const gutterWidth = compact ? "2.5rem" : "3rem";
  const todayKey: WeekDayKey | null = (() => {
    const idx = todaySchoolWeekDayIndex();
    const keys: WeekDayKey[] = ["mon", "tue", "wed", "thu", "fri"];
    return idx >= 0 ? keys[idx] : null;
  })();
  const visibleDays = isMobile && todayKey ? WEEK_DAYS.filter((d) => d.key === todayKey) : WEEK_DAYS;

  return (
    <div className="flex w-full min-w-0 gap-1">
      <div
        className={`${weekDayColumnClass(false, "shrink-0 overflow-hidden !p-0")}`}
        style={{ width: gutterWidth }}
      >
        <div className={DAY_HEADER_CLASS} aria-hidden="true">
          <span className="invisible block text-[10px] font-black uppercase sm:text-[9px]">Ven</span>
        </div>
        <div className="relative bg-white/95" style={{ height: totalHeight }}>
          {hours.map((hour) => {
            const top = (hour * 60 - startMin) * pxPerMinute;
            if (top > totalHeight) return null;
            return (
              <div
                key={hour}
                className="pointer-events-none absolute right-0 left-0 flex items-start justify-end border-t border-[color:var(--dash-soft-muted)]/90 pr-1"
                style={{ top, height: pxPerHour }}
              >
                <span className="-mt-2 text-[11px] font-bold tabular-nums text-stone-400 sm:text-xs">
                  {formatMinutesAsHour(hour * 60)}
                </span>
              </div>
            );
          })}
        </div>
      </div>

      <div
        className="grid min-w-0 flex-1 gap-1"
        style={{ gridTemplateColumns: `repeat(${visibleDays.length}, minmax(0, 1fr))` }}
      >
        {visibleDays.map((d) => {
          const isToday = todayKey === d.key;
          const dayLayouts = layoutDayEvents(
            eventsForDay(events, d.key),
            startMin,
            endMin,
            pxPerMinute,
            minEventPx,
          );

          return (
            <div
              key={d.key}
              className={weekDayColumnClass(isToday, "overflow-hidden !p-0")}
            >
              <p className={`${weekDayLabelClass(isToday)} ${DAY_HEADER_CLASS}`}>{d.short}</p>
              <div className="relative bg-white/95" style={{ height: totalHeight }}>
                {hours.map((hour) => {
                  const top = (hour * 60 - startMin) * pxPerMinute;
                  if (top > totalHeight) return null;
                  return (
                    <div
                      key={hour}
                      className="pointer-events-none absolute right-0 left-0 border-t border-[color:var(--dash-soft-muted)]/70"
                      style={{ top }}
                    />
                  );
                })}

                {dayLayouts.map(({ event, top, height, column, columnCount }) => {
                  const widthPct = 100 / columnCount;
                  const leftPct = column * widthPct;
                  return (
                    <WeekSheetEventBlock
                      key={event.id}
                      event={event}
                      top={top}
                      height={height}
                      left={`calc(${leftPct}% + 2px)`}
                      width={`calc(${widthPct}% - 4px)`}
                    />
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
