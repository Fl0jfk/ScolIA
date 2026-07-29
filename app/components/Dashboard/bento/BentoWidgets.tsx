"use client";


import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import type { Categories } from "@/app/contexts/data";
import BentoWidget from "@/app/components/Dashboard/bento/BentoWidget";
import DashboardModuleIcon from "@/app/components/Dashboard/bento/DashboardModuleIcon";
import { WeekSheetBentoWidget } from "@/app/components/Dashboard/bento/WeekSheetBentoWidget";
import type { BentoWidgetSize } from "@/app/lib/bento-widget-size";
import BentoWeekGrid from "@/app/components/Dashboard/bento/BentoWeekGrid";
import {
  DASHBOARD_ABSENCES_MAX_SLOTS,
  DashboardScrollList,
  absencesTodayCountLabel,
} from "@/app/components/Dashboard/DashboardScrollList";
import {
  BentoScheduleEntry,
  buildAbsenceTeacherColorMap,
  formatDashboardSlotTime,
  subjectSchedulePresentation,
  teacherSchedulePresentation,
} from "@/app/components/Dashboard/bento/BentoScheduleEntry";
import { DEFAULT_PROF_ROOM_SUBJECT_COLORS } from "@/app/lib/prof-room-defaults";
import BentoDropZone from "@/app/components/Dashboard/bento/BentoDropZone";
import { absencesInWeek, absencesToday, type AbsenceTodayRow, type AbsenceWeekRow } from "@/app/lib/dashboard-absences";
import { getRecentDocs, pushRecentDoc } from "@/app/lib/dashboard-recent-docs";
import { tripsThisWeek, tripsToday, type TripIndexRow } from "@/app/lib/dashboard-trips";
import QRCode from "qrcode";
import ToolboxModal from "@/app/components/toolbox/ToolboxModal";
import { ToolboxFolderIcon } from "@/app/components/toolbox/ToolboxIcons";
import { stageDashboardUpload } from "@/app/lib/dashboard-upload-bridge";
import {
  DASHBOARD_BTN_EMERALD,
  DASHBOARD_BTN_INDIGO,
  DASHBOARD_BTN_SLATE,
  DASHBOARD_SELECT,
  DASHBOARD_TILE_HIGHLIGHT,
  DASHBOARD_TILE_META,
  DASHBOARD_TILE_META_STRONG,
} from "@/app/lib/dashboard-theme";
import { canViewCalendar } from "@/app/lib/absences-types";
import { canViewPersonnelDashboard } from "@/app/lib/personnel-types";
import type { PersonnelDashboardData } from "@/app/lib/personnel-dashboard";
import { useUser } from "@clerk/nextjs";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { formatNomPrenom, schoolWeekDaysParis } from "@/app/lib/dashboard-week";
import { dash } from "@/app/lib/dashboard-brand";

const STATUS_LABEL: Record<string, string> = {
  NOUVELLE: "Nouvelle",
  EN_COURS: "En cours",
  EN_ATTENTE: "En attente",
  TERMINEE: "Terminée",
};

function QuickBtn({
  href,
  className,
  children,
}: {
  href: string;
  className: string;
  children: ReactNode;
}) {
  return (
    <Link href={href} className={`inline-flex rounded-lg px-3 py-1.5 text-[11px] font-bold ${className}`}>
      {children}
    </Link>
  );
}

function EmptyLine({ children }: { children: React.ReactNode }) {
  return <p className={DASHBOARD_TILE_META}>{children}</p>;
}

type WidgetProps = { category: Categories; size: BentoWidgetSize };

function widgetHeader(category: Categories, title?: string) {
  return {
    title: title ?? category.name,
    href: category.link,
    external: category.external,
    iconSrc: category.img,
  };
}

function tripDateLabel(t: TripIndexRow): string {
  const raw = t.data?.date || t.data?.startDate;
  if (!raw) return "";
  const d = new Date(raw);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleDateString("fr-FR", { weekday: "short", day: "numeric", month: "short" });
}

type DocItem = { type: "folder" | "file"; name: string; relPath: string; ext?: string };

