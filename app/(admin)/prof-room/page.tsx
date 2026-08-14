"use client";
import React, { Suspense, useEffect, useState, useMemo } from "react";
import { useSearchParams } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import { useAppContext } from "@/app/hooks/useAppContext";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole } from "@/app/lib/intranet-role-utils";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import ProfRoomGlassCard from "@/app/components/prof-room/ProfRoomGlassCard";
import ProfRoomSettingsTab from "@/app/components/prof-room/ProfRoomSettingsTab";
import { DEFAULT_PROF_ROOM_SUBJECT_COLORS } from "@/app/lib/prof-room-defaults";
import { getSubjectColorPresentation } from "@/app/lib/prof-room-subject-colors";
import {
  isReservationBookedForOther,
  reservationWhoCompact,
  reservationWhoLabel,
} from "@/app/lib/prof-room-reservation-label";
import { dash } from "@/app/lib/dashboard-brand";

const FALLBACK_CLASSES: Record<string, string[]> = {
  "ÉCOLE": ["CP", "CE1", "CE2", "CM1", "CM2"],
  "COLLÈGE": ["6A","6B","6C","6D","6E","6F","5A","5B","5C","5D","5E","5F","4A","4B","4C","4D","4E","4F","3A","3B","3C","3D","3E","3F"],
  "LYCÉE": ["2A","2B","2C","2D","2E","1A","1B","1C","1D","1E","1F","TA","TB","TC","TD","TE","TF"],
  "MAINTENANCE": ["MAINTENANCE"],
};

const HOURS = Array.from({ length: 10 }, (_, i) => 8 + i);
const DAYS = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi"];

const fieldClass =
  "w-full rounded-xl border border-[color:var(--dash-border)] bg-white/80 px-4 py-3 text-sm font-semibold text-[var(--dash-ink)] outline-none shadow-sm transition focus:border-[var(--dash-primary)]";
const selectClass = `${fieldClass} cursor-pointer appearance-none pr-10`;

function FieldLabel({ children }: { children: React.ReactNode }) {
  return <span className={`mb-1.5 block ${dash.fieldLabel}`}>{children}</span>;
}

function SelectShell({ children }: { children: React.ReactNode }) {
  return (
    <span className="relative block">
      {children}
      <span
        className={`pointer-events-none absolute inset-y-0 right-3.5 flex items-center text-[10px] ${dash.textMid}`}
        aria-hidden
      >
        ▾
      </span>
    </span>
  );
}

