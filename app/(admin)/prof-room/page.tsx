"use client";
import React, { Suspense, useEffect, useState, useMemo } from "react";
import { useSearchParams } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
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
import { getReservationTilePresentation } from "@/app/lib/prof-room-subject-colors";
import {
  isReservationBookedForOther,
  reservationWhoCompact,
  reservationWhoLabel,
} from "@/app/lib/prof-room-reservation-label";
import { isReservationBeneficiary } from "@/app/lib/prof-room-reservation-ownership";
import { dash } from "@/app/lib/dashboard-brand";
import { DEFAULT_CLASSES_BY_POLE, resolveProfRoomClassesByPole } from "@/app/lib/school-classes-catalog";
import {
  normalizeRoomReservationsList,
  reservationMatchesHourPrefix,
} from "@/app/lib/prof-room-reservations-normalize";
import ProfRoomBeneficiarySelect, {
  type ProfRoomBeneficiary,
} from "@/app/components/prof-room/ProfRoomBeneficiarySelect";

const FALLBACK_CLASSES: Record<string, string[]> = {
  ...DEFAULT_CLASSES_BY_POLE,
  MAINTENANCE: ["MAINTENANCE"],
};

const HOURS = Array.from({ length: 10 }, (_, i) => 8 + i);
const DAYS = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi"];

type EdtOccupancyCell = {
  date: string;
  hour: number;
  start: string;
  end: string;
  subject: string;
  classes: string[];
  teacherName: string;
  weekType: "A" | "B" | "replacement";
  room: string;
  source: "edt" | "replacement";
};

type EdtOccupancyPayload = {
  from: string;
  to: string;
  roomId: string;
  roomName: string;
  weekABByDate: Record<string, "A" | "B">;
  cells: EdtOccupancyCell[];
};