function ModuleAccessLink({
  href,
  external,
  description,
}: {
  href: string;
  external?: boolean;
  description?: string;
}) {
  const className =
    "flex w-full flex-col items-center justify-center rounded-xl border border-dashed border-[color:var(--dash-border)] bg-[color:var(--dash-soft-muted)]/40 px-3 py-4 text-center transition hover:border-[color:var(--dash-primary)]/35 hover:bg-[color:var(--dash-soft-muted)]/70 min-h-[4.5rem]";

  const inner = (
    <>
      <span className="text-xs font-bold text-[var(--dash-primary)]">Accéder au module →</span>
      {description ? (
        <span className={`mt-1 line-clamp-2 ${DASHBOARD_TILE_META}`}>{description}</span>
      ) : null}
    </>
  );

  if (external) {
    return (
      <a href={href} target="_blank" rel="noopener noreferrer" className={className}>
        {inner}
      </a>
    );
  }

  return (
    <Link href={href} className={className}>
      {inner}
    </Link>
  );
}

export function DefaultBentoWidget({ category }: WidgetProps) {
  return (
    <BentoWidget {...widgetHeader(category)}>
      <ModuleAccessLink
        href={category.link}
        external={category.external}
        description={category.description}
      />
    </BentoWidget>
  );
}