function ProfRoomPageContent() {
  const searchParams = useSearchParams();
  const { user, isLoaded } = useUser();
  const { data: appCtx } = useAppContext();
  const isOrgAdmin = useIsOrgAdmin();
  const CLASSES_DATA = appCtx?.profRoom?.classesByPole || FALLBACK_CLASSES;
  const SUBJECT_COLORS = { ...DEFAULT_PROF_ROOM_SUBJECT_COLORS, ...(appCtx?.profRoom?.subjectColors || {}) };
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [rooms, setRooms] = useState<any[]>([]);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [reservations, setReservations] = useState<any[]>([]);
  const [currentDate, setCurrentDate] = useState(new Date());
  const [selectedRoom, setSelectedRoom] = useState("");
  const [selectedDate, setSelectedDate] = useState("");
  const [selectedHours, setSelectedHours] = useState<number[]>([]);
  const [subject, setSubject] = useState("");
  const [level, setLevel] = useState("");
  const [className, setClassName] = useState("");
  const [comment, setComment] = useState("");
  const [recurrence, setRecurrence] = useState("none");
  const [untilDate, setUntilDate] = useState("");
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [clipboard, setClipboard] = useState<any>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [contextMenu, setContextMenu] = useState<{ x: number, y: number, res?: any, dateStr?: string, hour?: number } | null>(null);
  const [updateAllSeries, setUpdateAllSeries] = useState(false);
  const [targetFirstName, setTargetFirstName] = useState("");
  const [targetLastName, setTargetLastName] = useState("");
  const [bookForOther, setBookForOther] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [activeTab, setActiveTab] = useState<"reservation" | "settings">("reservation");
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [editingRes, setEditingRes] = useState<any>(null);
  const lastName = (user?.lastName || "").toUpperCase();
  const intranetRoles = intranetRolesFromMetadata(user?.publicMetadata);
  const isGlobalAdmin =
    isOrgAdmin ||
    hasGlobalAdminRole(intranetRoles) ||
    appCtx?.session?.isGlobalAdmin === true;
  const adminClerkUserIds = appCtx?.profRoom?.adminClerkUserIds || [];
  const isModuleListedAdmin = user?.id ? adminClerkUserIds.includes(user.id) : false;
  const canAccessSettings = isGlobalAdmin || isModuleListedAdmin;
  const isAdmin = canAccessSettings;
  const canBookForOthers = isModuleListedAdmin;
  const todayStr = new Date().toISOString().split("T")[0];
  const maxDateLimit = new Date();
  maxDateLimit.setDate(maxDateLimit.getDate() + (appCtx?.profRoom?.bookingHorizonDays ?? 56));
  const maxDateStr = isAdmin ? "" : maxDateLimit.toISOString().split("T")[0];
  const myUpcomingReservations = useMemo(() => {
    return reservations.filter(r => r.userId === user?.id && r.status !== "CANCELLED" && r.startsAt >= new Date().toISOString()).sort((a, b) => a.startsAt.localeCompare(b.startsAt)).slice(0, 5);
  }, [reservations, user?.id]);
  const startOfWeek = useMemo(() => {
    const d = new Date(currentDate);
    const day = d.getDay();
    const diff = d.getDate() - day + (day === 0 ? -6 : 1);
    return new Date(d.setDate(diff));
  }, [currentDate]);
  const weekDays = useMemo(() => {
    return Array.from({ length: 5 }, (_, i) => {
      const d = new Date(startOfWeek);
      d.setDate(d.getDate() + i);
      return d;
    });
  }, [startOfWeek]);
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const media = window.matchMedia("(max-width: 639px)");
    const update = () => setIsMobile(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  const displayDays = useMemo(() => {
    if (!isMobile) {
      return weekDays.map((date, i) => ({ date, label: DAYS[i] }));
    }
    const today = new Date();
    const todayIndex = weekDays.findIndex((date) => date.toDateString() === today.toDateString());
    if (todayIndex >= 0) {
      return [{ date: weekDays[todayIndex], label: DAYS[todayIndex] }];
    }
    return [
      {
        date: today,
        label: today.toLocaleDateString("fr-FR", { weekday: "long" }),
      },
    ];
  }, [isMobile, weekDays]);
  useEffect(() => {
    async function load() {
      try {
        const [roomsRes, resRes] = await Promise.all([
          fetch("/api/reservation-rooms/rooms"),
          fetch("/api/reservation-rooms/reservations")
        ]);
        if (roomsRes.ok) {
          const data = await roomsRes.json();
          setRooms(data.rooms || []);
          if (data.rooms?.length > 0) setSelectedRoom(data.rooms[0].id);
        }
        if (resRes.ok) setReservations((await resRes.json()).reservations || []);
      } catch (error) { console.error(error); }
    }
    load();
    const closeMenu = () => setContextMenu(null);
    window.addEventListener("click", closeMenu);
    return () => window.removeEventListener("click", closeMenu);
  }, []);

  useEffect(() => {
    if (searchParams.get("new") !== "1") return;
    setIsEditing(false);
    setEditingRes(null);
    setSelectedDate(todayStr);
    setSelectedHours([]);
    setSubject("");
    setLevel("");
    setClassName("");
    setComment("");
    setBookForOther(false);
    requestAnimationFrame(() => {
      document.getElementById("form-section")?.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  }, [searchParams, todayStr]);

  const clerkFirstName = user?.firstName || "";
  const clerkLastName = (user?.lastName || "").toUpperCase();
  useEffect(() => {
    if (!clerkFirstName && !clerkLastName) return;
    if (isEditing || bookForOther) return;
    setTargetFirstName(clerkFirstName);
    setTargetLastName(clerkLastName);
  }, [clerkFirstName, clerkLastName, isEditing, bookForOther]);

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const handleCellClick = (dateStr: string, hour: number, resExist?: any) => {
    setUpdateAllSeries(false);
    if (resExist) {
      if (isAdmin || resExist.userId === user?.id) {
        setIsEditing(true);
        setEditingRes(resExist);
        setSelectedDate(resExist.startsAt.split("T")[0]);
        const hourFromRes = parseInt(resExist.startsAt.split("T")[1].split(":")[0]);
        setSelectedHours([hourFromRes]);
        setSubject(resExist.subject);
        setClassName(resExist.className);        
        const foundLevel = Object.keys(CLASSES_DATA).find(l => CLASSES_DATA[l].includes(resExist.className));
        if (foundLevel) setLevel(foundLevel);
        setComment(resExist.comment || "");
        setTargetFirstName(resExist.firstName);
        setTargetLastName(resExist.lastName);
        setBookForOther(canBookForOthers && isReservationBookedForOther(resExist));
        document.getElementById("form-section")?.scrollIntoView({ behavior: "smooth" });
      }
    } else {
      setIsEditing(false);
      setEditingRes(null);
      setSelectedDate(dateStr);
      setSelectedHours([hour]);
      setTargetFirstName(user?.firstName || "");
      setTargetLastName(lastName);
      setBookForOther(false);
      document.getElementById("form-section")?.scrollIntoView({ behavior: "smooth" });
    }
  };
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const handleContextMenu = (e: React.MouseEvent, dateStr: string, hour: number, resExist?: any) => {
    e.preventDefault();
    setContextMenu({ x: e.pageX, y: e.pageY, res: resExist, dateStr, hour });
  };
// eslint-disable-next-line @typescript-eslint/no-explicit-any
  const copyReservation = (res: any) => {
    setClipboard({ subject: res.subject, className: res.className, comment: res.comment });
    setContextMenu(null);
  };
  const pasteReservation = (dateStr: string, hour: number) => {
    if (!clipboard) return;
    setIsEditing(false);
    setEditingRes(null);
    setSelectedDate(dateStr);
    setSelectedHours([hour]);
    setSubject(clipboard.subject);
    setClassName(clipboard.className);
    setComment(clipboard.comment || "");
    setTargetFirstName(user?.firstName || "");
    setTargetLastName(lastName);
    setBookForOther(false);
    setContextMenu(null);
    document.getElementById("form-section")?.scrollIntoView({ behavior: "smooth" });
  };
  async function handleConfirm() {
    if (!selectedRoom || !selectedDate || selectedHours.length === 0 || !subject || !className) {
      alert("Veuillez remplir tous les champs obligatoires.");
      return;
    }
    if (canBookForOthers && bookForOther && (!targetFirstName.trim() || !targetLastName.trim())) {
      alert("Indiquez le prénom et le nom de la personne pour qui vous réservez.");
      return;
    }
    const endpoint = isEditing
      ? "/api/reservation-rooms/reservations/update"
      : "/api/reservation-rooms/reservations/create";
    const userEmail = user?.primaryEmailAddress?.emailAddress || "";
    const body = {
      id: editingRes?.id,
      roomId: selectedRoom,
      selectedHours,
      newHour: selectedHours[0],
      date: selectedDate,
      subject,
      className,
      comment,
      recurrence,
      untilDate,
      updateAllSeries,
      firstName: canBookForOthers && bookForOther ? targetFirstName : user?.firstName,
      lastName: canBookForOthers && bookForOther ? targetLastName.toUpperCase() : lastName,
      email: userEmail,
    };

    console.info("[prof-room] save →", {
      endpoint,
      roomId: selectedRoom,
      date: selectedDate,
      selectedHours,
      email: userEmail || "(vide)",
    });

    try {
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(25_000),
      });
      const rawText = await res.text();
      let j: {
        error?: string;
        mailSent?: boolean;
        mailSkipReason?: string | null;
        count?: number;
        success?: boolean;
      } = {};
      try {
        j = rawText ? JSON.parse(rawText) : {};
      } catch {
        console.error("[prof-room] réponse non-JSON", res.status, rawText.slice(0, 300));
      }

      console.info("[prof-room] save ←", {
        status: res.status,
        ok: res.ok,
        mailSent: j.mailSent,
        mailSkipReason: j.mailSkipReason,
        count: j.count,
        error: j.error,
      });

      if (res.ok) {
        let extra = "";
        if (j.mailSent === true) {
          extra = "\n📧 Mail de confirmation envoyé.";
          console.info("[prof-room] mail OK");
        } else if (endpoint.endsWith("/create")) {
          extra = `\n⚠️ Mail non envoyé : ${j.mailSkipReason || "raison inconnue (voir console)"}`;
          console.warn("[prof-room] mail KO:", j.mailSkipReason || j);
        }
        alert(`✅ Enregistré !${extra}`);
        setIsEditing(false);
        setEditingRes(null);
        try {
          const resRes = await fetch("/api/reservation-rooms/reservations");
          if (resRes.ok) setReservations((await resRes.json()).reservations || []);
        } catch (reloadErr) {
          console.warn("[prof-room] refresh liste échoué", reloadErr);
        }
      } else {
        console.error("[prof-room] save erreur", res.status, j);
        alert(`❌ ${j.error || `Erreur HTTP ${res.status}`}`);
      }
    } catch (err) {
      console.error("[prof-room] save réseau / Load failed", err);
      alert(
        `❌ Échec réseau (souvent timeout mail SMTP ou Clerk).\nDétail : ${
          err instanceof Error ? err.message : String(err)
        }\nRegarde la console (filtre « prof-room »).`,
      );
    }
  }

  async function handleDelete() {
    if (!editingRes) return;
    const reason = prompt("Motif de suppression :", "Annulation");
    if (reason === null) return;
    let deleteAllSeries = false;
    if (editingRes.groupId) {
      deleteAllSeries = confirm("Supprimer TOUTE la série ?");
    }
    const currentUserEmail = user?.primaryEmailAddress?.emailAddress || "";
    const payload = {
      id: editingRes.id,
      groupId: editingRes.groupId,
      deleteAllSeries,
      reason,
      userEmail: currentUserEmail,
      startsAt: editingRes.startsAt,
    };

    console.info("[prof-room] delete →", {
      id: payload.id,
      email: currentUserEmail || "(vide)",
      deleteAllSeries,
    });

    try {
      const res = await fetch("/api/reservation-rooms/reservations/delete", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
        signal: AbortSignal.timeout(25_000),
      });
      const rawText = await res.text();
      let j: {
        error?: string;
        mailSent?: boolean;
        mailSkipReason?: string | null;
        cancelled?: number;
        success?: boolean;
      } = {};
      try {
        j = rawText ? JSON.parse(rawText) : {};
      } catch {
        console.error("[prof-room] delete non-JSON", res.status, rawText.slice(0, 300));
      }

      console.info("[prof-room] delete ←", {
        status: res.status,
        ok: res.ok,
        mailSent: j.mailSent,
        mailSkipReason: j.mailSkipReason,
        cancelled: j.cancelled,
        error: j.error,
      });

      if (res.ok) {
        let extra = "";
        if (j.mailSent === true) {
          extra = "\n📧 Mail d'annulation envoyé.";
        } else {
          extra = `\n⚠️ Mail non envoyé : ${j.mailSkipReason || "raison inconnue"}`;
          console.warn("[prof-room] mail annulation KO:", j.mailSkipReason || j);
        }
        alert(`🗑️ Supprimé !${extra}`);
        setIsEditing(false);
        setEditingRes(null);
        setReservations((prev) =>
          prev.map((r) =>
            r.id === editingRes.id || (deleteAllSeries && editingRes.groupId && r.groupId === editingRes.groupId)
              ? { ...r, status: "CANCELLED" }
              : r,
          ),
        );
        try {
          const resRes = await fetch("/api/reservation-rooms/reservations");
          if (resRes.ok) setReservations((await resRes.json()).reservations || []);
        } catch (reloadErr) {
          console.warn("[prof-room] refresh liste échoué", reloadErr);
        }
      } else {
        console.error("[prof-room] delete erreur", res.status, j);
        alert(`❌ ${j.error || `Erreur HTTP ${res.status}`}`);
      }
    } catch (err) {
      console.error("[prof-room] delete réseau / Load failed", err);
      alert(
        `❌ Échec réseau à la suppression.\nDétail : ${
          err instanceof Error ? err.message : String(err)
        }\nLa suppression a peut‑être quand même eu lieu — recharge et regarde la console « prof-room ».`,
      );
    }
  }
  if (!isLoaded || !user) {
    return (
      <ModulePageShell maxWidthClass="max-w-6xl">
        <p className={`text-sm ${dash.textMid}`}>Initialisation…</p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-6xl" tourModuleId="prof-room">
      <div className="relative space-y-4">
        <div className="pointer-events-none absolute -inset-10 -z-10 overflow-hidden" aria-hidden>
          <div className="absolute -left-24 top-0 h-[22rem] w-[22rem] rounded-full bg-[color:var(--dash-soft)]/80 blur-3xl" />
          <div className="absolute right-0 top-24 h-[18rem] w-[18rem] rounded-full bg-[color:var(--dash-bright)]/20 blur-3xl" />
          <div className="absolute bottom-10 left-1/3 h-[14rem] w-[14rem] rounded-full bg-[color:var(--dash-mid)]/15 blur-3xl" />
        </div>

        {contextMenu ? (
          <div
            className="fixed z-[100] min-w-[180px] overflow-hidden rounded-xl border border-white/70 bg-white/90 p-1 text-xs font-semibold shadow-xl backdrop-blur-xl"
            style={{ top: contextMenu.y, left: contextMenu.x }}
          >
            {contextMenu.res ? (
              <button
                type="button"
                onClick={() => copyReservation(contextMenu.res)}
                className={`flex w-full cursor-pointer items-center gap-2 rounded-lg p-3 text-left ${dash.ink} ${dash.hoverBgSoft}`}
              >
                <span>📋</span> Copier ce créneau
              </button>
            ) : clipboard ? (
              <button
                type="button"
                onClick={() => pasteReservation(contextMenu.dateStr!, contextMenu.hour!)}
                className={`flex w-full cursor-pointer items-center gap-2 rounded-lg p-3 text-left ${dash.ink} ${dash.hoverBgSoft}`}
              >
                <span>📥</span> Coller : {clipboard.subject} ({clipboard.className})
              </button>
            ) : (
              <div className={`p-3 italic ${dash.textMid}`}>Rien à coller...</div>
            )}
          </div>
        ) : null}

        <ModulePageHeader
          eyebrow="Services"
          title="Réservation de salles"
          description="Planning des salles, créneaux et demandes du personnel."
        />

        <ModuleTabNav
          className="mb-2"
          tabs={[
            { id: "reservation", label: "Réservation", dataAttrs: { "data-prof-room-tab": "reservation" } },
            { id: "settings", label: "Paramétrage", hidden: !canAccessSettings },
          ]}
          active={activeTab}
          onChange={setActiveTab}
        />

        {activeTab === "settings" && canAccessSettings ? (
          <ProfRoomSettingsTab />
        ) : (
          <>
            <ProfRoomGlassCard
              data-tour="prof-room-room-select"
              bodyClassName="flex flex-col gap-3 p-4 md:flex-row md:items-center md:justify-between"
            >
              <div className="flex w-full flex-col items-stretch gap-3 sm:flex-row sm:items-center md:w-1/2">
                <SelectShell>
                  <select
                    value={selectedRoom}
                    onChange={(e) => setSelectedRoom(e.target.value)}
                    className={`${selectClass} text-center`}
                  >
                    {rooms.map((r) => (
                      <option key={r.id} value={r.id}>
                        {r.name}
                      </option>
                    ))}
                  </select>
                </SelectShell>
                <div className="flex w-full items-center justify-between rounded-xl border border-[color:var(--dash-border)] bg-white/60">
                  {!isMobile ? (
                    <button
                      type="button"
                      onClick={() => setCurrentDate(new Date(currentDate.setDate(currentDate.getDate() - 7)))}
                      className={`cursor-pointer rounded-lg p-2 py-3 ${dash.textMid} hover:bg-white/80`}
                    >
                      ◀
                    </button>
                  ) : (
                    <span className="w-8" aria-hidden />
                  )}
                  <div className={`px-4 text-center text-[11px] font-semibold uppercase tracking-wide ${dash.ink}`}>
                    {isMobile ? (
                      <>
                        Aujourd&apos;hui
                        <br />
                        <span className={dash.textPrimary}>
                          {new Date().toLocaleDateString("fr-FR", { weekday: "long", day: "numeric", month: "short" })}
                        </span>
                      </>
                    ) : (
                      <>
                        Semaine du <br />
                        <span className={dash.textPrimary}>
                          {startOfWeek.toLocaleDateString("fr-FR", { day: "numeric", month: "short" })}
                        </span>
                      </>
                    )}
                  </div>
                  {!isMobile ? (
                    <button
                      type="button"
                      onClick={() => setCurrentDate(new Date(currentDate.setDate(currentDate.getDate() + 7)))}
                      className={`cursor-pointer rounded-lg p-2 ${dash.textMid} hover:bg-white/80`}
                    >
                      ▶
                    </button>
                  ) : (
                    <span className="w-8" aria-hidden />
                  )}
                </div>
              </div>
              <div className="flex w-full items-center justify-between gap-3 md:w-1/2 md:justify-end">
                <label className="flex h-11 min-w-0 flex-1 cursor-pointer items-center gap-2 overflow-hidden rounded-full border border-[color:var(--dash-border)] bg-white/80 px-3 shadow-sm">
                  <span className="flex-shrink-0 text-sm" aria-hidden>
                    📅
                  </span>
                  <input
                    type="date"
                    onChange={(e) => setCurrentDate(new Date(e.target.value))}
                    className={`min-w-0 w-full flex-1 bg-transparent text-[15px] font-semibold outline-none ${dash.ink}`}
                  />
                </label>
                {isAdmin ? (
                  <span className={`shrink-0 whitespace-nowrap rounded-full px-3 py-1 text-[11px] font-semibold tracking-wide text-white ${dash.bgPrimary}`}>
                    Mode admin
                  </span>
                ) : null}
              </div>
            </ProfRoomGlassCard>

            <ProfRoomGlassCard data-tour="prof-room-calendar" className="overflow-hidden">
              <div
                className="pointer-events-none absolute -right-10 -top-12 z-0 h-44 w-44 rounded-full bg-[color:var(--dash-soft)]/80 blur-3xl"
                aria-hidden
              />
              <div className={`relative z-[1] grid border-b border-[color:var(--dash-border)] ${dash.bgSoft50} ${isMobile ? "grid-cols-2" : "grid-cols-6"}`}>
                <div className={`p-4 text-center text-[11px] font-semibold uppercase tracking-wide ${dash.textMid}`}>
                  Heure
                </div>
                {displayDays.map((day, i) => (
                  <div
                    key={`${day.label}-${i}`}
                    className={`border-l border-[color:var(--dash-border)] p-4 text-center ${
                      day.date.toDateString() === new Date().toDateString() ? dash.bgSoft : ""
                    }`}
                  >
                    <p className={`text-[10px] font-semibold uppercase tracking-wide ${dash.textMid}`}>{day.label}</p>
                    <p className={`text-xl font-semibold tracking-tight ${dash.ink}`}>{day.date.getDate()}</p>
                  </div>
                ))}
              </div>
              <div className="relative z-[1] divide-y divide-[color:var(--dash-border)]">
                {HOURS.map((h) => (
                  <div key={h} className={`grid min-h-[95px] ${isMobile ? "grid-cols-2" : "grid-cols-6"}`}>
                    <div className={`flex items-center justify-center text-[12px] font-semibold italic ${dash.bgSoft50} ${dash.textMid}`}>
                      {h}h30
                    </div>
                    {displayDays.map((day, i) => {
                      const date = day.date;
                      const dateStr = date.toISOString().split("T")[0];
                      const hourPrefix = `${dateStr}T${h.toString().padStart(2, "0")}`;
                      const res = reservations.find(
                        (r) =>
                          r.roomId === selectedRoom &&
                          r.startsAt.startsWith(hourPrefix) &&
                          r.status !== "CANCELLED",
                      );
                      const isOwn = res?.userId === user.id;
                      const canModify = isAdmin || isOwn;
                      const colorValue = res ? SUBJECT_COLORS[res.subject] || "bg-slate-600 text-white" : "";
                      const colorPresentation = res ? getSubjectColorPresentation(colorValue) : null;
                      return (
                        <div
                          key={i}
                          onClick={() => handleCellClick(dateStr, h, res)}
                          onContextMenu={(e) => handleContextMenu(e, dateStr, h, res)}
                          className={`group relative cursor-pointer border-l border-[color:var(--dash-border)] p-1 transition-all sm:h-[120px] ${
                            !res ? "hover:bg-[color:var(--dash-soft)]/45" : ""
                          }`}
                        >
                          {res ? (
                            <>
                              <div
                                className={`flex h-full w-full flex-col justify-between rounded-xl p-2 text-[11px] ${colorPresentation?.className || ""} ${
                                  isOwn ? "ring-2 ring-[color:var(--dash-bright)]/70 ring-inset" : ""
                                }`}
                                style={colorPresentation?.style}
                              >
                                <div>
                                  <div className="flex items-start justify-between sm:flex-col">
                                    <p className="truncate font-semibold uppercase leading-none">{res.subject}</p>
                                    <span className="rounded bg-white/20 px-1 text-[11px] font-semibold">{res.className}</span>
                                  </div>
                                  {res.comment ? (
                                    <p className="mt-1 whitespace-normal break-words border-t border-white/10 pt-1 italic leading-tight opacity-90 sm:line-clamp-3">
                                      &apos;{res.comment}&apos;
                                    </p>
                                  ) : null}
                                </div>
                                <div className="mt-1 flex items-end justify-between">
                                  <span className="line-clamp-2 font-semibold uppercase leading-tight opacity-80">
                                    {reservationWhoCompact(res)}
                                  </span>
                                  {canModify ? <span className="text-[10px] sm:hidden">✎</span> : null}
                                </div>
                              </div>
                              <div
                                className={`pointer-events-none absolute left-1/2 z-[100] w-72 -translate-x-1/2 rounded-xl border border-white/70 bg-white/95 p-4 opacity-0 shadow-xl backdrop-blur-xl transition-all group-hover:opacity-100 ${
                                  h <= 10 ? "top-full mt-2" : "bottom-full mb-2"
                                }`}
                              >
                                <p className={`mb-1 break-words text-[15px] font-semibold uppercase leading-tight ${dash.textPrimary}`}>
                                  {res.subject} - {res.className}
                                </p>
                                <p className={`mb-3 text-sm font-semibold ${dash.ink}`}>
                                  Par : {reservationWhoLabel(res)}
                                </p>
                                {res.comment ? (
                                  <div className={`rounded-lg border p-3 ${dash.borderSoft} ${dash.bgSoft50}`}>
                                    <p className={`whitespace-normal break-words text-sm italic leading-relaxed ${dash.textMid}`}>
                                      &apos;{res.comment}&apos;
                                    </p>
                                  </div>
                                ) : null}
                                <div
                                  className={`absolute left-1/2 h-3 w-3 -translate-x-1/2 rotate-45 border-white/70 bg-white/95 ${
                                    h <= 10 ? "-top-1.5 border-l border-t" : "-bottom-1.5 border-b border-r"
                                  }`}
                                />
                              </div>
                            </>
                          ) : (
                            <div className="flex h-full w-full items-center justify-center rounded-lg opacity-0 transition-opacity group-hover:bg-[color:var(--dash-soft)]/60 group-hover:opacity-100">
                              <span className={`text-[10px] font-semibold ${dash.textPrimary}`}>+ Libre</span>
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                ))}
              </div>
            </ProfRoomGlassCard>

            {myUpcomingReservations.length > 0 ? (
              <ProfRoomGlassCard data-tour="prof-room-upcoming" bodyClassName="p-5 sm:p-6">
                <h3 className={`mb-4 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.18em] ${dash.textMid}`}>
                  📅 Mes 5 prochaines réservations
                </h3>
                <div className="grid grid-cols-1 gap-2 md:grid-cols-5">
                  {myUpcomingReservations.map((res) => (
                    <button
                      type="button"
                      key={res.id}
                      onClick={() => {
                        const dStr = res.startsAt.split("T")[0];
                        const hNum = parseInt(res.startsAt.split("T")[1].split(":")[0]);
                        handleCellClick(dStr, hNum, res);
                      }}
                      className={`cursor-pointer rounded-xl border bg-white/70 p-3 text-left shadow-sm transition hover:-translate-y-0.5 hover:bg-white ${dash.borderSoft} ${dash.hoverBorder}`}
                    >
                      <p className={`text-[10px] font-semibold uppercase tracking-wide ${dash.textMid}`}>
                        {new Date(res.startsAt).toLocaleDateString("fr-FR", {
                          weekday: "short",
                          day: "numeric",
                          month: "short",
                        })}
                      </p>
                      <p className={`text-xs font-semibold ${dash.textPrimary}`}>
                        {res.startsAt.split("T")[1].substring(0, 5).replace(":", "h")}
                      </p>
                      <div className={`mt-2 text-[10px] font-semibold ${dash.ink}`}>
                        <span className="block truncate">📍 {rooms.find((r) => r.id === res.roomId)?.name || "Salle"}</span>
                        <span className={`block ${dash.textMid}`}>
                          📚 {res.subject} ({res.className})
                        </span>
                        {isReservationBookedForOther(res) ? (
                          <span className="mt-1 block truncate opacity-80">
                            {reservationWhoCompact(res)}
                          </span>
                        ) : null}
                      </div>
                    </button>
                  ))}
                </div>
              </ProfRoomGlassCard>
            ) : null}

            <ProfRoomGlassCard id="form-section" data-tour="prof-room-form" className="overflow-hidden" bodyClassName="p-5 sm:p-6 md:p-7">
              <div
                className="pointer-events-none absolute -right-10 -top-12 z-0 h-36 w-36 rounded-full bg-[color:var(--dash-soft)]/70 blur-3xl"
                aria-hidden
              />
              <div className="relative z-[1]">
              <div className="mb-6 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                <div className="flex min-w-0 items-center gap-3">
                  <div
                    className={`shrink-0 rounded-full px-3 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-white ${
                      isEditing ? "bg-amber-500" : dash.bgPrimary
                    }`}
                  >
                    {isEditing ? "Modifier" : "Réserver"}
                  </div>
                  <h2 className={`text-xl font-semibold tracking-tight md:text-2xl ${dash.ink}`}>
                    {isEditing ? "Détails du créneau" : "Nouvelle demande"}
                  </h2>
                </div>
                {isEditing ? (
                  <ModuleButton variant="danger" onClick={handleDelete} className="w-full md:w-auto">
                    🗑️ Supprimer ce créneau
                  </ModuleButton>
                ) : null}
              </div>

              <div className={`mb-5 flex flex-wrap items-center gap-3 rounded-2xl border bg-white/70 px-4 py-3 ${dash.borderSoft}`}>
                <span className={`text-[10px] font-semibold uppercase tracking-[0.18em] ${dash.textMid}`}>
                  Identité
                </span>
                <span className={`text-sm font-semibold ${dash.textPrimary}`}>
                  {user.firstName} {lastName}
                </span>
                {canBookForOthers ? (
                  <label className={`ml-auto flex cursor-pointer items-center gap-2 text-sm font-semibold ${dash.ink}`}>
                    <input
                      type="checkbox"
                      checked={bookForOther}
                      onChange={(e) => setBookForOther(e.target.checked)}
                      className="rounded border-slate-300"
                    />
                    Pour une autre personne
                  </label>
                ) : null}
              </div>
              {canBookForOthers && bookForOther ? (
                <div className="mb-5 grid grid-cols-1 gap-3 sm:grid-cols-2">
                  <label className="block min-w-0">
                    <FieldLabel>Prénom</FieldLabel>
                    <input
                      type="text"
                      placeholder="Prénom"
                      value={targetFirstName}
                      onChange={(e) => setTargetFirstName(e.target.value)}
                      className={fieldClass}
                    />
                  </label>
                  <label className="block min-w-0">
                    <FieldLabel>Nom</FieldLabel>
                    <input
                      type="text"
                      placeholder="NOM"
                      value={targetLastName}
                      onChange={(e) => setTargetLastName(e.target.value.toUpperCase())}
                      className={fieldClass}
                    />
                  </label>
                </div>
              ) : null}

              <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                <label className="block min-w-0">
                  <FieldLabel>Matière</FieldLabel>
                  <SelectShell>
                    <select value={subject} onChange={(e) => setSubject(e.target.value)} className={selectClass}>
                      <option value="">Choisir une matière</option>
                      {Object.keys(SUBJECT_COLORS).map((s) => (
                        <option key={s} value={s}>
                          {s}
                        </option>
                      ))}
                    </select>
                  </SelectShell>
                </label>
                <label className="block min-w-0">
                  <FieldLabel>Date</FieldLabel>
                  <input
                    type="date"
                    value={selectedDate}
                    min={todayStr}
                    max={maxDateStr}
                    onChange={(e) => setSelectedDate(e.target.value)}
                    className={`${fieldClass} block text-[16px]`}
                  />
                </label>
                <label className="block min-w-0">
                  <FieldLabel>Niveau</FieldLabel>
                  <SelectShell>
                    <select value={level} onChange={(e) => setLevel(e.target.value)} className={selectClass}>
                      <option value="">Choisir un niveau</option>
                      {Object.keys(CLASSES_DATA).map((l) => (
                        <option key={l} value={l}>
                          {l}
                        </option>
                      ))}
                    </select>
                  </SelectShell>
                </label>
                <label className="block min-w-0">
                  <FieldLabel>Classe</FieldLabel>
                  <SelectShell>
                    <select
                      value={className}
                      onChange={(e) => setClassName(e.target.value)}
                      className={selectClass}
                    >
                      <option value="">Choisir une classe</option>
                      {level && CLASSES_DATA[level].map((c) => (
                        <option key={c} value={c}>
                          {c}
                        </option>
                      ))}
                    </select>
                  </SelectShell>
                </label>
                <div className="md:col-span-2">
                  <FieldLabel>Heure</FieldLabel>
                  <div className="grid grid-cols-5 gap-2">
                    {HOURS.map((h) => {
                      const hourPrefix = `${selectedDate}T${h.toString().padStart(2, "0")}`;
                      const isTaken = reservations.some(
                        (r) =>
                          r.roomId === selectedRoom &&
                          r.startsAt.startsWith(hourPrefix) &&
                          r.status !== "CANCELLED" &&
                          r.id !== editingRes?.id,
                      );
                      return (
                        <button
                          key={h}
                          type="button"
                          disabled={isTaken}
                          onClick={() => setSelectedHours([h])}
                          className={`rounded-xl px-2 py-2.5 text-xs font-semibold transition ${
                            selectedHours.includes(h)
                              ? `${dash.bgPrimary} text-white shadow-sm`
                              : isTaken
                                ? "cursor-not-allowed border border-rose-200 bg-rose-50 text-rose-400"
                                : `cursor-pointer border bg-white/80 ${dash.borderSoft} ${dash.ink} ${dash.hoverBgSoft}`
                          }`}
                        >
                          {h}h30
                        </button>
                      );
                    })}
                  </div>
                </div>
                <label className="block min-w-0 md:col-span-2">
                  <FieldLabel>Commentaire</FieldLabel>
                  <textarea
                    placeholder="Ex. Valise PC"
                    value={comment}
                    onChange={(e) => setComment(e.target.value)}
                    className={`${fieldClass} h-20 resize-none`}
                  />
                </label>
                <label className="block min-w-0">
                  <FieldLabel>Répétition</FieldLabel>
                  <SelectShell>
                    <select value={recurrence} onChange={(e) => setRecurrence(e.target.value)} className={selectClass}>
                      <option value="none">Une seule fois</option>
                      <option value="weekly">Toutes les semaines</option>
                      <option value="biweekly">Toutes les 2 semaines</option>
                    </select>
                  </SelectShell>
                </label>
                {recurrence !== "none" ? (
                  <label className="block min-w-0">
                    <FieldLabel>Jusqu&apos;au</FieldLabel>
                    <input
                      type="date"
                      value={untilDate}
                      min={selectedDate}
                      max={maxDateStr}
                      onChange={(e) => setUntilDate(e.target.value)}
                      className={`${fieldClass} border-amber-200 bg-amber-50/70 text-amber-900`}
                    />
                  </label>
                ) : (
                  <div className="hidden md:block" />
                )}
              </div>
              {isEditing && editingRes?.groupId ? (
                <div className={`mt-6 flex items-center gap-3 rounded-2xl border p-4 ${dash.borderSoft} ${dash.bgSoft}`}>
                  <input
                    type="checkbox"
                    id="updateSeries"
                    checked={updateAllSeries}
                    onChange={(e) => setUpdateAllSeries(e.target.checked)}
                    className="h-5 w-5 rounded border-[color:var(--dash-border)] text-[var(--dash-primary)] focus:ring-[color:var(--dash-bright)]/30"
                  />
                  <label htmlFor="updateSeries" className={`cursor-pointer text-sm font-semibold ${dash.ink}`}>
                    🔄 Appliquer les modifications à TOUTE la série de réservations
                  </label>
                </div>
              ) : null}
              <div className="mt-8 flex gap-4 sm:max-md:flex-col">
                <ModuleButton onClick={handleConfirm} className="flex-1 py-4 text-base">
                  {isEditing ? "Enregistrer les modifications" : "Confirmer la réservation"}
                </ModuleButton>
                <ModuleButton
                  variant="secondary"
                  onClick={() => {
                    setIsEditing(false);
                    setEditingRes(null);
                    setSubject("");
                    setClassName("");
                    setComment("");
                    setLevel("");
                  }}
                  className="px-8 sm:py-4"
                >
                  Annuler
                </ModuleButton>
              </div>
              </div>
            </ProfRoomGlassCard>
          </>
        )}
      </div>
    </ModulePageShell>
  );
}

export default function ProfRoomPage() {
  return (
    <Suspense
      fallback={
        <ModulePageShell maxWidthClass="max-w-6xl">
          <p className={`text-sm ${dash.textMid}`}>Chargement des salles…</p>
        </ModulePageShell>
      }
    >
      <ProfRoomPageContent />
    </Suspense>
  );
}