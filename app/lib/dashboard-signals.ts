import {
  canViewCalendar,
  getRoleFlags,
  isAbsencePendingForManager,
  resolveAbsenceScope,
  type AbsenceRecord,
} from "@/app/lib/absences-types";
import { absencesToday } from "@/app/lib/dashboard-absences";
import type { DashboardPillarId } from "@/app/lib/dashboard-pillars";
import { tripsThisWeek, tripsToday, type TripIndexRow } from "@/app/lib/dashboard-trips";
import { moduleHref } from "@/app/lib/pillar-module-routes";
import { pickExactCurrentWeekSheet } from "@/app/lib/dashboard-week-sheet-active";
import type { WeekSheetData, WeekSheetEvent } from "@/app/lib/dashboard-week-sheet-types";
import { WEEK_DAYS, type WeekDayKey } from "@/app/lib/dashboard-week-sheet-types";
import { canAccessHseModule, getHseRoleFlags, type HseRecordLike } from "@/app/lib/demandes-hse-access";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { canSeeInternatRollCallSignal } from "@/app/lib/internat-rbac";
import { resolveDirectionEtab } from "@/app/lib/travels-direction-dashboard";
import { normalizeRequestEmail } from "@/app/lib/requests-board";

export type DashboardShortcutTone = "neutral" | "info" | "action" | "warn";

export type DashboardShortcut = {
  id: string;
  pillarId: DashboardPillarId;
  moduleId: string;
  href: string;
  label: string;
  rich?: boolean;
  badge?: string;
  detail?: string;
  tone?: DashboardShortcutTone;
  /** Visible uniquement sur les sous-dashboards piliers (pas la grille home). */
  pillarOnly?: boolean;
};

export type DashboardTodayNewsItem = {
  id: string;
  title: string;
  time?: string;
  location?: string;
};

/** Notification actionnable (à traiter / attribuée) pour le badge global. */
export type DashboardNotification = {
  id: string;
  label: string;
  count: number;
  href: string;
  detail: string;
};

export type DashboardSignals = {
  shortcuts: DashboardShortcut[];
  todayNews: DashboardTodayNewsItem[];
  hasCurrentWeek: boolean;
  notifications: DashboardNotification[];
};

export type DashboardSignalsInput = {
  roles: string[];
  userId: string;
  email: string;
  accessibleModuleIds: Set<string>;
  trips?: TripIndexRow[];
  absences?: AbsenceRecord[];
  reservations?: Array<{
    id: string;
    roomId: string;
    roomName?: string;
    startsAt: string;
    endsAt?: string;
    subject?: string;
    status?: string;
  }>;
  rooms?: Array<{ id: string; name: string }>;
  requestsBoard?: Array<{
    id: string;
    status: string;
    subject?: string;
    assignedTo?: {
      email?: string;
      claimedBy?: { email?: string; userId?: string | null } | null;
    };
  }>;
  photocopies?: Array<{
    id: string;
    status: string;
    etablissement?: string;
    createdBy?: { userId?: string };
  }>;
  hse?: Array<HseRecordLike & { id: string }>;
  stagesPendingSignatures?: number;
  internatRollCallStatus?: "validee" | "en_cours" | "non_demarre" | null;
  weekSheet?: WeekSheetData | null;
};

function weekDayFromDateKey(dateKey: string): WeekDayKey | null {
  const d = new Date(`${dateKey}T12:00:00`);
  const wd = new Intl.DateTimeFormat("en-US", {
    timeZone: "Europe/Paris",
    weekday: "short",
  }).format(d);
  const map: Record<string, WeekDayKey> = {
    Mon: "mon",
    Tue: "tue",
    Wed: "wed",
    Thu: "thu",
    Fri: "fri",
  };
  return map[wd] ?? null;
}

function formatEventTime(ev: WeekSheetEvent): string | undefined {
  if (!ev.startTime) return undefined;
  return ev.endTime ? `${ev.startTime} – ${ev.endTime}` : ev.startTime;
}