export function DocumentsBentoWidget({ category, size }: WidgetProps) {
  const [items, setItems] = useState<DocItem[]>([]);
  const [recent, setRecent] = useState(getRecentDocs());
  const [loading, setLoading] = useState(true);
  const [opening, setOpening] = useState<string | null>(null);
  const isLg = size === "lg";

  const refreshBrowse = useCallback(async () => {
    const res = await fetch("/api/documents/browse?scope=personal&path=", { cache: "no-store" });
    const data = await res.json();
    if (res.ok) setItems((data.items ?? []) as DocItem[]);
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await refreshBrowse();
      } catch {
        /* ignore */
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [refreshBrowse]);

  const openFile = async (relPath: string, name: string) => {
    setOpening(relPath);
    try {
      const params = new URLSearchParams({ scope: "personal", path: relPath });
      const res = await fetch(`/api/documents/get-url?${params}`);
      const data = await res.json();
      if (data.url) {
        pushRecentDoc({ relPath, name });
        setRecent(getRecentDocs());
        window.open(data.url, "_blank", "noopener,noreferrer");
      }
    } finally {
      setOpening(null);
    }
  };

  const uploadFiles = async (files: FileList) => {
    const formData = new FormData();
    formData.append("scope", "personal");
    formData.append("path", "");
    for (const file of Array.from(files)) {
      formData.append("file", file);
      formData.append(`relPath:${file.name}`, file.name);
    }
    const res = await fetch("/api/documents/upload", { method: "POST", body: formData });
    if (res.ok) await refreshBrowse();
  };

  const listLimit = isLg ? 5 : 3;
  const recentSlice = recent.slice(0, listLimit);
  const rootFiles = items.filter((i) => i.type === "file").slice(0, listLimit);
  const displayFiles =
    recentSlice.length > 0
      ? recentSlice.map((r) => ({ type: "file" as const, name: r.name, relPath: r.relPath }))
      : rootFiles;

  return (
    <BentoWidget {...widgetHeader(category, "Cloud personnel")}>
      {loading ? (
        <EmptyLine>Chargement…</EmptyLine>
      ) : (
        <>
          {recentSlice.length > 0 ? (
            <p className={`mb-1.5 ${DASHBOARD_TILE_META_STRONG}`}>Récents</p>
          ) : null}
          {displayFiles.length > 0 ? (
            <ul className="space-y-1.5">
              {displayFiles.map((item) => (
                <li key={item.relPath}>
                  <button
                    type="button"
                    disabled={opening === item.relPath}
                    onClick={() => void openFile(item.relPath, item.name)}
                    className="flex w-full items-center gap-2 rounded-lg bg-[color:var(--dash-soft-muted)]/60 px-2.5 py-2 text-left transition hover:bg-[color:var(--dash-soft)]/80 disabled:opacity-60"
                  >
                    <span className="text-sm">📄</span>
                    <span className={`${DASHBOARD_TILE_HIGHLIGHT} truncate`}>{item.name}</span>
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyLine>Aucun fichier récent.</EmptyLine>
          )}

          {isLg && items.some((i) => i.type === "folder") ? (
            <ul className="mt-3 space-y-1 border-t border-[color:var(--dash-border)] pt-2">
              {items
                .filter((i) => i.type === "folder")
                .slice(0, 3)
                .map((folder) => (
                  <li key={folder.relPath}>
                    <Link
                      href={`/documents?path=${encodeURIComponent(folder.relPath)}`}
                      className="flex items-center gap-2 rounded-lg px-2 py-1.5 text-xs font-semibold text-[var(--dash-primary)] hover:bg-[color:var(--dash-soft-muted)]"
                    >
                      <span>📁</span>
                      <span className="truncate">{folder.name}</span>
                    </Link>
                  </li>
                ))}
            </ul>
          ) : null}
        </>
      )}

      {isLg ? (
        <div className="mt-3 border-t border-[color:var(--dash-border)] pt-3">
          <BentoDropZone onFiles={uploadFiles} label="Déposer des fichiers dans le cloud" />
        </div>
      ) : null}
    </BentoWidget>
  );
}

export function RequestsBentoWidget({ category, size }: WidgetProps) {
  const [rows, setRows] = useState<{ id: string; subject: string; status: string; category: string }[]>([]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/requests/list?scope=submitted", { cache: "no-store" });
        if (!res.ok || cancelled) return;
        const data = await res.json();
        if (!cancelled && Array.isArray(data)) setRows(data.slice(0, size === "lg" ? 6 : 3));
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <BentoWidget
      {...widgetHeader(category, "Mes demandes")}
      headerExtra={
        <QuickBtn href="/faire-une-demande" className="bg-[var(--dash-primary)] text-white hover:brightness-110">
          + Faire une demande
        </QuickBtn>
      }
    >
      {rows.length === 0 ? (
        <EmptyLine>Aucune demande récente.</EmptyLine>
      ) : (
        <ul className="space-y-2">
          {rows.map((r) => (
            <li key={r.id}>
              <Link
                href={`/mes-demandes`}
                className="block rounded-xl border border-[color:var(--dash-border)] bg-white px-3 py-2 transition hover:border-[color:var(--dash-primary)]/35 hover:bg-[color:var(--dash-soft-muted)]/40"
              >
                <p className={`${DASHBOARD_TILE_HIGHLIGHT} line-clamp-1`}>{r.subject}</p>
                <p className="mt-0.5 flex gap-2 text-[10px] text-stone-500">
                  <span>{STATUS_LABEL[r.status] ?? r.status}</span>
                  <span>·</span>
                  <span className="truncate">{r.category}</span>
                </p>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </BentoWidget>
  );
}

export function TravelsBentoWidget({ category, size }: WidgetProps) {
  const [allTrips, setAllTrips] = useState<TripIndexRow[]>([]);
  const isLg = size === "lg";

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/travels/list", { cache: "no-store" });
        if (!res.ok || cancelled) return;
        const data = (await res.json()) as TripIndexRow[];
        if (!cancelled) setAllTrips(Array.isArray(data) ? data : []);
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const trips = isLg ? tripsThisWeek(allTrips) : tripsToday(allTrips);
  const limit = isLg ? 8 : 3;

  return (
    <BentoWidget
      {...widgetHeader(category)}
      pulse={trips.length > 0}
      headerExtra={
        <QuickBtn href="/travels?new=1" className={`${DASHBOARD_BTN_INDIGO} !py-1.5`}>
          + Sortie
        </QuickBtn>
      }
    >
      {trips.length === 0 ? (
        <EmptyLine>{isLg ? "Aucune sortie cette semaine." : "Aucune sortie aujourd&apos;hui."}</EmptyLine>
      ) : (
        <ul className="max-h-48 space-y-1.5 overflow-y-auto">
          <li className={DASHBOARD_TILE_META_STRONG}>
            {trips.length} sortie{trips.length > 1 ? "s" : ""} {isLg ? "sur 7 jours" : "aujourd&apos;hui"}
          </li>
          {trips.slice(0, limit).map((t) => (
            <li key={t.id} className={DASHBOARD_TILE_HIGHLIGHT}>
              {isLg && tripDateLabel(t) ? (
                <span className="text-[10px] font-bold uppercase text-[var(--dash-mid)]">{tripDateLabel(t)} · </span>
              ) : null}
              {t.data?.title || "Sans titre"}
            </li>
          ))}
        </ul>
      )}
    </BentoWidget>
  );
}

export function AbsencesBentoWidget({ category, size }: WidgetProps) {
  const { user, isLoaded } = useUser();
  const [todayRows, setTodayRows] = useState<AbsenceTodayRow[]>([]);
  const [weekRows, setWeekRows] = useState<AbsenceWeekRow[]>([]);
  const isLg = size === "lg";

  const roles = useMemo(() => {
    const rolesRaw = user?.publicMetadata?.role;
    return Array.isArray(rolesRaw) ? (rolesRaw as string[]) : rolesRaw ? [String(rolesRaw)] : [];
  }, [user]);

  const showCalendar = isLoaded && canViewCalendar(roles);

  useEffect(() => {
    if (!showCalendar) return;
    let cancelled = false;
    (async () => {
      try {
        const url = isLg
          ? "/api/absences?calendar=true"
          : "/api/absences?calendar=true&today=true";
        const res = await fetch(url, { cache: "no-store" });
        if (!res.ok || cancelled) return;
        const data = await res.json();
        if (cancelled || !Array.isArray(data)) return;
        setTodayRows(absencesToday(data));
        setWeekRows(isLg ? absencesInWeek(data) : []);
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [showCalendar, isLg]);

  const schoolDays = useMemo(() => schoolWeekDaysParis(), []);

  const teacherColorMap = useMemo(() => {
    const names = [...weekRows, ...todayRows].map((r) => r.teacherName);
    return buildAbsenceTeacherColorMap(names);
  }, [weekRows, todayRows]);

  const absenceWeekGrid = useMemo(() => {
    return schoolDays.map((day) => ({
      key: day.key,
      short: day.short,
      items: weekRows
        .filter((r) => r.dayKey === day.key)
        .map((r) => {
          const label = formatNomPrenom(r.teacherName);
          const meta = [r.examType, r.timeLabel].filter(Boolean).join(" · ");
          return (
            <BentoScheduleEntry
              key={r.id}
              primary={label}
              secondary={meta || undefined}
              title={`${label}${r.examType ? ` — ${r.examType}` : ""}${r.timeLabel ? ` (${r.timeLabel})` : ""}`}
              presentation={teacherSchedulePresentation(r.teacherName, teacherColorMap)}
            />
          );
        }),
    }));
  }, [schoolDays, weekRows, teacherColorMap]);

  const todayCount = todayRows.length;
  const weekHasAny = weekRows.length > 0;

  return (
    <BentoWidget
      {...widgetHeader(category)}
      pulse={showCalendar && (todayCount > 0 || weekHasAny)}
      headerExtra={
        <QuickBtn
          href="/rh?tab=absences&view=se-declarer#nouvelle-absence"
          className="bg-sky-600 text-white hover:bg-sky-700"
        >
          Déclarer
        </QuickBtn>
      }
    >
      {!showCalendar ? (
        <EmptyLine>Déclarer et suivre vos absences.</EmptyLine>
      ) : isLg ? (
        !weekHasAny && todayCount === 0 ? (
          <EmptyLine>Personne d&apos;absent cette semaine.</EmptyLine>
        ) : (
          <>
            {todayCount > 0 ? (
              <p className="mb-2 text-[11px] font-bold text-stone-500">
                {absencesTodayCountLabel(todayCount)}
              </p>
            ) : null}
            <BentoWeekGrid days={absenceWeekGrid} maxVisibleSlots={DASHBOARD_ABSENCES_MAX_SLOTS} />
          </>
        )
      ) : todayCount === 0 ? (
        <EmptyLine>Personne d&apos;absent aujourd&apos;hui.</EmptyLine>
      ) : (
        <>
          <p className="mb-2 text-[11px] font-bold text-stone-500">
            {absencesTodayCountLabel(todayCount)}
          </p>
          <DashboardScrollList totalCount={todayCount} slotSize="card">
            <ul className="list-none space-y-1.5">
              {todayRows.map((r) => (
                <li key={r.id}>
                  <BentoScheduleEntry
                    primary={formatNomPrenom(r.teacherName)}
                    secondary={[r.examType, r.timeLabel].filter(Boolean).join(" · ") || undefined}
                    presentation={teacherSchedulePresentation(r.teacherName, teacherColorMap)}
                  />
                </li>
              ))}
            </ul>
          </DashboardScrollList>
        </>
      )}
    </BentoWidget>
  );
}

export function ProfRoomBentoWidget({ category, size }: WidgetProps) {
  const [rooms, setRooms] = useState<{ id: string; name: string }[]>([]);
  const [reservations, setReservations] = useState<
    {
      id: string;
      roomId: string;
      startsAt: string;
      subject: string;
      className: string;
      firstName?: string;
      lastName?: string;
      status?: string;
    }[]
  >([]);
  const [roomId, setRoomId] = useState("");
  const [subjectColors, setSubjectColors] = useState<Record<string, string>>(DEFAULT_PROF_ROOM_SUBJECT_COLORS);
  const isLg = size === "lg";

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [roomsRes, resRes, configRes] = await Promise.all([
          fetch("/api/reservation-rooms/rooms", { cache: "no-store" }),
          fetch("/api/reservation-rooms/reservations", { cache: "no-store" }),
          fetch("/api/reservation-rooms/module-config", { cache: "no-store" }),
        ]);
        if (cancelled) return;
        if (roomsRes.ok) {
          const list = ((await roomsRes.json()).rooms || []) as { id: string; name: string }[];
          setRooms(list);
          if (list[0]) setRoomId(list[0].id);
        }
        if (resRes.ok) {
          setReservations(((await resRes.json()).reservations || []) as typeof reservations);
        }
        if (configRes.ok) {
          const cfg = (await configRes.json()).config as { subjectColors?: Record<string, string> };
          if (cfg?.subjectColors) {
            setSubjectColors({ ...DEFAULT_PROF_ROOM_SUBJECT_COLORS, ...cfg.subjectColors });
          }
        }
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const schoolDays = useMemo(() => schoolWeekDaysParis(), []);

  const filtered = useMemo(() => {
    const keys = isLg ? schoolDays.map((d) => d.key) : [schoolDays.find((d) => d.key === calendarDateKeyParis())?.key ?? schoolDays[0].key];
    return reservations
      .filter(
        (r) =>
          r.status !== "CANCELLED" &&
          r.roomId === roomId &&
          keys.some((k) => r.startsAt.startsWith(k)),
      )
      .sort((a, b) => a.startsAt.localeCompare(b.startsAt));
  }, [reservations, roomId, isLg, schoolDays]);

  const roomWeekGrid = useMemo(() => {
    return schoolDays.map((day) => ({
      key: day.key,
      short: day.short,
      items: filtered
        .filter((r) => r.startsAt.startsWith(day.key))
        .map((r) => {
          const time = formatDashboardSlotTime(r.startsAt);
          const prof =
            r.lastName || r.firstName
              ? formatNomPrenom(`${r.firstName ?? ""} ${r.lastName ?? ""}`.trim())
              : "";
          return (
            <BentoScheduleEntry
              key={r.id}
              time={time}
              primary={r.subject}
              secondary={r.className}
              title={prof ? `${time} — ${r.subject} · ${r.className}\nPar : ${prof}` : `${time} — ${r.subject} · ${r.className}`}
              presentation={subjectSchedulePresentation(
                subjectColors[r.subject] || "bg-slate-600 text-white",
              )}
            />
          );
        }),
    }));
  }, [schoolDays, filtered, subjectColors]);

  const todayKey = calendarDateKeyParis();
  const todayRows = filtered.filter((r) => r.startsAt.startsWith(todayKey));

  return (
    <BentoWidget
      {...widgetHeader(category)}
      headerExtra={
        <QuickBtn href="/prof-room?new=1#form-section" className={`${DASHBOARD_BTN_EMERALD} !py-1.5`}>
          Réserver
        </QuickBtn>
      }
    >
      <div className="flex flex-col">
      <select value={roomId} onChange={(e) => setRoomId(e.target.value)} className={`${DASHBOARD_SELECT} mb-2 shrink-0`}>
        {rooms.map((r) => (
          <option key={r.id} value={r.id}>
            {r.name}
          </option>
        ))}
      </select>
      {isLg ? (
        <BentoWeekGrid days={roomWeekGrid} expand />
      ) : todayRows.length === 0 ? (
        <EmptyLine>Aucune réservation aujourd&apos;hui.</EmptyLine>
      ) : (
        <ul className="list-none space-y-1.5">
          {todayRows.map((r) => (
            <li key={r.id}>
              <BentoScheduleEntry
                time={formatDashboardSlotTime(r.startsAt)}
                primary={r.subject}
                secondary={r.className}
                presentation={subjectSchedulePresentation(
                  subjectColors[r.subject] || "bg-slate-600 text-white",
                )}
              />
            </li>
          ))}
        </ul>
      )}
      </div>
    </BentoWidget>
  );
}

export function InternatBentoWidget({ category, size }: WidgetProps) {
  const [status, setStatus] = useState<"validee" | "en_cours" | "non_demarre">("non_demarre");
  const [activeStudents, setActiveStudents] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/internat/stats", { cache: "no-store" });
        if (!res.ok || cancelled) return;
        const data = await res.json();
        const s = data?.stats?.tonightRollCall?.status;
        if (!cancelled) {
          setStatus(s === "validee" || s === "en_cours" ? s : "non_demarre");
          setActiveStudents(
            typeof data?.stats?.activeStudents === "number" ? data.stats.activeStudents : null,
          );
        }
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const label =
    status === "validee" ? "Appel validé" : status === "en_cours" ? "Appel en cours" : "Appel à faire";

  return (
    <BentoWidget
      {...widgetHeader(category)}
      pulse={status === "en_cours"}
      headerExtra={
        <QuickBtn href="/gestion-internat?tab=appel" className={`${DASHBOARD_BTN_SLATE} !py-1.5`}>
          Appel
        </QuickBtn>
      }
    >
      <p className={DASHBOARD_TILE_META_STRONG}>{label}</p>
      {activeStudents != null ? (
        <p className={`mt-1 ${DASHBOARD_TILE_HIGHLIGHT}`}>
          {activeStudents} interne{activeStudents > 1 ? "s" : ""} actif{activeStudents > 1 ? "s" : ""}
        </p>
      ) : null}
    </BentoWidget>
  );
}

export function RhBentoWidget({ category, size }: WidgetProps) {
  const { user, isLoaded } = useUser();
  const [data, setData] = useState<PersonnelDashboardData | null>(null);

  const roles = useMemo(() => {
    const rolesRaw = user?.publicMetadata?.role;
    return Array.isArray(rolesRaw) ? rolesRaw.map(String) : rolesRaw ? [String(rolesRaw)] : [];
  }, [user?.publicMetadata?.role]);

  const isRh = useMemo(() => canViewPersonnelDashboard(roles), [roles]);

  useEffect(() => {
    if (!isLoaded || !isRh) return;
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/personnel/dashboard", { cache: "no-store" });
        if (!res.ok || cancelled) return;
        if (!cancelled) setData(await res.json());
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [isLoaded, isRh]);

  const total = data
    ? data.counts.onboardings +
      data.counts.signatures +
      data.counts.habilitations +
      data.counts.formations +
      data.counts.medecine +
      data.counts.entretiens
    : 0;

  return (
    <BentoWidget {...widgetHeader(category)} pulse={total > 0}>
      {!isRh ? (
        <EmptyLine>Mon dossier personnel.</EmptyLine>
      ) : total > 0 ? (
        <>
          <p className={DASHBOARD_TILE_META_STRONG}>{total} action{total > 1 ? "s" : ""} RH</p>
          {data!.counts.signatures > 0 ? (
            <p className={`mt-1 ${DASHBOARD_TILE_HIGHLIGHT}`}>
              {data!.counts.signatures} signature{data!.counts.signatures > 1 ? "s" : ""} en attente
            </p>
          ) : null}
        </>
      ) : (
        <EmptyLine>Tout est à jour côté RH.</EmptyLine>
      )}
    </BentoWidget>
  );
}

type PhotoCopieRow = {
  id: string;
  status: "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE";
  etablissement: string;
  nombrePhotocopies: number;
  motif: string;
  createdAt: string;
  createdBy: { name: string };
};

const PHOTO_STATUS: Record<PhotoCopieRow["status"], string> = {
  EN_ATTENTE: "En attente",
  ACCEPTEE: "Acceptée",
  REFUSEE: "Refusée",
};

export function PhotocopiesBentoWidget({ category, size }: WidgetProps) {
  const [rows, setRows] = useState<PhotoCopieRow[]>([]);
  const limit = size === "lg" ? 5 : 3;

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/photocopies-couleur", { cache: "no-store" });
        if (!res.ok || cancelled) return;
        const data = await res.json();
        const items = (Array.isArray(data?.items) ? data.items : []) as PhotoCopieRow[];
        const visible = items
          .filter((i) => i.status === "EN_ATTENTE" || i.status === "ACCEPTEE")
          .sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt));
        if (!cancelled) setRows(visible.slice(0, limit));
      } catch {
        /* ignore */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [limit]);

  const pending = rows.filter((r) => r.status === "EN_ATTENTE").length;
  const accepted = rows.filter((r) => r.status === "ACCEPTEE").length;

  return (
    <BentoWidget
      {...widgetHeader(category)}
      pulse={pending > 0}
      headerExtra={
        <QuickBtn href="/photocopies-couleur" className="bg-indigo-600 text-white hover:bg-indigo-700 !py-1.5">
          + Demande
        </QuickBtn>
      }
    >
      {rows.length === 0 ? (
        <EmptyLine>Aucune demande en attente ni acceptée.</EmptyLine>
      ) : (
        <>
          <p className={`mb-2 ${DASHBOARD_TILE_META}`}>
            {pending > 0 ? `${pending} en attente` : "Aucune en attente"}
            {accepted > 0 ? ` · ${accepted} acceptée${accepted > 1 ? "s" : ""}` : ""}
          </p>
          <ul className="space-y-1">
            {rows.map((r) => (
              <li key={r.id} className={`${DASHBOARD_TILE_HIGHLIGHT} line-clamp-2 text-xs`}>
                <span
                  className={
                    r.status === "EN_ATTENTE"
                      ? "font-bold text-amber-700"
                      : "font-bold text-emerald-700"
                  }
                >
                  {PHOTO_STATUS[r.status]}
                </span>{" "}
                — {r.etablissement} · {r.nombrePhotocopies} ex. · {r.motif}
              </li>
            ))}
          </ul>
        </>
      )}
    </BentoWidget>
  );
}

export function AgentIaBentoWidget({ category }: WidgetProps) {
  const router = useRouter();

  const go = useCallback(
    (files: FileList) => {
      if (!stageDashboardUpload("class", files)) return;
      router.push("/agentIAOCR?from=dashboard");
    },
    [router],
  );

  return (
    <BentoWidget {...widgetHeader(category)}>
      <BentoDropZone
        label="Déposez vos PDF — détection automatique"
        onFiles={go}
      />
    </BentoWidget>
  );
}

export function ToolboxBentoWidget({ category }: WidgetProps) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <article
        className={`flex flex-col overflow-hidden rounded-2xl border bg-white/92 shadow-sm backdrop-blur-sm transition duration-300 hover:shadow-md ${dash.tileBorder} ${dash.tileBorderHover} h-auto min-h-0`}
      >
        <header
          className={`flex shrink-0 items-center justify-between gap-2 rounded-t-2xl border-b px-3 py-2.5 sm:px-4 ${dash.border} ${dash.gradientHeader}`}
        >
          <div className="flex min-w-0 items-center gap-2">
            <DashboardModuleIcon src={category.img} label={category.name} />
            <button
              type="button"
              onClick={() => setOpen(true)}
              className={`truncate text-sm font-black transition sm:text-base text-left ${dash.ink} ${dash.hoverPrimary}`}
            >
              {category.name}
            </button>
          </div>
          <button
            type="button"
            onClick={() => setOpen(true)}
            className={`text-[11px] ${dash.linkBold}`}
          >
            Ouvrir →
          </button>
        </header>
        <button
          type="button"
          onClick={() => setOpen(true)}
          className="overflow-auto p-3 sm:p-4 text-left w-full hover:bg-slate-50/80 transition-colors"
        >
          <div className="flex items-center gap-3">
            <span className="flex h-[4.6rem] w-[4.6rem] shrink-0 items-center justify-center rounded-2xl bg-amber-50 text-amber-700">
              <ToolboxFolderIcon className="w-8 h-8" />
            </span>
            <div className="min-w-0">
              <p className={DASHBOARD_TILE_META_STRONG}>Outils pratiques</p>
              <p className={`${DASHBOARD_TILE_META} mt-1`}>
                {category.description ?? "QR code, rentrée, portes ouvertes, Secret Santa…"}
              </p>
            </div>
          </div>
        </button>
      </article>
      <ToolboxModal open={open} onClose={() => setOpen(false)} />
    </>
  );
}

export function QrBentoWidget({ category }: WidgetProps) {
  const router = useRouter();
  const [url, setUrl] = useState("https://");
  const [qrPreview, setQrPreview] = useState<string | null>(null);

  useEffect(() => {
    if (!url || url.length < 8) {
      setQrPreview(null);
      return;
    }
    let cancelled = false;
    QRCode.toDataURL(url, { width: 128, margin: 1, errorCorrectionLevel: "M" })
      .then((data) => {
        if (!cancelled) setQrPreview(data);
      })
      .catch(() => {
        if (!cancelled) setQrPreview(null);
      });
    return () => {
      cancelled = true;
    };
  }, [url]);

  return (
    <BentoWidget {...widgetHeader(category)}>
      <div className="flex items-stretch gap-2">
        <div className="w-[4.75rem] shrink-0 self-stretch">
          {qrPreview ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={qrPreview}
              alt="Aperçu QR"
              className="h-full min-h-[4.25rem] w-full rounded-lg border border-[color:var(--dash-border)] bg-white p-0.5 object-contain shadow-sm"
            />
          ) : (
            <div className="flex h-full min-h-[4.25rem] items-center justify-center rounded-lg border border-dashed border-[color:var(--dash-border)] bg-[color:var(--dash-soft-muted)]/50 text-[10px] text-stone-400">
              QR
            </div>
          )}
        </div>
        <div className="flex min-w-0 flex-1 flex-col justify-between gap-2">
          <input
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://…"
            className="w-full rounded-lg border border-[color:var(--dash-border)] px-2.5 py-2 text-xs outline-none focus:border-[var(--dash-primary)]"
          />
          <button
            type="button"
            onClick={() => router.push(`/qrcreator?url=${encodeURIComponent(url)}`)}
            className="w-full rounded-lg bg-[var(--dash-primary)] py-2 text-xs font-bold text-white hover:brightness-110"
          >
            Créer le QR
          </button>
        </div>
      </div>
    </BentoWidget>
  );
}

export function renderBentoWidget(category: Categories, size: BentoWidgetSize) {
  const props = { category, size };
  switch (category.variant) {
    case "week-sheet":
      return <WeekSheetBentoWidget {...props} />;
    case "travels":
      return <TravelsBentoWidget {...props} />;
    case "absences":
      return <AbsencesBentoWidget {...props} />;
    case "photocopies-couleur":
      return <PhotocopiesBentoWidget {...props} />;
    case "prof-room":
      return <ProfRoomBentoWidget {...props} />;
    case "internat":
      return <InternatBentoWidget {...props} />;
    case "personnel-ogec":
      return <RhBentoWidget {...props} />;
    case "agent-ia":
      return <AgentIaBentoWidget {...props} />;
    case "toolbox":
      return <ToolboxBentoWidget {...props} />;
    default:
      break;
  }
  switch (category.moduleId) {
    case "documents":
      return <DocumentsBentoWidget {...props} />;
    case "requests-staff":
      return <RequestsBentoWidget {...props} />;
    case "qrcreator":
      return <QrBentoWidget {...props} />;
    default:
      return <DefaultBentoWidget {...props} />;
  }
}
