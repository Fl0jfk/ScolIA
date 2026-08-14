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
  "w-full rounded-xl border-2 border-slate-300 bg-white px-4 py-3 text-sm font-semibold text-[var(--dash-ink)] outline-none shadow-sm transition focus:border-[var(--dash-primary)]";

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
    requestAnimationFrame(() => {
      document.getElementById("form-section")?.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  }, [searchParams, todayStr]);
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
        document.getElementById("form-section")?.scrollIntoView({ behavior: "smooth" });
      }
    } else {
      setIsEditing(false);
      setEditingRes(null);
      setSelectedDate(dateStr);
      setSelectedHours([hour]);
      setTargetFirstName(user?.firstName || "");
      setTargetLastName(lastName);
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
    setContextMenu(null);
    document.getElementById("form-section")?.scrollIntoView({ behavior: "smooth" });
  };
  async function handleConfirm() {
    if (!selectedRoom || !selectedDate || selectedHours.length === 0 || !subject || !className) {
      alert("Veuillez remplir tous les champs obligatoires.");
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
      firstName: isAdmin ? targetFirstName : user?.firstName,
      lastName: isAdmin ? targetLastName.toUpperCase() : lastName,
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
              className="border-2 !border-slate-300"
              bodyClassName="flex flex-col gap-3 p-4 md:flex-row md:items-center md:justify-between"
            >
              <div className="flex w-full flex-col items-stretch gap-3 sm:flex-row sm:items-center md:w-1/2">
                <select
                  value={selectedRoom}
                  onChange={(e) => setSelectedRoom(e.target.value)}
                  className={`${fieldClass} cursor-pointer text-center`}
                >
                  {rooms.map((r) => (
                    <option key={r.id} value={r.id}>
                      {r.name}
                    </option>
                  ))}
                </select>
                <div className="flex w-full items-center justify-between rounded-xl border-2 border-slate-300 bg-white">
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
                <label className="flex min-w-0 flex-1 cursor-pointer items-center gap-2 rounded-full border-2 border-slate-300 bg-white px-3 py-1.5 shadow-sm">
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

            <ProfRoomGlassCard data-tour="prof-room-calendar" className="border-2 !border-slate-300 bg-white">
              <div className={`grid border-b-2 border-slate-300 bg-slate-100 ${isMobile ? "grid-cols-2" : "grid-cols-6"}`}>
                <div className={`p-4 text-center text-[11px] font-semibold uppercase tracking-wide ${dash.textMid}`}>
                  Heure
                </div>
                {displayDays.map((day, i) => (
                  <div
                    key={`${day.label}-${i}`}
                    className={`border-l-2 border-slate-300 p-4 text-center ${
                      day.date.toDateString() === new Date().toDateString() ? "bg-[color:var(--dash-soft)]" : "bg-white"
                    }`}
                  >
                    <p className={`text-[10px] font-semibold uppercase tracking-wide ${dash.textMid}`}>{day.label}</p>
                    <p className={`text-xl font-semibold tracking-tight ${dash.ink}`}>{day.date.getDate()}</p>
                  </div>
                ))}
              </div>
              <div className="divide-y-2 divide-slate-300">
                {HOURS.map((h) => (
                  <div key={h} className={`grid min-h-[95px] ${isMobile ? "grid-cols-2" : "grid-cols-6"}`}>
                    <div className={`flex items-center justify-center border-r-2 border-slate-300 bg-slate-100 text-[12px] font-semibold italic ${dash.textMid}`}>
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
                      const isToday = date.toDateString() === new Date().toDateString();
                      return (
                        <div
                          key={i}
                          onClick={() => handleCellClick(dateStr, h, res)}
                          onContextMenu={(e) => handleContextMenu(e, dateStr, h, res)}
                          className={`group relative cursor-pointer border-l-2 border-slate-300 p-1.5 transition-all sm:h-[120px] ${
                            isToday ? "bg-[color:var(--dash-soft)]/40" : "bg-white"
                          }`}
                        >
                          {res ? (
                            <>
                              <div
                                className={`flex h-full w-full flex-col justify-between rounded-xl p-2 text-[11px] ${colorPresentation?.className || ""} ${
                                  isOwn ? "ring-2 ring-[color:var(--dash-primary)] ring-inset" : ""
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
                                  <span className="font-semibold uppercase opacity-80">{res.lastName}</span>
                                  {canModify ? <span className="text-[10px] sm:hidden">✎</span> : null}
                                </div>
                              </div>
                              <div
                                className={`pointer-events-none absolute left-1/2 z-[100] w-72 -translate-x-1/2 rounded-xl border-2 border-slate-200 bg-white p-4 opacity-0 shadow-xl transition-all group-hover:opacity-100 ${
                                  h <= 10 ? "top-full mt-2" : "bottom-full mb-2"
                                }`}
                              >
                                <p className={`mb-1 break-words text-[15px] font-semibold uppercase leading-tight ${dash.textPrimary}`}>
                                  {res.subject} - {res.className}
                                </p>
                                <p className={`mb-3 text-sm font-semibold ${dash.ink}`}>
                                  Par : {res.firstName} {res.lastName}
                                </p>
                                {res.comment ? (
                                  <div className={`rounded-lg border p-3 ${dash.borderSoft} ${dash.bgSoft50}`}>
                                    <p className={`whitespace-normal break-words text-sm italic leading-relaxed ${dash.textMid}`}>
                                      &apos;{res.comment}&apos;
                                    </p>
                                  </div>
                                ) : null}
                                <div
                                  className={`absolute left-1/2 h-3 w-3 -translate-x-1/2 rotate-45 border-slate-200 bg-white ${
                                    h <= 10 ? "-top-1.5 border-l border-t" : "-bottom-1.5 border-b border-r"
                                  }`}
                                />
                              </div>
                            </>
                          ) : (
                            <div className="flex h-full w-full items-center justify-center rounded-lg border-2 border-dashed border-slate-300 bg-slate-50/80 text-[10px] font-semibold text-slate-400 transition group-hover:border-[var(--dash-primary)] group-hover:bg-[color:var(--dash-soft)] group-hover:text-[var(--dash-primary)]">
                              + Libre
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
                      className={`cursor-pointer rounded-xl border-2 bg-white p-3 text-left shadow-sm transition hover:-translate-y-0.5 hover:border-[var(--dash-primary)] hover:bg-white ${dash.border}`}
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
                      </div>
                    </button>
                  ))}
                </div>
              </ProfRoomGlassCard>
            ) : null}

            <ProfRoomGlassCard
              id="form-section"
              data-tour="prof-room-form"
              className="border-2 !border-[color:var(--dash-primary)]/35 bg-[color:var(--dash-soft-muted)]"
              bodyClassName="p-4 md:p-8"
            >
              <div className="mb-8 flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
                <div className="flex min-w-0 items-center gap-3">
                  <div
                    className={`shrink-0 rounded-2xl px-3 py-2 text-xs font-semibold uppercase tracking-wide text-white ${
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
              <div className="grid grid-cols-1 gap-8">
                <div className="space-y-4">
                  <label className={`text-[10px] font-semibold uppercase tracking-[0.18em] ${dash.textMid}`}>
                    Professeur & cours
                  </label>
                  {isAdmin ? (
                    <div className="flex gap-2">
                      <input
                        type="text"
                        placeholder="Prénom"
                        value={targetFirstName}
                        onChange={(e) => setTargetFirstName(e.target.value)}
                        className={`${fieldClass} flex-1`}
                      />
                      <input
                        type="text"
                        placeholder="NOM"
                        value={targetLastName}
                        onChange={(e) => setTargetLastName(e.target.value.toUpperCase())}
                        className={`${fieldClass} flex-1`}
                      />
                    </div>
                  ) : (
                    <div className={`rounded-xl border bg-white/70 p-4 text-sm font-semibold ${dash.border}`}>
                      <p className={`mb-1 text-[10px] uppercase tracking-wide ${dash.textMid}`}>Identité Clerk :</p>
                      <span className={dash.textPrimary}>
                        {user.firstName} {lastName}
                      </span>
                    </div>
                  )}
                  <select value={subject} onChange={(e) => setSubject(e.target.value)} className={`${fieldClass} cursor-pointer`}>
                    <option value="">-- MATIÈRE --</option>
                    {Object.keys(SUBJECT_COLORS).map((s) => (
                      <option key={s} value={s}>
                        {s}
                      </option>
                    ))}
                  </select>
                  <div className="flex gap-2">
                    <select value={level} onChange={(e) => setLevel(e.target.value)} className={`${fieldClass} flex-1 cursor-pointer`}>
                      <option value="">NIVEAU</option>
                      {Object.keys(CLASSES_DATA).map((l) => (
                        <option key={l} value={l}>
                          {l}
                        </option>
                      ))}
                    </select>
                    <select
                      value={className}
                      onChange={(e) => setClassName(e.target.value)}
                      className={`${fieldClass} flex-1 cursor-pointer`}
                    >
                      <option value="">CLASSE</option>
                      {level && CLASSES_DATA[level].map((c) => (
                        <option key={c} value={c}>
                          {c}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>
                <div className="space-y-4">
                  <label className={`w-full text-[10px] font-semibold uppercase tracking-[0.18em] ${dash.textMid}`}>
                    Calendrier
                  </label>
                  <div className="w-full overflow-hidden">
                    <input
                      type="date"
                      value={selectedDate}
                      min={todayStr}
                      max={maxDateStr}
                      onChange={(e) => setSelectedDate(e.target.value)}
                      className={`${fieldClass} block text-[16px]`}
                    />
                  </div>
                  <div className="rounded-xl border-2 border-slate-300 bg-white p-4">
                    <p className={`mb-2 text-[10px] font-semibold uppercase tracking-wide ${dash.textMid}`}>
                      Choisir l&apos;heure :
                    </p>
                    <div className="flex flex-wrap gap-2">
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
                            className={`relative rounded-lg px-3 py-1.5 text-xs font-semibold transition-all ${
                              selectedHours.includes(h)
                                ? `${dash.bgPrimary} z-10 scale-110 text-white shadow-md`
                                : isTaken
                                  ? "cursor-not-allowed border border-rose-200 bg-rose-50 text-rose-400"
                                  : `cursor-pointer border-2 border-slate-300 bg-white ${dash.ink} hover:border-[var(--dash-primary)] hover:bg-[color:var(--dash-soft)]`
                            }`}
                          >
                            {h}h30
                          </button>
                        );
                      })}
                    </div>
                  </div>
                </div>
                <div className="space-y-4">
                  <label className={`text-[10px] font-semibold uppercase tracking-[0.18em] ${dash.textMid}`}>
                    Notes & répétition
                  </label>
                  <textarea
                    placeholder="Commentaire (ex: Valise PC)"
                    value={comment}
                    onChange={(e) => setComment(e.target.value)}
                    className={`${fieldClass} h-20 resize-none`}
                  />
                  <select value={recurrence} onChange={(e) => setRecurrence(e.target.value)} className={`${fieldClass} cursor-pointer`}>
                    <option value="none">Une seule fois</option>
                    <option value="weekly">Toutes les semaines</option>
                    <option value="biweekly">Toutes les 2 semaines</option>
                  </select>
                  {recurrence !== "none" ? (
                    <input
                      type="date"
                      value={untilDate}
                      min={selectedDate}
                      max={maxDateStr}
                      onChange={(e) => setUntilDate(e.target.value)}
                      className={`${fieldClass} border-amber-200 bg-amber-50/70 text-amber-900`}
                    />
                  ) : null}
                </div>
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
              <div className="mt-10 flex gap-4 sm:max-md:flex-col">
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