function isDirectionRole(roles: string[]): boolean {
  const f = getRoleFlags(roles);
  return f.isDirectionEcole || f.isDirectionCollege || f.isDirectionLycee;
}

function isCompta(roles: string[]): boolean {
  return getRoleFlags(roles).isCompta;
}

function canSeeTodayTripHighlight(roles: string[]): boolean {
  if (isDirectionRole(roles) || isCompta(roles)) return false;
  return (
    hasRole(roles, "administratif") ||
    hasRole(roles, "education") ||
    hasRole(roles, "professeur")
  );
}

function photocopieEtabForDirection(roles: string[]): string | null {
  const f = getHseRoleFlags(roles);
  if (f.isDirectionEcole) return "École";
  if (f.isDirectionCollege) return "Collège";
  if (f.isDirectionLycee) return "Lycée";
  return null;
}

function slotTimeLabel(startsAt: string): string {
  const t = startsAt.slice(11, 16);
  return t || startsAt;
}

export function buildTodayNewsFromWeekSheet(
  weekSheet: WeekSheetData | null | undefined,
): { items: DashboardTodayNewsItem[]; hasCurrentWeek: boolean } {
  if (!weekSheet) return { items: [], hasCurrentWeek: false };
  const exact = pickExactCurrentWeekSheet(weekSheet);
  if (!exact) return { items: [], hasCurrentWeek: false };

  const todayKey = calendarDateKeyParis();
  const dayKey = weekDayFromDateKey(todayKey);
  if (!dayKey) return { items: [], hasCurrentWeek: true };

  const dayEvents = exact.events
    .filter((ev) => ev.day === dayKey)
    .sort((a, b) => (a.startTime || "").localeCompare(b.startTime || ""));

  return {
    hasCurrentWeek: true,
    items: dayEvents.map((ev) => ({
      id: ev.id,
      title: ev.title,
      time: formatEventTime(ev),
      location: ev.location,
    })),
  };
}

