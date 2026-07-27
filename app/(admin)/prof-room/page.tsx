"use client";
import React, { Suspense, useEffect, useState, useMemo } from "react";
import { useSearchParams } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import { useAppContext } from "@/app/hooks/useAppContext";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole } from "@/app/lib/intranet-role-utils";
import ProfRoomSettingsTab from "@/app/components/prof-room/ProfRoomSettingsTab";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import { DEFAULT_PROF_ROOM_SUBJECT_COLORS } from "@/app/lib/prof-room-defaults";
import { getSubjectColorPresentation } from "@/app/lib/prof-room-subject-colors";

const FALLBACK_CLASSES: Record<string, string[]> = {
  "ÉCOLE": ["CP", "CE1", "CE2", "CM1", "CM2"],
  "COLLÈGE": ["6A","6B","6C","6D","6E","6F","5A","5B","5C","5D","5E","5F","4A","4B","4C","4D","4E","4F","3A","3B","3C","3D","3E","3F"],
  "LYCÉE": ["2A","2B","2C","2D","2E","1A","1B","1C","1D","1E","1F","TA","TB","TC","TD","TE","TF"],
  "MAINTENANCE": ["MAINTENANCE"],
};

const HOURS = Array.from({ length: 10 }, (_, i) => 8 + i);
const DAYS = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi"];

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
  if (!isLoaded || !user) return <div className="p-20 text-center font-bold">Initialisation...</div>;
  return (
    <div className="px-0 py-4 md:px-4 pb-0 sm:pb-4 max-w-6xl mx-auto">
      {contextMenu && (
        <div className="fixed z-[100] bg-white shadow-2xl border rounded-xl p-1 min-w-[180px] text-xs font-bold overflow-hidden" style={{ top: contextMenu.y, left: contextMenu.x }}>
          {contextMenu.res ? (
            <button onClick={() => copyReservation(contextMenu.res)} className="w-full text-left p-3 hover:bg-blue-50 flex items-center gap-2 rounded-lg transition-colors">
              <span>📋</span> Copier ce créneau
            </button>
          ) : clipboard ? (
            <button onClick={() => pasteReservation(contextMenu.dateStr!, contextMenu.hour!)} className="w-full text-left p-3 hover:bg-green-50 flex items-center gap-2 rounded-lg transition-colors">
              <span>📥</span> Coller : {clipboard.subject} ({clipboard.className})
            </button>
          ) : (
            <div className="p-3 text-gray-400 italic">Rien à coller...</div>
          )}
        </div>
      )}
      <h1 className="text-4xl font-black text-slate-900 tracking-tight p-4">Réservation de salles</h1>
      <div className="flex flex-wrap gap-2 px-4 pb-4">
        <button
          type="button"
          data-prof-room-tab="reservation"
          onClick={() => setActiveTab("reservation")}
          className={`px-5 py-2.5 rounded-xl text-sm font-black ${activeTab === "reservation" ? "bg-blue-600 text-white" : "bg-slate-100 text-slate-700"}`}
        >
          Réservation
        </button>
        {canAccessSettings && (
          <button
            type="button"
            onClick={() => setActiveTab("settings")}
            className={`px-5 py-2.5 rounded-xl text-sm font-black ${activeTab === "settings" ? "bg-purple-600 text-white" : "bg-slate-100 text-slate-700"}`}
          >
            Paramétrage
          </button>
        )}
      </div>
      {activeTab === "settings" && canAccessSettings ? (
        <ProfRoomSettingsTab />
      ) : (
      <>
      <div data-tour="prof-room-room-select" className="bg-white rounded-2xl p-4 flex flex-col md:flex-row md:justify-between md:items-center gap-3 w-full">
        <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-3 w-full md:w-1/2">
          <select
            value={selectedRoom}
            onChange={(e) => setSelectedRoom(e.target.value)}
            className="bg-blue-600 w-full text-center text-white font-black px-4 p-3 rounded-xl outline-none"
          >
            {rooms.map((r) => (
              <option key={r.id} value={r.id}>
                {r.name}
              </option>
            ))}
          </select>
          <div className="flex items-center bg-gray-100 rounded-xl w-full justify-between">
            {!isMobile ? (
              <button onClick={() => setCurrentDate(new Date(currentDate.setDate(currentDate.getDate() - 7)))} className="p-2 py-3 hover:bg-white rounded-lg">◀</button>
            ) : (
              <span className="w-8" aria-hidden />
            )}
            <div className="px-4 text-[12px] font-black uppercase text-center">
              {isMobile ? (
                <>
                  Aujourd&apos;hui
                  <br />
                  <span className="text-blue-600">
                    {new Date().toLocaleDateString("fr-FR", { weekday: "long", day: "numeric", month: "short" })}
                  </span>
                </>
              ) : (
                <>
                  Semaine du <br />
                  <span className="text-blue-600">
                    {startOfWeek.toLocaleDateString("fr-FR", { day: "numeric", month: "short" })}
                  </span>
                </>
              )}
            </div>
            {!isMobile ? (
              <button onClick={() => setCurrentDate(new Date(currentDate.setDate(currentDate.getDate() + 7)))} className="p-2 hover:bg-white rounded-lg">▶</button>
            ) : (
              <span className="w-8" aria-hidden />
            )}
          </div>
        </div>
        <div className="flex items-center justify-between md:justify-end w-full md:w-1/2 gap-3">
          <label className="flex items-center gap-2 flex-1 min-w-0 bg-white border-2 border-blue-100 rounded-full px-3 py-1 cursor-pointer">
            <span className="text-blue-400 text-sm flex-shrink-0">📅</span>
            <input type="date" onChange={(e) => setCurrentDate(new Date(e.target.value))} className="flex-1 min-w-0 w-full text-[15px] bg-transparent outline-none text-slate-600 font-semibold"/>
          </label>
          {isAdmin && <span className="bg-purple-600 text-white text-[15px] font-black px-3 py-1 rounded-full tracking-tighter whitespace-nowrap">ADMIN MODE</span>}
        </div>
      </div>
      <div data-tour="prof-room-calendar" className="bg-white rounded-3xl overflow-hidden">
        <div className={`grid ${isMobile ? "grid-cols-2" : "grid-cols-6"} bg-gray-50 border-b`}>
          <div className="p-4 text-[13px] font-black text-gray-400 uppercase text-center">Heure</div>
          {displayDays.map((day, i) => (
            <div key={`${day.label}-${i}`} className={`p-4 text-center border-l ${day.date.toDateString() === new Date().toDateString() ? "bg-blue-50" : ""}`}>
              <p className="text-[10px] uppercase font-bold text-gray-400">{day.label}</p>
              <p className="text-xl font-black">{day.date.getDate()}</p>
            </div>
          ))}
        </div>
        <div className="divide-y">
          {HOURS.map(h => (
            <div key={h} className={`grid ${isMobile ? "grid-cols-2" : "grid-cols-6"} min-h-[95px]`}>
              <div className="text-[13px] font-black text-gray-400 flex items-center justify-center bg-gray-50/50 italic">{h}h30</div>
              {displayDays.map((day, i) => {
                const date = day.date;
                const dateStr = date.toISOString().split("T")[0];
                const hourPrefix = `${dateStr}T${h.toString().padStart(2, "0")}`;
                const res = reservations.find(r => r.roomId === selectedRoom && r.startsAt.startsWith(hourPrefix) && r.status !== "CANCELLED");
                const isOwn = res?.userId === user.id;
                const canModify = isAdmin || isOwn;
                const colorValue = res ? (SUBJECT_COLORS[res.subject] || "bg-slate-600 text-white") : "";
                const colorPresentation = res ? getSubjectColorPresentation(colorValue) : null;
                return (
                  <div key={i} onClick={() => handleCellClick(dateStr, h, res)} onContextMenu={(e) => handleContextMenu(e, dateStr, h, res)} className={`border-l relative p-1 transition-all sm:h-[120px] group ${!res ? 'hover:bg-green-50' : 'cursor-pointer'}`}>
                    {res ? (
                      <>
                        <div
                          className={`h-full w-full rounded-xl p-2 text-[11px] flex flex-col justify-between ${colorPresentation?.className || ""} ${isOwn ? "ring-2 ring-blue-400 ring-inset" : ""}`}
                          style={colorPresentation?.style}
                        >
                          <div>
                            <div className="flex justify-between items-start sm:flex-col">
                              <p className="font-black uppercase leading-none truncate">{res.subject}</p>
                              <span className="bg-white/20 px-1 rounded text-[11px] font-bold">{res.className}</span>
                            </div>
                            {res.comment && (
                              <p className="mt-1 italic opacity-90 leading-tight border-t border-white/10 pt-1 whitespace-normal break-words sm:line-clamp-3">
                                &apos;{res.comment}&apos;
                              </p>
                            )}
                          </div>
                          <div className="flex justify-between items-end mt-1">
                            <span className="font-bold opacity-80 uppercase ">{res.lastName}</span>
                            {canModify && <span className="text-[10px] sm:hidden">✎</span>}
                          </div>
                        </div>
                        <div className={`absolute left-1/2 -translate-x-1/2 w-72 bg-slate-900 text-white p-4 rounded-xl shadow-2xl  opacity-0 group-hover:opacity-100 pointer-events-none transition-all z-[100] ${h <= 10 ? 'top-full mt-2' : 'bottom-full mb-2'}`}>
                          <p className="text-[16px] font-black text-blue-400 uppercase mb-1 break-words leading-tight">{res.subject} - {res.className}</p>
                          <p className="text-[15px] font-bold mb-3 opacity-90">Par : {res.firstName} {res.lastName}</p>
                          {res.comment && (
                            <div className="bg-white/10 p-3 rounded-lg border border-white/5">
                              <p className="text-[14px] leading-relaxed italic text-slate-200 whitespace-normal break-words">&apos;{res.comment}&apos;</p>
                            </div>
                          )}
                          <div className={`absolute left-1/2 -translate-x-1/2 w-3 h-3 bg-slate-900 rotate-45 ${h <= 10 ? '-top-1.5' : '-bottom-1.5'}`}></div>
                        </div> 
                      </>
                    ) : (
                      <div className="h-full w-full flex items-center justify-center opacity-0 hover:opacity-100 transition-opacity">
                        <span className="text-[10px] font-black text-green-600">+ LIBRE</span>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>
      {myUpcomingReservations.length > 0 && (
        <div data-tour="prof-room-upcoming" className="bg-white border-2 border-blue-100 rounded-3xl p-6 shadow-lg">
          <h3 className="text-sm font-black text-blue-600 uppercase mb-4 flex items-center gap-2">📅 Mes 5 prochaines réservations</h3>
          <div className="grid grid-cols-1 md:grid-cols-5 gap-3">
            {myUpcomingReservations.map((res) => (
              <div 
                key={res.id} 
                onClick={() => {
                    const dStr = res.startsAt.split("T")[0];
                    const hNum = parseInt(res.startsAt.split("T")[1].split(":")[0]);
                    handleCellClick(dStr, hNum, res);
                }}
                className="bg-gray-50 hover:bg-blue-50 border border-gray-100 rounded-2xl p-3 cursor-pointer transition-all"
              >
                <p className="text-[10px] font-black text-gray-400 uppercase">
                  {new Date(res.startsAt).toLocaleDateString("fr-FR", { weekday: 'short', day: 'numeric', month: 'short' })}
                </p>
                <p className="text-xs font-black text-blue-700">{res.startsAt.split("T")[1].substring(0, 5).replace(":", "h")}</p>
                <div className="mt-2 text-[10px] font-bold">
                  <span className="block truncate">📍 {rooms.find(r => r.id === res.roomId)?.name || "Salle"}</span>
                  <span className="block text-gray-500">📚 {res.subject} ({res.className})</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
      <div id="form-section" data-tour="prof-room-form" className="bg-slate-900 rounded-b-none sm:rounded-b-[40px] rounded-[40px] p-4 md:p-8 text-white shadow-2xl mt-6">
        <div className="flex flex-col md:flex-row md:justify-between md:items-start gap-4 mb-8">
          <div className="flex items-center gap-3 min-w-0">
            <div className={`p-2 md:p-3 rounded-2xl flex-shrink-0 ${isEditing ? 'bg-orange-500' : 'bg-green-500'}`}>
              <span className="text-base md:text-xl font-bold">{isEditing ? 'MODIFIER' : 'RÉSERVER'}</span>
            </div>
            <h2 className="text-xl md:text-2xl font-black uppercase italic tracking-tighter leading-tight">{isEditing ? "Détails du créneau" : "Nouvelle demande"}</h2>
          </div>
          {isEditing && (
            <button onClick={handleDelete} className="w-full md:w-auto bg-red-600 hover:bg-red-500 text-white text-xs font-black px-6 py-3 rounded-2xl shadow-lg transition-transform active:scale-90">🗑️ SUPPRIMER CE CRÉNEAU</button>
          )}
        </div>
        <div className="grid grid-cols-1 gap-8">
          <div className="space-y-4">
            <label className="text-[10px] font-black text-slate-500 uppercase tracking-widest">Professeur & Cours</label>
            {isAdmin ? (
              <div className="flex gap-2">
                <input type="text" placeholder="Prénom" value={targetFirstName} onChange={(e) => setTargetFirstName(e.target.value)} className="flex-1 bg-slate-800 border-none rounded-xl p-3 text-xs font-bold text-blue-400" />
                <input type="text" placeholder="NOM" value={targetLastName} onChange={(e) => setTargetLastName(e.target.value.toUpperCase())} className="flex-1 bg-slate-800 border-none rounded-xl p-3 text-xs font-bold text-blue-400" />
              </div>
            ) : (
              <div className="bg-slate-800 p-4 rounded-xl text-sm font-bold border border-slate-700">
                <p className="text-[10px] text-slate-500 uppercase mb-1">Identité Clerk :</p>
                <span className="text-blue-400">{user.firstName} {lastName}</span>
              </div>
            )}
            <select value={subject} onChange={(e) => setSubject(e.target.value)} className="w-full bg-slate-800 border-none rounded-xl p-4 text-sm font-bold focus:ring-2 ring-blue-500">
              <option value="">-- MATIÈRE --</option>
              {Object.keys(SUBJECT_COLORS).map(s => <option key={s} value={s}>{s}</option>)}
            </select>
            <div className="flex gap-2">
              <select value={level} onChange={(e) => setLevel(e.target.value)} className="flex-1 bg-slate-800 border-none rounded-xl p-4 text-xs font-bold">
                <option value="">NIVEAU</option>
                {Object.keys(CLASSES_DATA).map(l => <option key={l} value={l}>{l}</option>)}
              </select>
              <select value={className} onChange={(e) => setClassName(e.target.value)} className="flex-1 bg-slate-800 border-none rounded-xl p-4 text-xs font-bold">
                <option value="">CLASSE</option>
                {level && CLASSES_DATA[level].map(c => <option key={c} value={c}>{c}</option>)}
              </select>
            </div>
          </div>
          <div className="space-y-4">
            <label className="text-[10px] font-black text-slate-500 w-full uppercase tracking-widest">Calendrier</label>
            <div className="w-full overflow-hidden">
              <input type="date" value={selectedDate} min={todayStr} max={maxDateStr} onChange={(e) => setSelectedDate(e.target.value)} className="w-full block bg-slate-800 border-none rounded-xl px-4 py-3 text-[16px] font-bold text-white" style={{ colorScheme: "dark" }} />
            </div>
            <div className="p-4 bg-slate-800/50 border border-slate-700 rounded-xl">
              <p className="text-[10px] font-bold text-slate-500 mb-2">Choisir l&apos;heure :</p>
              <div className="flex flex-wrap gap-2">
                {HOURS.map(h => {
                  const hourPrefix = `${selectedDate}T${h.toString().padStart(2, "0")}`;
                  const isTaken = reservations.some(r => 
                    r.roomId === selectedRoom && 
                    r.startsAt.startsWith(hourPrefix) && 
                    r.status !== "CANCELLED" &&
                    r.id !== editingRes?.id
                  );
                  return (
                    <button
                      key={h}
                      type="button"
                      disabled={isTaken}
                      onClick={() => setSelectedHours([h])}
                      className={`relative px-3 py-1 rounded-lg font-black text-xs transition-all ${
                        selectedHours.includes(h) 
                        ? "bg-blue-600 text-white shadow-lg scale-110 z-10" 
                        : isTaken 
                          ? "bg-red-900/50 text-red-200 cursor-not-allowed border border-red-700/70" 
                          : "bg-slate-700 text-slate-400 hover:bg-slate-600"
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
            <label className="text-[10px] font-black text-slate-500 uppercase tracking-widest">Notes & Répétition</label>
            <textarea placeholder="Commentaire (ex: Valise PC)" value={comment} onChange={(e) => setComment(e.target.value)} className="w-full bg-slate-800 border-none rounded-xl p-4 text-sm font-bold h-20 resize-none focus:ring-2 ring-blue-500" />
            <select value={recurrence} onChange={(e) => setRecurrence(e.target.value)} className="w-full bg-slate-800 border-none rounded-xl p-4 text-xs font-bold">
              <option value="none">Une seule fois</option>
              <option value="weekly">Toutes les semaines</option>
              <option value="biweekly">Toutes les 2 semaines</option>
            </select>
            {recurrence !== "none" && (
              <input type="date" value={untilDate} min={selectedDate} max={maxDateStr} onChange={(e) => setUntilDate(e.target.value)} className="w-full bg-orange-900/30 border border-orange-500/50 rounded-xl p-3 text-xs font-bold text-orange-400" />
            )}
          </div>
        </div>
        {isEditing && editingRes?.groupId && (
          <div className="mt-6 p-4 bg-blue-900/30 border border-blue-500/50 rounded-2xl flex items-center gap-3">
            <input 
              type="checkbox" 
              id="updateSeries" 
              checked={updateAllSeries} 
              onChange={(e) => setUpdateAllSeries(e.target.checked)}
              className="w-5 h-5 rounded border-slate-700 bg-slate-800 text-blue-600 focus:ring-blue-500"
            />
            <label htmlFor="updateSeries" className="text-sm font-bold text-blue-400 cursor-pointer">🔄 Appliquer les modifications à TOUTE la série de réservations</label>
          </div>
        )}
        <div className="mt-10 flex gap-4 sm:max-md:flex-col">
          <button onClick={handleConfirm} className="flex-1 bg-blue-600 hover:bg-blue-500 text-white font-black py-4 rounded-2xl shadow-xl transition-all active:scale-95 text-lg">
            {isEditing ? "ENREGISTRER LES MODIFICATIONS" : "CONFIRMER LA RÉSERVATION"}
          </button>
          <button onClick={() => { setIsEditing(false); setEditingRes(null); setSubject(""); setClassName(""); setComment(""); setLevel(""); }} className="bg-slate-700 px-8 rounded-2xl font-bold hover:bg-slate-600 transition-colors sm:py-4">ANNULER</button>
        </div>
      </div>
      </>
      )}
      <ReplayModuleTourButton moduleId="prof-room" />
    </div>
  );
}

export default function ProfRoomPage() {
  return (
    <Suspense fallback={<div className="p-8 text-slate-500 text-sm">Chargement des salles…</div>}>
      <ProfRoomPageContent />
    </Suspense>
  );
}