function localYmd(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function parseLocalYmd(value: string): Date {
  const [y, m, d] = value.split("-").map(Number);
  return new Date(y, (m || 1) - 1, d || 1);
}

function shiftDate(base: Date, days: number): Date {
  const next = new Date(base.getFullYear(), base.getMonth(), base.getDate() + days);
  return next;
}

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
  const { user, isLoaded } = useSessionUser();
  const { data: appCtx } = useAppContext();
  const isOrgAdmin = useIsOrgAdmin();
  const CLASSES_DATA = useMemo(() => {
    const raw = appCtx?.profRoom?.classesByPole;
    if (raw && Object.keys(raw).length > 0) {
      return resolveProfRoomClassesByPole(raw);
    }
    return FALLBACK_CLASSES;
  }, [appCtx?.profRoom?.classesByPole]);
  const SUBJECT_COLORS = { ...DEFAULT_PROF_ROOM_SUBJECT_COLORS, ...(appCtx?.profRoom?.subjectColors || {}) };
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [rooms, setRooms] = useState<any[]>([]);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [reservations, setReservations] = useState<any[]>([]);
  const [edtOccupancy, setEdtOccupancy] = useState<EdtOccupancyPayload | null>(null);
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
  const [beneficiary, setBeneficiary] = useState<ProfRoomBeneficiary | null>(null);
  const [bookForOther, setBookForOther] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [activeTab, setActiveTab] = useState<"reservation" | "settings">("reservation");
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [editingRes, setEditingRes] = useState<any>(null);
  const [feedback, setFeedback] = useState<{ kind: "ok" | "warn" | "err"; text: string } | null>(
    null,
  );
  const [deleting, setDeleting] = useState(false);
  const lastName = (user?.lastName || "").toUpperCase();
  const intranetRoles = intranetRolesFromMetadata(user?.publicMetadata);
  const isGlobalAdmin =
    isOrgAdmin ||
    hasGlobalAdminRole(intranetRoles) ||
    appCtx?.session?.isGlobalAdmin === true;
  const adminExternalUserIds = appCtx?.profRoom?.adminExternalUserIds || [];
  const isModuleListedAdmin = user?.id ? adminExternalUserIds.includes(user.id) : false;
  const canAccessSettings = isGlobalAdmin || isModuleListedAdmin;
  const isAdmin = canAccessSettings;
  const canBookForOthers = isModuleListedAdmin;
  const todayStr = localYmd(new Date());
  const maxDateLimit = new Date();
  maxDateLimit.setDate(maxDateLimit.getDate() + (appCtx?.profRoom?.bookingHorizonDays ?? 56));
  const maxDateStr = isAdmin ? "" : localYmd(maxDateLimit);
  const myUpcomingReservations = useMemo(() => {
    if (!user?.id) return [];
    const nowIso = new Date().toISOString();
    const viewer = {
      userId: user.id,
      email: user.primaryEmailAddress?.emailAddress ?? undefined,
      firstName: user.firstName ?? undefined,
      lastName: user.lastName ?? undefined,
    };
    return reservations
      .filter(
        (r) =>
          isReservationBeneficiary(r, viewer) &&
          r.status !== "CANCELLED" &&
          typeof r.startsAt === "string" &&
          r.startsAt >= nowIso,
      )
      .sort((a, b) => String(a.startsAt || "").localeCompare(String(b.startsAt || "")))
      .slice(0, 5);
  }, [reservations, user]);
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
    const selectedIndex = weekDays.findIndex(
      (date) => date.toDateString() === currentDate.toDateString(),
    );
    if (selectedIndex >= 0) {
      return [{ date: weekDays[selectedIndex], label: DAYS[selectedIndex] }];
    }
    return [
      {
        date: currentDate,
        label: currentDate.toLocaleDateString("fr-FR", { weekday: "long" }),
      },
    ];
  }, [isMobile, weekDays, currentDate]);
  useEffect(() => {
    async function load() {
      try {
        const [roomsRes, resRes] = await Promise.all([
          fetch("/api/reservation-rooms/rooms"),
          fetch("/api/reservation-rooms/reservations")
        ]);
        if (roomsRes.ok) {
          const data = await roomsRes.json();
          const allRooms = (data.rooms || []) as Array<{
            id: string;
            name: string;
            bookable?: boolean;
            kind?: string;
          }>;
          setRooms(allRooms);
          const bookable = allRooms.filter((r) => r.bookable !== false);
          const initial = bookable[0] || allRooms[0];
          if (initial) setSelectedRoom(initial.id);
        }
        if (resRes.ok) {
          const body = await resRes.json();
          setReservations(normalizeRoomReservationsList(body.reservations));
        }
      } catch (error) {
        console.error(error);
      }
    }
    void load();
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

  useEffect(() => {
    if (!bookForOther) setBeneficiary(null);
  }, [bookForOther]);

  useEffect(() => {
    if (!selectedRoom) {
      setEdtOccupancy(null);
      return;
    }
    const from = localYmd(weekDays[0] || startOfWeek);
    const to = localYmd(weekDays[4] || startOfWeek);
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch(
          `/api/reservation-rooms/edt-occupancy?roomId=${encodeURIComponent(selectedRoom)}&from=${from}&to=${to}`,
          { cache: "no-store" },
        );
        if (!res.ok) return;
        const j = await res.json();
        if (!cancelled && j.occupancy) setEdtOccupancy(j.occupancy as EdtOccupancyPayload);
      } catch (err) {
        console.error("[prof-room] edt-occupancy", err);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [selectedRoom, startOfWeek, weekDays]);

  const edtCellAt = (dateStr: string, hour: number): EdtOccupancyCell | undefined =>
    edtOccupancy?.cells.find((c) => c.date === dateStr && c.hour === hour);

  const weekParityLabel = useMemo(() => {
    const mid = weekDays[2] || startOfWeek;
    const iso = localYmd(mid);
    return edtOccupancy?.weekABByDate?.[iso] || null;
  }, [edtOccupancy, weekDays, startOfWeek]);

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const handleCellClick = (dateStr: string, hour: number, resExist?: any) => {
    if (!resExist && edtCellAt(dateStr, hour)) {
      const edt = edtCellAt(dateStr, hour)!;
      alert(
        `Créneau occupé par l’emploi du temps (${edt.teacherName} — ${edt.subject}${
          edt.classes.length ? `, ${edt.classes.join(", ")}` : ""
        }, sem. ${edt.weekType}). Non réservable.`,
      );
      return;
    }
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
        const forOther = canBookForOthers && isReservationBookedForOther(resExist);
        setBookForOther(forOther);
        if (forOther) {
          const maybeOwnerId =
            typeof resExist.userId === "string" ? resExist.userId : undefined;
          const bookedById =
            typeof resExist.bookedByUserId === "string" ? resExist.bookedByUserId : undefined;
          // Nouveau modèle : userId bénéficiaire ≠ bookedByUserId. Legacy : userId = booker → on force le rematch annuaire.
          const keepUserId =
            maybeOwnerId && bookedById && maybeOwnerId !== bookedById ? maybeOwnerId : undefined;
          setBeneficiary({
            userId: keepUserId,
            firstName: String(resExist.firstName || "").trim(),
            lastName: String(resExist.lastName || "").trim().toUpperCase(),
            email: typeof resExist.email === "string" ? resExist.email : undefined,
            source: keepUserId ? "directory" : "manual",
          });
        } else {
          setBeneficiary(null);
        }
        document.getElementById("form-section")?.scrollIntoView({ behavior: "smooth" });
      }
    } else {
      setIsEditing(false);
      setEditingRes(null);
      setSelectedDate(dateStr);
      setSelectedHours([hour]);
      setBeneficiary(null);
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
    if (edtCellAt(dateStr, hour)) {
      alert("Ce créneau est occupé par l’emploi du temps — collage impossible.");
      setContextMenu(null);
      return;
    }
    setIsEditing(false);
    setEditingRes(null);
    setSelectedDate(dateStr);
    setSelectedHours([hour]);
    setSubject(clipboard.subject);
    setClassName(clipboard.className);
    setComment(clipboard.comment || "");
    setBeneficiary(null);
    setBookForOther(false);
    setContextMenu(null);
    document.getElementById("form-section")?.scrollIntoView({ behavior: "smooth" });
  };
  async function handleConfirm() {
    if (!selectedRoom || !selectedDate || selectedHours.length === 0 || !subject || !className) {
      alert("Veuillez remplir tous les champs obligatoires.");
      return;
    }
    if (canBookForOthers && bookForOther) {
      if (!beneficiary?.firstName?.trim() || !beneficiary?.lastName?.trim()) {
        alert("Choisissez la personne pour qui vous réservez dans l’annuaire du personnel.");
        return;
      }
      if (!beneficiary.userId) {
        alert(
          "Sélectionnez une personne de l’annuaire (pas une saisie libre) pour qu’elle voie la réservation sur son tableau de bord.",
        );
        return;
      }
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
      bookForOther: Boolean(canBookForOthers && bookForOther),
      firstName: canBookForOthers && bookForOther ? beneficiary!.firstName : user?.firstName,
      lastName:
        canBookForOthers && bookForOther ? beneficiary!.lastName.toUpperCase() : lastName,
      email: userEmail,
      beneficiaryUserId:
        canBookForOthers && bookForOther ? beneficiary!.userId : undefined,
      beneficiaryEmail:
        canBookForOthers && bookForOther ? beneficiary!.email || undefined : undefined,
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
          if (resRes.ok) {
            const body = await resRes.json();
            setReservations(normalizeRoomReservationsList(body.reservations));
          }
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
        `❌ Échec réseau (souvent timeout mail SMTP ou auth).\nDétail : ${
          err instanceof Error ? err.message : String(err)
        }\nRegarde la console (filtre « prof-room »).`,
      );
    }
  }

  async function handleDelete() {
    if (!editingRes || deleting) return;
    const target = editingRes;
    const reason = prompt("Motif de suppression :", "Annulation");
    if (reason === null) return;
    let deleteAllSeries = false;
    if (target.groupId) {
      deleteAllSeries = confirm("Supprimer TOUTE la série ?");
    }
    const sessionEmail = user?.primaryEmailAddress?.emailAddress || "";
    const notifyEmail = String(target.email || sessionEmail || "").trim();
    const payload = {
      id: target.id,
      groupId: target.groupId,
      deleteAllSeries,
      reason,
      userEmail: notifyEmail,
      startsAt: target.startsAt,
    };

    console.info("[prof-room] delete →", {
      id: payload.id,
      email: notifyEmail || "(vide)",
      deleteAllSeries,
    });

    setDeleting(true);
    setFeedback(null);
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
        cancelledIds?: string[];
        mailTo?: string[];
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
        mailTo: j.mailTo,
        error: j.error,
      });

      if (res.ok) {
        const cancelledIdSet = new Set<string>(
          Array.isArray(j.cancelledIds) && j.cancelledIds.length > 0
            ? j.cancelledIds
            : [target.id],
        );
        // Mise à jour immédiate de la grille (sans attendre le refresh).
        setReservations((prev) =>
          prev.map((r) => {
            const matchId = cancelledIdSet.has(r.id);
            const matchSeries =
              deleteAllSeries &&
              Boolean(target.groupId) &&
              r.groupId === target.groupId;
            return matchId || matchSeries ? { ...r, status: "CANCELLED" } : r;
          }),
        );
        setIsEditing(false);
        setEditingRes(null);
        setSelectedHours([]);

        let mailPart = "";
        if (j.mailSent === true) {
          const to = Array.isArray(j.mailTo) && j.mailTo.length ? j.mailTo.join(", ") : "destinataire";
          mailPart = ` Mail d'annulation envoyé à ${to}.`;
        } else {
          mailPart = ` Mail non envoyé : ${j.mailSkipReason || "raison inconnue"}.`;
          console.warn("[prof-room] mail annulation KO:", j.mailSkipReason || j);
        }
        const count = typeof j.cancelled === "number" ? j.cancelled : cancelledIdSet.size;
        const okText = `Créneau${count > 1 ? "s" : ""} supprimé${count > 1 ? "s" : ""} (${count}).${mailPart}`;
        setFeedback({
          kind: j.mailSent === true ? "ok" : "warn",
          text: okText,
        });
        alert(`🗑️ ${okText}`);

        try {
          const resRes = await fetch("/api/reservation-rooms/reservations");
          if (resRes.ok) {
            const body = await resRes.json();
            setReservations(normalizeRoomReservationsList(body.reservations));
          }
        } catch (reloadErr) {
          console.warn("[prof-room] refresh liste échoué", reloadErr);
        }
      } else {
        console.error("[prof-room] delete erreur", res.status, j);
        const errText = j.error || `Erreur HTTP ${res.status}`;
        setFeedback({ kind: "err", text: errText });
        alert(`❌ ${errText}`);
      }
    } catch (err) {
      console.error("[prof-room] delete réseau / Load failed", err);
      const errText = `Échec réseau à la suppression (${
        err instanceof Error ? err.message : String(err)
      }). Rechargez la page pour vérifier.`;
      setFeedback({ kind: "err", text: errText });
      alert(`❌ ${errText}`);
      try {
        const resRes = await fetch("/api/reservation-rooms/reservations");
        if (resRes.ok) {
          const body = await resRes.json();
          setReservations(normalizeRoomReservationsList(body.reservations));
        }
      } catch {
        /* ignore */
      }
    } finally {
      setDeleting(false);
    }
  }
  if (!isLoaded || !user) {
    return (
      <ModulePageShell maxWidthClass="max-w-[1400px]">
        <p className={`text-sm ${dash.textMid}`}>Initialisation…</p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1400px]" tourModuleId="prof-room">
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

        {feedback ? (
          <div
            role="status"
            className={`rounded-2xl border px-4 py-3 text-sm font-semibold ${
              feedback.kind === "ok"
                ? "border-emerald-200 bg-emerald-50 text-emerald-900"
                : feedback.kind === "warn"
                  ? "border-amber-200 bg-amber-50 text-amber-950"
                  : "border-red-200 bg-red-50 text-red-900"
            }`}
          >
            <div className="flex items-start justify-between gap-3">
              <p>{feedback.text}</p>
              <button
                type="button"
                onClick={() => setFeedback(null)}
                className="shrink-0 text-xs font-semibold uppercase tracking-wide opacity-70 hover:opacity-100"
              >
                Fermer
              </button>
            </div>
          </div>
        ) : null}

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
                    {rooms
                      .filter((r) => r.bookable !== false)
                      .map((r) => (
                        <option key={r.id} value={r.id}>
                          {r.name}
                        </option>
                      ))}
                  </select>
                </SelectShell>
                {rooms.some((r) => r.bookable === false) ? (
                  <p className={`text-[11px] ${dash.textMid} sm:max-w-[14rem]`}>
                    {rooms.filter((r) => r.bookable === false).length} salle(s) de classe
                    visibles au catalogue, pas encore réservables.
                  </p>
                ) : null}
                <div className="flex w-full items-center justify-between rounded-xl border border-[color:var(--dash-border)] bg-white/60">
                  <button
                    type="button"
                    onClick={() => setCurrentDate(shiftDate(currentDate, isMobile ? -1 : -7))}
                    className={`cursor-pointer rounded-lg p-2 py-3 ${dash.textMid} hover:bg-white/80`}
                  >
                    ◀
                  </button>
                  <div className={`px-4 text-center text-[11px] font-semibold uppercase tracking-wide ${dash.ink}`}>
                    {isMobile ? (
                      <>
                        {currentDate.toDateString() === new Date().toDateString()
                          ? "Aujourd'hui"
                          : currentDate.toLocaleDateString("fr-FR", { weekday: "long" })}
                        <br />
                        <span className={dash.textPrimary}>
                          {currentDate.toLocaleDateString("fr-FR", { day: "numeric", month: "short" })}
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
                  <button
                    type="button"
                    onClick={() => setCurrentDate(shiftDate(currentDate, isMobile ? 1 : 7))}
                    className={`cursor-pointer rounded-lg p-2 ${dash.textMid} hover:bg-white/80`}
                  >
                    ▶
                  </button>
                </div>
              </div>
              <div className="flex w-full items-center justify-between gap-3 md:w-1/2 md:justify-end">
                <label className="flex h-11 min-w-0 flex-1 cursor-pointer items-center gap-2 overflow-hidden rounded-full border border-[color:var(--dash-border)] bg-white/80 px-3 shadow-sm">
                  <span className="flex-shrink-0 text-sm" aria-hidden>
                    📅
                  </span>
                  <input
                    type="date"
                    value={localYmd(currentDate)}
                    onChange={(e) => setCurrentDate(parseLocalYmd(e.target.value))}
                    className={`min-w-0 w-full flex-1 bg-transparent text-[15px] font-semibold outline-none ${dash.ink}`}
                  />
                </label>
                {isAdmin ? (
                  <span className={`shrink-0 whitespace-nowrap rounded-full px-3 py-1 text-[11px] font-semibold tracking-wide text-white ${dash.bgPrimary}`}>
                    Mode admin
                  </span>
                ) : null}
                {weekParityLabel ? (
                  <span className="shrink-0 whitespace-nowrap rounded-full border border-indigo-200 bg-indigo-50 px-3 py-1 text-[11px] font-bold text-indigo-800">
                    Semaine {weekParityLabel}
                  </span>
                ) : null}
              </div>
            </ProfRoomGlassCard>

            <ProfRoomGlassCard
              data-tour="prof-room-calendar"
              className="overflow-hidden"
              bodyClassName="overflow-hidden rounded-[1.5rem]"
            >
              <div
                className="pointer-events-none absolute -right-10 -top-12 z-0 h-44 w-44 rounded-full bg-[color:var(--dash-soft)]/80 blur-3xl"
                aria-hidden
              />
              <div className={`relative z-[1] grid border-b border-[color:var(--dash-border)] ${dash.bgSoft50} ${isMobile ? "grid-cols-2" : "grid-cols-6"}`}>
                <div className={`rounded-tl-[1.5rem] p-4 text-center text-[11px] font-semibold uppercase tracking-wide ${dash.textMid}`}>
                  Heure
                </div>
                {displayDays.map((day, i) => (
                  <div
                    key={`${day.label}-${i}`}
                    className={`border-l border-[color:var(--dash-border)] p-4 text-center ${
                      i === displayDays.length - 1 ? "rounded-tr-[1.5rem]" : ""
                    } ${
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
                      const dateStr = localYmd(date);
                      const hourPrefix = `${dateStr}T${h.toString().padStart(2, "0")}`;
                      const res = reservations.find((r) =>
                        reservationMatchesHourPrefix(r, selectedRoom, hourPrefix),
                      );
                      const edt = !res ? edtCellAt(dateStr, h) : undefined;
                      const isOwn = res?.userId === user.id;
                      const canModify = isAdmin || isOwn;
                      const colorValue = res ? SUBJECT_COLORS[res.subject] || "bg-slate-600 text-white" : "";
                      const tilePresentation = res ? getReservationTilePresentation(colorValue) : null;
                      return (
                        <div
                          key={i}
                          onClick={() => handleCellClick(dateStr, h, res)}
                          onContextMenu={(e) => {
                            if (edt && !res) {
                              e.preventDefault();
                              return;
                            }
                            handleContextMenu(e, dateStr, h, res);
                          }}
                          className={`group relative cursor-pointer border-l border-[color:var(--dash-border)] p-1 transition-all sm:h-[120px] ${
                            !res && !edt ? "hover:bg-[color:var(--dash-soft)]/45" : ""
                          } ${edt ? "cursor-not-allowed" : ""}`}
                        >
                          {res ? (
                            <>
                              <div
                                className={`flex h-full w-full flex-col justify-between rounded-xl p-2 text-[11px] ${
                                  isOwn ? "ring-2 ring-[color:var(--dash-bright)]/70 ring-inset" : ""
                                }`}
                                style={tilePresentation?.style}
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
                          ) : edt ? (
                            <div className="flex h-full w-full flex-col justify-between rounded-xl border border-slate-300/80 bg-slate-100/90 p-2 text-[11px] text-slate-700">
                              <div>
                                <p className="truncate font-bold uppercase leading-none tracking-wide text-slate-800">
                                  {edt.subject}
                                </p>
                                <p className="mt-1 truncate text-[10px] font-semibold text-slate-600">
                                  {(edt.classes || []).join(", ") || "EDT"}
                                </p>
                              </div>
                              <div className="mt-1 flex items-end justify-between gap-1">
                                <span className="line-clamp-2 font-semibold uppercase leading-tight text-slate-500">
                                  {edt.teacherName}
                                </span>
                                <span className="shrink-0 rounded bg-slate-200 px-1 text-[9px] font-black uppercase text-slate-600">
                                  EDT {edt.weekType === "replacement" ? "R" : edt.weekType}
                                </span>
                              </div>
                            </div>
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
                  <ModuleButton
                    variant="danger"
                    onClick={handleDelete}
                    disabled={deleting}
                    className="w-full md:w-auto"
                  >
                    {deleting ? "Suppression…" : "🗑️ Supprimer ce créneau"}
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
                    {isEditing ? "Rattacher à une autre personne" : "Pour une autre personne"}
                  </label>
                ) : null}
              </div>
              {canBookForOthers && bookForOther ? (
                <div className="mb-5">
                  <FieldLabel>Personne concernée</FieldLabel>
                  <p className={`mb-2 text-xs ${dash.textMid}`}>
                    {isEditing
                      ? "Corrigez le rattachement : choisissez le collègue dans l’annuaire pour qu’il voie le créneau sur son tableau de bord."
                      : "Choisissez un collègue dans l’annuaire : la réservation lui sera rattachée et apparaîtra sur son tableau de bord."}
                  </p>
                  <ProfRoomBeneficiarySelect
                    key={
                      editingRes?.id
                        ? `edit-${editingRes.id}-${bookForOther ? "other" : "self"}`
                        : `new-${bookForOther ? "other" : "self"}`
                    }
                    value={beneficiary}
                    onChange={setBeneficiary}
                    matchFirstName={editingRes?.firstName || beneficiary?.firstName}
                    matchLastName={editingRes?.lastName || beneficiary?.lastName}
                  />
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
                          reservationMatchesHourPrefix(r, selectedRoom, hourPrefix) &&
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
        <ModulePageShell maxWidthClass="max-w-[1400px]">
          <p className={`text-sm ${dash.textMid}`}>Chargement des salles…</p>
        </ModulePageShell>
      }
    >
      <ProfRoomPageContent />
    </Suspense>
  );
}