export function getDashboardSignals(input: DashboardSignalsInput): DashboardSignals {
  const {
    roles,
    userId,
    email,
    accessibleModuleIds,
    trips = [],
    absences = [],
    reservations = [],
    rooms = [],
    requestsBoard = [],
    photocopies = [],
    hse = [],
    stagesPendingSignatures = 0,
    internatRollCallStatus = null,
    weekSheet = null,
  } = input;

  const shortcuts: DashboardShortcut[] = [];
  const notifications: DashboardNotification[] = [];
  const has = (moduleId: string) => accessibleModuleIds.has(moduleId);
  const emailNorm = normalizeRequestEmail(email || "");

  const pushNotif = (n: DashboardNotification) => {
    if (n.count > 0) notifications.push(n);
  };

  // —— Élèves : Sorties ——
  if (has("travels")) {
    const travelsHome = moduleHref("travels");
    const todayTrips = tripsToday(trips);
    const weekTrips = tripsThisWeek(trips);

    if (canSeeTodayTripHighlight(roles) && todayTrips.length > 0) {
      const first = todayTrips[0]!;
      shortcuts.push({
        id: "travels-today",
        pillarId: "eleves",
        moduleId: "travels",
        href: `/travels/${first.id}`,
        label: first.data?.title || "Sortie scolaire",
        rich: true,
        badge: todayTrips.length > 1 ? `${todayTrips.length} sorties` : "Aujourd'hui",
        detail: todayTrips.length > 1 ? `+ ${todayTrips.length - 1} autre(s) aujourd'hui` : "En cours aujourd'hui",
        tone: "action",
      });
    } else if (isCompta(roles)) {
      const n = trips.filter((t) => t.status === "EN_ATTENTE_COMPTA").length;
      if (n > 0) {
        shortcuts.push({
          id: "travels-compta",
          pillarId: "eleves",
          moduleId: "travels",
          href: travelsHome,
          label: "Sorties scolaires",
          rich: true,
          badge: `${n} à traiter`,
          detail: n === 1 ? "1 séjour en attente compta" : `${n} séjours en attente compta`,
          tone: "warn",
        });
        pushNotif({
          id: "travels-compta",
          label: "Sorties scolaires",
          count: n,
          href: travelsHome,
          detail: n === 1 ? "1 séjour en attente compta" : `${n} séjours en attente compta`,
        });
      } else {
        shortcuts.push({
          id: "travels",
          pillarId: "eleves",
          moduleId: "travels",
          href: travelsHome,
          label: "Sorties scolaires",
        });
      }
    } else if (isDirectionRole(roles)) {
      const etab = resolveDirectionEtab(roles);
      const pending = trips.filter((t) => {
        if (t.status !== "EN_ATTENTE_DIR_INITIAL" && t.status !== "EN_ATTENTE_DIR_FINAL") return false;
        if (!etab) return true;
        return (t.data?.etablissement || "Groupe Scolaire") === etab;
      });
      if (pending.length > 0) {
        shortcuts.push({
          id: "travels-dir",
          pillarId: "eleves",
          moduleId: "travels",
          href: travelsHome,
          label: "Sorties scolaires",
          rich: true,
          badge: `${pending.length} à valider`,
          detail:
            pending.length === 1
              ? "1 séjour en attente de votre validation"
              : `${pending.length} séjours en attente de votre validation`,
          tone: "warn",
        });
        pushNotif({
          id: "travels-dir",
          label: "Sorties scolaires",
          count: pending.length,
          href: travelsHome,
          detail:
            pending.length === 1
              ? "1 séjour en attente de votre validation"
              : `${pending.length} séjours en attente de votre validation`,
        });
      } else {
        shortcuts.push({
          id: "travels",
          pillarId: "eleves",
          moduleId: "travels",
          href: travelsHome,
          label: "Sorties scolaires",
        });
      }
    } else {
      shortcuts.push({
        id: "travels",
        pillarId: "eleves",
        moduleId: "travels",
        href: travelsHome,
        label: "Sorties scolaires",
      });
    }

    // Aperçu semaine home (agrégé) + détail sous-dashboard (une ligne par sortie)
    if (weekTrips.length > 0) {
      const titles = weekTrips
        .slice(0, 3)
        .map((t) => t.data?.title || "Sortie")
        .join(" · ");
      shortcuts.push({
        id: "travels-week",
        pillarId: "eleves",
        moduleId: "travels",
        href: travelsHome,
        label: "Cette semaine",
        rich: true,
        badge: `${weekTrips.length} sortie${weekTrips.length > 1 ? "s" : ""}`,
        detail: titles + (weekTrips.length > 3 ? ` · +${weekTrips.length - 3}` : ""),
        tone: "info",
      });

      for (const t of weekTrips.slice(0, 6)) {
        const rawDate = t.data?.startDate || t.data?.date || "";
        const when = rawDate
          ? new Date(rawDate).toLocaleDateString("fr-FR", {
              weekday: "short",
              day: "numeric",
              month: "short",
            })
          : "À venir";
        const isToday = todayTrips.some((x) => x.id === t.id);
        shortcuts.push({
          id: `travels-up-${t.id}`,
          pillarId: "eleves",
          moduleId: "travels",
          href: `/travels/${t.id}`,
          label: t.data?.title || "Sortie",
          rich: true,
          badge: isToday ? "Aujourd'hui" : when,
          detail: t.data?.etablissement || undefined,
          tone: isToday ? "action" : "info",
          pillarOnly: true,
        });
      }
    } else if (
      !shortcuts.some((s) => s.moduleId === "travels" && s.rich)
    ) {
      shortcuts.push({
        id: "travels-empty",
        pillarId: "eleves",
        moduleId: "travels",
        href: travelsHome,
        label: "Aucune sortie à venir",
        rich: true,
        detail: "Pas de sortie prévue cette semaine",
        tone: "neutral",
        pillarOnly: true,
      });
    }
  }

  // —— Élèves : Internat ——
  if (has("internat")) {
    const internatHome = moduleHref("internat");
    const showAppelSignal =
      canSeeInternatRollCallSignal(roles) &&
      (internatRollCallStatus === "non_demarre" || internatRollCallStatus === "en_cours");

    if (showAppelSignal) {
      shortcuts.push({
        id: "internat-appel",
        pillarId: "eleves",
        moduleId: "internat",
        href: internatHome,
        label: "Appel du soir",
        rich: true,
        badge: internatRollCallStatus === "en_cours" ? "En cours" : "À faire",
        detail:
          internatRollCallStatus === "en_cours"
            ? "Appel du soir en cours"
            : "Appel du soir non démarré",
        tone: "action",
      });
      pushNotif({
        id: "internat-appel",
        label: "Appel du soir",
        count: 1,
        href: internatHome,
        detail:
          internatRollCallStatus === "en_cours"
            ? "Appel du soir en cours"
            : "Appel du soir non démarré",
      });
    } else {
      shortcuts.push({
        id: "internat",
        pillarId: "eleves",
        moduleId: "internat",
        href: internatHome,
        label: "Internat",
      });
      shortcuts.push({
        id: "internat-ok",
        pillarId: "eleves",
        moduleId: "internat",
        href: internatHome,
        label: "Appel du soir",
        rich: true,
        badge: internatRollCallStatus === "validee" ? "Validé" : "OK",
        detail:
          internatRollCallStatus === "validee"
            ? "Appel du soir déjà validé"
            : "Rien d'urgent côté appel pour le moment",
        tone: "neutral",
        pillarOnly: true,
      });
    }
  }

  // —— Élèves : Stages ——
  if (has("stages")) {
    const stagesHome = moduleHref("stages");
    if (stagesPendingSignatures > 0) {
      shortcuts.push({
        id: "stages-sign",
        pillarId: "eleves",
        moduleId: "stages",
        href: stagesHome,
        label: "Stages & conventions",
        rich: true,
        badge: `${stagesPendingSignatures} signature${stagesPendingSignatures > 1 ? "s" : ""}`,
        detail:
          stagesPendingSignatures === 1
            ? "1 signature stage à faire"
            : `${stagesPendingSignatures} signatures stages à faire`,
        tone: "warn",
      });
      pushNotif({
        id: "stages-sign",
        label: "Stages & conventions",
        count: stagesPendingSignatures,
        href: stagesHome,
        detail:
          stagesPendingSignatures === 1
            ? "1 signature stage à faire"
            : `${stagesPendingSignatures} signatures stages à faire`,
      });
    } else {
      shortcuts.push({
        id: "stages",
        pillarId: "eleves",
        moduleId: "stages",
        href: stagesHome,
        label: "Stages & conventions",
      });
      shortcuts.push({
        id: "stages-ok",
        pillarId: "eleves",
        moduleId: "stages",
        href: stagesHome,
        label: "Signatures",
        rich: true,
        detail: "Aucune signature en attente",
        tone: "neutral",
        pillarOnly: true,
      });
    }
  }

  // —— Élèves : OCR / certificats (stables) ——
  if (has("agent-ia-ocr")) {
    shortcuts.push({
      id: "ocr",
      pillarId: "eleves",
      moduleId: "agent-ia-ocr",
      href: moduleHref("agent-ia-ocr"),
      label: "Ajout de documents IA",
    });
  }
  if (has("certificates")) {
    shortcuts.push({
      id: "certificates",
      pillarId: "eleves",
      moduleId: "certificates",
      href: moduleHref("certificates"),
      label: "Parcours & certificats",
    });
  }

  // —— RH : Absences ——
  if (has("rh")) {
    shortcuts.push({
      id: "rh-mon-espace",
      pillarId: "rh",
      moduleId: "rh",
      href: "/rh?tab=dashboard",
      label: "Mon espace",
    });

    if (canViewCalendar(roles)) {
      const flags = getRoleFlags(roles);
      let scoped = absences;
      let labelSingular = "personne absente";
      let labelPlural = "personnes absentes";

      if (flags.isCompta && !flags.isAdministratif && !flags.isEducation && !isDirectionRole(roles)) {
        scoped = absences.filter((a) => resolveAbsenceScope(a) === "ogec");
        labelSingular = "personnel (OGEC) absent";
        labelPlural = "personnels (OGEC) absents";
      } else if ((flags.isEducation || flags.isAdministratif) && !isDirectionRole(roles)) {
        scoped = absences.filter((a) => resolveAbsenceScope(a) === "professeur");
        labelSingular = "prof absent";
        labelPlural = "profs absents";
      }

      const count = absencesToday(scoped).length;
      const pendingManager = absences.filter((a) => isAbsencePendingForManager(a, userId, roles));

      if (count > 0) {
        shortcuts.push({
          id: "absences-today",
          pillarId: "rh",
          moduleId: "absences",
          href: "/rh?tab=absences",
          label: "Absences",
          rich: true,
          badge: String(count),
          detail: count === 1 ? `1 ${labelSingular} aujourd'hui` : `${count} ${labelPlural} aujourd'hui`,
          tone: "info",
        });
      } else if (pendingManager.length === 0) {
        shortcuts.push({
          id: "absences",
          pillarId: "rh",
          moduleId: "absences",
          href: "/rh?tab=absences&view=se-declarer",
          label: "Absences",
        });
      }

      if (pendingManager.length > 0) {
        shortcuts.push({
          id: "absences-pending",
          pillarId: "rh",
          moduleId: "absences",
          href: "/rh?tab=absences&view=a-traiter",
          label: "Absences à traiter",
          rich: true,
          badge: `${pendingManager.length} à traiter`,
          detail:
            pendingManager.length === 1
              ? "1 absence en attente de votre décision"
              : `${pendingManager.length} absences en attente de votre décision`,
          tone: "warn",
        });
        pushNotif({
          id: "absences-pending",
          label: "Absences à traiter",
          count: pendingManager.length,
          href: "/rh?tab=absences&view=a-traiter",
          detail:
            pendingManager.length === 1
              ? "1 absence en attente de votre décision"
              : `${pendingManager.length} absences en attente de votre décision`,
        });
      }
    } else {
      shortcuts.push({
        id: "absences",
        pillarId: "rh",
        moduleId: "absences",
        href: "/rh?tab=absences&view=se-declarer",
        label: "Mes absences",
      });
    }

    // HSE
    if (canAccessHseModule(roles)) {
      const hseFlags = getHseRoleFlags(roles);
      const isDir =
        hseFlags.isDirectionEcole || hseFlags.isDirectionCollege || hseFlags.isDirectionLycee;
      if (isDir) {
        const pending = hse.filter((h) => h.status === "EN_ATTENTE").length;
        if (pending > 0) {
          shortcuts.push({
            id: "hse-pending",
            pillarId: "rh",
            moduleId: "demandes-hse",
            href: "/rh?tab=hse",
            label: "Demandes HSE",
            rich: true,
            badge: `${pending} à traiter`,
            detail:
              pending === 1
                ? "1 demande HSE à traiter"
                : `${pending} demandes HSE à traiter`,
            tone: "warn",
          });
          pushNotif({
            id: "hse-pending",
            label: "Demandes HSE",
            count: pending,
            href: "/rh?tab=hse",
            detail:
              pending === 1
                ? "1 demande HSE à traiter"
                : `${pending} demandes HSE à traiter`,
          });
        } else {
          shortcuts.push({
            id: "hse",
            pillarId: "rh",
            moduleId: "demandes-hse",
            href: "/rh?tab=hse",
            label: "Demandes HSE",
          });
        }
      } else {
        shortcuts.push({
          id: "hse",
          pillarId: "rh",
          moduleId: "demandes-hse",
          href: "/rh?tab=hse",
          label: "Mes demandes HSE",
        });
      }
    }

    // Mon espace reste le point d’entrée personnel ; le pilier ouvre aussi /rh.
  }

  // —— Services : Salles ——
  if (has("prof-room")) {
    const roomsHome = moduleHref("prof-room");
    const todayKey = calendarDateKeyParis();
    const roomNameById = new Map(rooms.map((r) => [r.id, r.name]));
    const todayRes = reservations
      .filter((r) => r.status !== "CANCELLED" && r.startsAt.startsWith(todayKey))
      .sort((a, b) => a.startsAt.localeCompare(b.startsAt));

    if (todayRes.length === 1) {
      const r = todayRes[0]!;
      const name = r.roomName || roomNameById.get(r.roomId) || "Salle";
      shortcuts.push({
        id: "rooms-today-one",
        pillarId: "services",
        moduleId: "prof-room",
        href: roomsHome,
        label: name,
        rich: true,
        badge: slotTimeLabel(r.startsAt),
        detail: r.subject || "Réservée aujourd'hui",
        tone: "info",
      });
    } else if (todayRes.length > 1) {
      const next = todayRes[0]!;
      const name = next.roomName || roomNameById.get(next.roomId) || "Salle";
      shortcuts.push({
        id: "rooms-today-many",
        pillarId: "services",
        moduleId: "prof-room",
        href: roomsHome,
        label: "Réservation de salle",
        rich: true,
        badge: `${todayRes.length} salles`,
        detail: `Prochaine : ${name} · ${slotTimeLabel(next.startsAt)}`,
        tone: "info",
      });
    } else {
      shortcuts.push({
        id: "rooms-empty",
        pillarId: "services",
        moduleId: "prof-room",
        href: roomsHome,
        label: "Aucune salle réservée aujourd'hui",
      });
    }
  }

  // —— Services : Demandes ——
  if (has("requests-staff")) {
    const requestsHome = moduleHref("requests-staff");
    const claimedMine = requestsBoard.filter((r) => {
      if (r.status === "TERMINEE") return false;
      const c = r.assignedTo?.claimedBy;
      if (!c) return false;
      if (c.userId && c.userId === userId) return true;
      return emailNorm && normalizeRequestEmail(c.email || "") === emailNorm;
    });
    const unassigned = requestsBoard.filter((r) => !r.assignedTo?.claimedBy && r.status !== "TERMINEE");

    if (claimedMine.length > 0) {
      shortcuts.push({
        id: "requests-claimed",
        pillarId: "services",
        moduleId: "requests-staff",
        href: requestsHome,
        label: "Demandes",
        rich: true,
        badge: claimedMine.length === 1 ? "1 attribuée" : `${claimedMine.length} attribuées`,
        detail:
          claimedMine.length === 1
            ? "Une demande vous a été attribuée"
            : `${claimedMine.length} demandes vous ont été attribuées`,
        tone: "action",
      });
      pushNotif({
        id: "requests-claimed",
        label: "Demandes attribuées",
        count: claimedMine.length,
        href: requestsHome,
        detail:
          claimedMine.length === 1
            ? "1 demande vous a été attribuée"
            : `${claimedMine.length} demandes vous ont été attribuées`,
      });
    }
    if (unassigned.length > 0) {
      shortcuts.push({
        id: "requests-pool",
        pillarId: "services",
        moduleId: "requests-staff",
        href: requestsHome,
        label: "File demandes",
        rich: true,
        badge: `${unassigned.length} non assignée${unassigned.length > 1 ? "s" : ""}`,
        detail:
          unassigned.length === 1
            ? "1 demande non assignée dans votre file"
            : `${unassigned.length} demandes non assignées dans votre file`,
        tone: "warn",
      });
      pushNotif({
        id: "requests-pool",
        label: "File demandes",
        count: unassigned.length,
        href: requestsHome,
        detail:
          unassigned.length === 1
            ? "1 demande non assignée dans votre file"
            : `${unassigned.length} demandes non assignées dans votre file`,
      });
    }
    if (claimedMine.length === 0 && unassigned.length === 0) {
      shortcuts.push({
        id: "requests-new",
        pillarId: "services",
        moduleId: "requests-staff",
        href: "/faire-une-demande",
        label: "Faire une demande",
      });
      shortcuts.push({
        id: "requests-mine",
        pillarId: "services",
        moduleId: "requests-staff",
        href: "/mes-demandes",
        label: "Voir mes demandes",
      });
    }
  }

  // —— Services : Photocopies ——
  if (has("photocopies-couleur")) {
    const photoHome = moduleHref("photocopies-couleur");
    const etab = photocopieEtabForDirection(roles);
    if (etab) {
      const pending = photocopies.filter(
        (p) => p.status === "EN_ATTENTE" && p.etablissement === etab,
      ).length;
      if (pending > 0) {
        shortcuts.push({
          id: "photo-dir",
          pillarId: "services",
          moduleId: "photocopies-couleur",
          href: photoHome,
          label: "Photocopies couleur",
          rich: true,
          badge: `${pending} à traiter`,
          detail:
            pending === 1
              ? "1 photocopie à traiter"
              : `${pending} photocopies à traiter`,
          tone: "warn",
        });
        pushNotif({
          id: "photo-dir",
          label: "Photocopies couleur",
          count: pending,
          href: photoHome,
          detail:
            pending === 1
              ? "1 photocopie à traiter"
              : `${pending} photocopies à traiter`,
        });
      } else {
        shortcuts.push({
          id: "photo",
          pillarId: "services",
          moduleId: "photocopies-couleur",
          href: photoHome,
          label: "Photocopies couleur",
        });
      }
    } else {
      shortcuts.push({
        id: "photo",
        pillarId: "services",
        moduleId: "photocopies-couleur",
        href: photoHome,
        label: "Photocopies couleur",
      });
    }
  }

  // —— Services : stables ——
  const stableServices: Array<{ moduleId: string; label: string }> = [
    { moduleId: "documents", label: "Cloud personnel" },
    { moduleId: "toolbox", label: "Boîte à outils" },
    { moduleId: "covoiturage", label: "Covoiturage" },
    { moduleId: "channels", label: "Salons" },
    { moduleId: "assistance", label: "Assistance" },
  ];
  for (const s of stableServices) {
    if (!has(s.moduleId)) continue;
    shortcuts.push({
      id: s.moduleId,
      pillarId: "services",
      moduleId: s.moduleId,
      href: moduleHref(s.moduleId),
      label: s.label,
    });
  }

  // —— Établissement : stables ——
  const stableEtab: Array<{ moduleId: string; label: string }> = [
    { moduleId: "organigramme", label: "Organigramme" },
    { moduleId: "conformite-rgpd", label: "Conformité RGPD" },
    { moduleId: "chatbot-knowledge", label: "Brain AI" },
    { moduleId: "domain-planning", label: "Enseignements transversaux" },
  ];
  for (const s of stableEtab) {
    if (!has(s.moduleId)) continue;
    shortcuts.push({
      id: s.moduleId,
      pillarId: "etablissement",
      moduleId: s.moduleId,
      href: moduleHref(s.moduleId),
      label: s.label,
    });
  }

  const news = buildTodayNewsFromWeekSheet(weekSheet);

  return {
    shortcuts,
    todayNews: news.items,
    hasCurrentWeek: news.hasCurrentWeek,
    notifications,
  };
}

export function dayLabelFr(dayKey: WeekDayKey): string {
  return WEEK_DAYS.find((d) => d.key === dayKey)?.label ?? dayKey;
}
