import {
  canViewCalendar,
  getRoleFlags,
  isAbsencePendingForManager,
  resolveAbsenceScope,
  type AbsenceRecord,
} from "@/app/lib/absences-types";
import { absencesToday } from "@/app/lib/dashboard-absences";
import { moduleIdToPillarId, type DashboardPillarId } from "@/app/lib/dashboard-pillars";
import { tripsThisWeek, tripsToday, type TripIndexRow } from "@/app/lib/dashboard-trips";
import { moduleHref } from "@/app/lib/pillar-module-routes";
import { pickExactCurrentWeekSheet } from "@/app/lib/dashboard-week-sheet-active";
import type { WeekSheetData, WeekSheetEvent } from "@/app/lib/dashboard-week-sheet-types";
import { WEEK_DAYS, type WeekDayKey } from "@/app/lib/dashboard-week-sheet-types";
import { canAccessHseModule, canCreateHseDemand, getHseRoleFlags, type HseRecordLike } from "@/app/lib/demandes-hse-access";
import { directionRolesMatchEstablishmentRef, isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import type { Establishment } from "@/app/lib/app-config-schemas";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { isProfesseurScopedDossierViewer } from "@/app/lib/eleve-dossier-scope";
import { canSeeInternatRollCallSignal } from "@/app/lib/internat-rbac";
import { resolveDirectionEtab } from "@/app/lib/travels-direction-dashboard";
import { normalizeRequestEmail } from "@/app/lib/requests-board";
import { isOwnRoomReservation } from "@/app/lib/prof-room-reservation-ownership";
import { subjectColorToHex } from "@/app/lib/prof-room-subject-colors";

export type DashboardShortcutTone = "neutral" | "info" | "action" | "warn";

/** Slide d’un carrousel (salles en cours, sorties du jour, …). */
export type DashboardShortcutSlide = {
  id: string;
  label: string;
  detail?: string;
  badge?: string;
  /** Couleur d’accent (hex). */
  colorHex?: string;
  /** Lien spécifique à la slide (sinon href du shortcut). */
  href?: string;
};

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
  /** Emoji de tuile (sinon MODULE_EMOJI[moduleId]). */
  emoji?: string;
  /** Visible uniquement sur les sous-dashboards piliers (pas la grille home). */
  pillarOnly?: boolean;
  /** Si présent, la tuile fait défiler ces slides (~3 s). */
  slides?: DashboardShortcutSlide[];
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
  /** Module intranet pour pastille rouge sur la tuile / hub. */
  moduleId: string;
  label: string;
  count: number;
  href: string;
  detail: string;
};

/** Compteur rouge sur une tuile : match exact sur l’id, sinon orphelins du module (ex. dossiers partagés). */
export function notificationCountForShortcut(
  item: { id: string; moduleId: string },
  notifications: DashboardNotification[],
): number {
  const related = notifications.filter((n) => n.moduleId === item.moduleId && n.count > 0);
  if (related.length === 0) return 0;

  const exact = related.filter((n) => n.id === item.id);
  if (exact.length > 0) return exact.reduce((sum, n) => sum + n.count, 0);

  if (item.id !== item.moduleId) return 0;

  return related
    .filter((n) => n.id === item.moduleId || n.id.startsWith(`${item.moduleId}-`))
    .reduce((sum, n) => sum + n.count, 0);
}

/** Total des notifications d’un module (hubs piliers). */
export function notificationCountForModule(
  moduleId: string,
  notifications: DashboardNotification[],
): number {
  return notifications
    .filter((n) => n.moduleId === moduleId && n.count > 0)
    .reduce((sum, n) => sum + n.count, 0);
}

export type DashboardSignals = {
  shortcuts: DashboardShortcut[];
  todayNews: DashboardTodayNewsItem[];
  hasCurrentWeek: boolean;
  notifications: DashboardNotification[];
  /** Année scolaire courante de l’établissement (si connue). */
  anneeScolaireLabel?: string | null;
};

type DashboardSignalsInput = {
  roles: string[];
  userId: string;
  email: string;
  firstName?: string;
  lastName?: string;
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
    className?: string;
    status?: string;
    userId?: string;
    email?: string;
    bookedForOther?: boolean;
    bookedByUserId?: string;
    firstName?: string;
    lastName?: string;
    bookedByFirstName?: string;
    bookedByLastName?: string;
  }>;
  rooms?: Array<{ id: string; name: string }>;
  /** Couleurs matières (valeur Tailwind ou hex) pour le carrousel salles. */
  roomSubjectColors?: Record<string, string>;
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
  /** true si l’utilisateur a déjà soumis le pulse RH du jour. */
  moodPulseSubmittedToday?: boolean;
  /** Activité en cours d’après le planning RH. */
  planningNow?: {
    title: string;
    detail: string;
    start: string;
    end: string;
  } | null;
  establishments?: Establishment[];
  /** Dossiers cloud partagés invitant l'utilisateur, non encore ouverts. */
  unseenSharedFolders?: Array<{ id: string; name: string }>;
  /** Absences élèves à traiter (CPE). */
  vsAbsencesATraiter?: number;
  /** Motifs famille en attente de validation CPE. */
  vsAbsencesJustifFamille?: number;
  /** Créneaux du jour sans appel clôturé. */
  vsAppelsManquants?: number;
  /** Sanctions actives du jour (direction / CPE). */
  vsSanctionsAujourdhui?: number;
  /** Entrées carnet visibles famille non encore signées. */
  vsCarnetNonSignees?: number;
  /** Factures émises en retard de paiement. */
  facturesEnRetard?: number;
  /** Libellé année scolaire courante (ex. 2025-2026). */
  anneeScolaireLabel?: string | null;
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
  return isAnyDirectionRole(roles) || getRoleFlags(roles).isDirection;
}

function isCompta(roles: string[]): boolean {
  return getRoleFlags(roles).isCompta;
}

function canSeeTodayTripHighlight(roles: string[]): boolean {
  if (isDirectionRole(roles) || isCompta(roles)) return false;
  return (
    hasRole(roles, "administratif") ||
    hasRole(roles, "surveillant") ||
    hasRole(roles, "cpe") ||
    hasRole(roles, "professeur")
  );
}

function photocopiePendingForDirection(
  roles: string[],
  photocopies: Array<{ status: string; etablissement?: string }>,
  establishments: Establishment[],
): number {
  return photocopies.filter(
    (p) =>
      p.status === "EN_ATTENTE" &&
      directionRolesMatchEstablishmentRef(roles, p.etablissement, establishments),
  ).length;
}

function slotTimeLabel(startsAt: string): string {
  const t = startsAt.slice(11, 16);
  return t ? t.replace(":", "h") : startsAt;
}

/** Horodatage local Europe/Paris `YYYY-MM-DDTHH:mm:ss` (aligné sur les créneaux salles). */
function parisNowLocalIso(d: Date = new Date()): string {
  const parts = Object.fromEntries(
    new Intl.DateTimeFormat("en-GB", {
      timeZone: "Europe/Paris",
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    })
      .formatToParts(d)
      .filter((p) => p.type !== "literal")
      .map((p) => [p.type, p.value]),
  ) as Record<string, string>;
  const hour = parts.hour === "24" ? "00" : parts.hour;
  return `${parts.year}-${parts.month}-${parts.day}T${hour}:${parts.minute}:${parts.second}`;
}

function reservationComparable(iso: string): string {
  return String(iso || "").substring(0, 19);
}

function reservationEndsAt(r: { startsAt: string; endsAt?: string }): string {
  if (r.endsAt) return reservationComparable(r.endsAt);
  const start = reservationComparable(r.startsAt);
  // Créneaux module = 1 h (ex. 08h30 → 09h30), sans passer par Date (TZ serveur).
  const m = start.match(/^(\d{4}-\d{2}-\d{2})T(\d{2}):(\d{2}):(\d{2})/);
  if (!m) return start;
  const hour = String(Math.min(23, parseInt(m[2]!, 10) + 1)).padStart(2, "0");
  return `${m[1]}T${hour}:${m[3]}:${m[4]}`;
}

function isReservationLiveNow(
  r: { startsAt: string; endsAt?: string; status?: string },
  nowLocal: string,
): boolean {
  if (r.status === "CANCELLED") return false;
  const start = reservationComparable(r.startsAt);
  const end = reservationEndsAt(r);
  return start <= nowLocal && nowLocal < end;
}

function buildTodayNewsFromWeekSheet(
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
    firstName,
    lastName,
    accessibleModuleIds,
    trips = [],
    absences = [],
    reservations = [],
    rooms = [],
    roomSubjectColors = {},
    requestsBoard = [],
    photocopies = [],
    hse = [],
    stagesPendingSignatures = 0,
    internatRollCallStatus = null,
    weekSheet = null,
    establishments = [],
    unseenSharedFolders = [],
    vsAbsencesATraiter = 0,
    vsAbsencesJustifFamille = 0,
    vsAppelsManquants = 0,
    vsSanctionsAujourdhui = 0,
    vsCarnetNonSignees = 0,
    facturesEnRetard = 0,
    anneeScolaireLabel = null,
  } = input;

  const shortcuts: DashboardShortcut[] = [];
  const notifications: DashboardNotification[] = [];
  const has = (moduleId: string) => accessibleModuleIds.has(moduleId);
  const emailNorm = normalizeRequestEmail(email || "");
  const viewerNames = { firstName, lastName };

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
      const travelColors = ["#0284c7", "#0369a1", "#0ea5e9", "#1d4ed8", "#075985"];
      shortcuts.push({
        id: "travels-today",
        pillarId: "vie_scolaire",
        moduleId: "travels",
        href: todayTrips.length === 1 ? `/travels/${first.id}` : travelsHome,
        label: first.data?.title || "Sortie scolaire",
        rich: true,
        badge: todayTrips.length > 1 ? `${todayTrips.length} sorties` : "Aujourd'hui",
        detail:
          todayTrips.length > 1
            ? `${todayTrips.length} sorties aujourd'hui`
            : first.data?.etablissement || "En cours aujourd'hui",
        tone: "action",
        slides:
          todayTrips.length > 1
            ? todayTrips.map((t, i) => ({
                id: t.id,
                label: t.data?.title || "Sortie scolaire",
                detail: t.data?.etablissement || undefined,
                badge: "Aujourd'hui",
                colorHex: travelColors[i % travelColors.length],
                href: `/travels/${t.id}`,
              }))
            : undefined,
      });
    } else if (isCompta(roles)) {
      const n = trips.filter((t) => t.status === "EN_ATTENTE_COMPTA").length;
      if (n > 0) {
        shortcuts.push({
          id: "travels-compta",
          pillarId: "vie_scolaire",
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
          moduleId: "travels",
          label: "Sorties scolaires",
          count: n,
          href: travelsHome,
          detail: n === 1 ? "1 séjour en attente compta" : `${n} séjours en attente compta`,
        });
      } else {
        shortcuts.push({
          id: "travels",
          pillarId: "vie_scolaire",
          moduleId: "travels",
          href: travelsHome,
          label: "Sorties scolaires",
        });
      }
    } else if (isDirectionRole(roles)) {
      const etab = resolveDirectionEtab(roles, establishments);
      const pending = trips.filter((t) => {
        if (t.status !== "EN_ATTENTE_DIR_INITIAL" && t.status !== "EN_ATTENTE_DIR_FINAL") return false;
        if (!etab) return true;
        return (t.data?.etablissement || "Groupe Scolaire") === etab;
      });
      if (pending.length > 0) {
        shortcuts.push({
          id: "travels-dir",
          pillarId: "vie_scolaire",
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
          moduleId: "travels",
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
          pillarId: "vie_scolaire",
          moduleId: "travels",
          href: travelsHome,
          label: "Sorties scolaires",
        });
      }
    } else {
      shortcuts.push({
        id: "travels",
        pillarId: "vie_scolaire",
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
        pillarId: "vie_scolaire",
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
          pillarId: "vie_scolaire",
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
        pillarId: "vie_scolaire",
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
    shortcuts.push({
      id: "internat",
      pillarId: "vie_scolaire",
      moduleId: "internat",
      href: internatHome,
      label: "Internat",
    });

    if (canSeeInternatRollCallSignal(roles)) {
      const showAppelSignal =
        internatRollCallStatus === "non_demarre" || internatRollCallStatus === "en_cours";

      if (showAppelSignal) {
        shortcuts.push({
          id: "internat-appel",
          pillarId: "vie_scolaire",
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
          moduleId: "internat",
          label: "Appel du soir",
          count: 1,
          href: internatHome,
          detail:
            internatRollCallStatus === "en_cours"
              ? "Appel du soir en cours"
              : "Appel du soir non démarré",
        });
      } else if (internatRollCallStatus === "validee") {
        shortcuts.push({
          id: "internat-ok",
          pillarId: "vie_scolaire",
          moduleId: "internat",
          href: internatHome,
          label: "Appel du soir",
          rich: true,
          badge: "Validé",
          detail: "Appel du soir déjà validé",
          tone: "neutral",
          pillarOnly: true,
        });
      }
    }
  }

  // —— Élèves : Stages ——
  if (has("stages")) {
    const stagesHome = moduleHref("stages");
    if (stagesPendingSignatures > 0) {
      shortcuts.push({
        id: "stages-sign",
        pillarId: "vie_scolaire",
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
        moduleId: "stages",
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
        pillarId: "vie_scolaire",
        moduleId: "stages",
        href: stagesHome,
        label: "Stages & conventions",
      });
      shortcuts.push({
        id: "stages-ok",
        pillarId: "vie_scolaire",
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

  // —— Services : OCR documents ——
  if (has("agent-ia-ocr")) {
    shortcuts.push({
      id: "ocr",
      pillarId: "administratif",
      moduleId: "agent-ia-ocr",
      href: moduleHref("agent-ia-ocr"),
      label: "Ajout de documents IA",
    });
  }

  // —— Élèves : certificats (stables) ——
  if (has("certificates")) {
    shortcuts.push({
      id: "certificates",
      pillarId: "vie_scolaire",
      moduleId: "certificates",
      href: moduleHref("certificates"),
      label: "Parcours & certificats",
    });
  }

  // —— RH : Absences ——
  if (has("rh") || has("mon-planning")) {
    if (has("mon-planning")) {
      shortcuts.push({
        id: "mon-planning",
        pillarId: "compta_rh",
        moduleId: "mon-planning",
        href: "/mon-planning",
        label: "Mon planning",
        rich: true,
        detail: "Semaine type · absences · dossier RH",
        tone: "info",
      });
    }

    if (input.planningNow) {
      shortcuts.push({
        id: "planning-now",
        pillarId: "compta_rh",
        moduleId: "mon-planning",
        href: "/mon-planning",
        label: input.planningNow.title,
        rich: true,
        detail: input.planningNow.detail,
        badge: `${input.planningNow.start}–${input.planningNow.end}`,
        tone: "info",
      });
    }

    if (has("rh")) {
      shortcuts.push({
        id: "rh-mon-espace",
        pillarId: "compta_rh",
        moduleId: "rh",
        href: "/rh/moi",
        label: "Mon dossier RH",
        rich: true,
        detail: "Documents personnels · absences",
        tone: "info",
      });
      shortcuts.push({
        id: "rh-demande-absence",
        pillarId: "compta_rh",
        moduleId: "rh",
        href: "/rh?tab=dashboard&section=absences#nouvelle-absence",
        label: "Demander une absence",
        rich: true,
        detail: "Autorisation d’absence (self-service)",
        tone: "action",
      });

      if (canCreateHseDemand(roles)) {
        shortcuts.push({
          id: "rh-demande-hse",
          pillarId: "compta_rh",
          moduleId: "demandes-hse",
          href: "/rh?tab=dashboard&section=hse",
          label: "Faire une demande de HSE",
          rich: true,
          detail: "Heures supplémentaires exceptionnelles",
          tone: "action",
        });
      }
    }

    if (has("rh") && (canViewCalendar(roles) || isDirectionRole(roles))) {
      const flags = getRoleFlags(roles);
      const dirCtx = { establishments, userId };
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

      const count = canViewCalendar(roles) ? absencesToday(scoped).length : 0;
      const pendingManager = absences.filter((a) =>
        isAbsencePendingForManager(a, userId, roles, dirCtx),
      );

      if (pendingManager.length > 0) {
        shortcuts.push({
          id: "absences-pending",
          pillarId: "compta_rh",
          moduleId: "absences",
          href: "/rh?tab=dashboard&section=absences&view=a-traiter",
          label: "Absences à traiter",
          rich: true,
          badge: `${pendingManager.length} à traiter`,
          detail:
            pendingManager.length === 1
              ? "1 demande d'autorisation d'absence en attente"
              : `${pendingManager.length} demandes d'autorisation d'absence en attente`,
          tone: "warn",
        });
        pushNotif({
          id: "absences-pending",
          moduleId: "absences",
          label: "Absences à traiter",
          count: pendingManager.length,
          href: "/rh?tab=dashboard&section=absences&view=a-traiter",
          detail:
            pendingManager.length === 1
              ? "1 demande d'autorisation d'absence en attente de votre décision"
              : `${pendingManager.length} demandes d'autorisation d'absence en attente de votre décision`,
        });
      }

      if (count > 0) {
        shortcuts.push({
          id: "absences-today",
          pillarId: "compta_rh",
          moduleId: "absences",
          href: "/rh?tab=pilotage&section=overview",
          label: "Absences",
          rich: true,
          badge: String(count),
          detail: count === 1 ? `1 ${labelSingular} aujourd'hui` : `${count} ${labelPlural} aujourd'hui`,
          tone: "info",
        });
      } else if (pendingManager.length === 0) {
        shortcuts.push({
          id: "absences",
          pillarId: "compta_rh",
          moduleId: "absences",
          href: isDirectionRole(roles)
            ? "/rh?tab=pilotage&section=overview"
            : "/rh?tab=dashboard&section=absences",
          label: "Absences",
        });
      }
    } else if (has("rh")) {
      shortcuts.push({
        id: "absences",
        pillarId: "compta_rh",
        moduleId: "absences",
        href: "/rh?tab=dashboard&section=absences",
        label: "Mes absences",
      });
    }

    // HSE
    if (has("rh") && canAccessHseModule(roles)) {
      const hseFlags = getHseRoleFlags(roles);
      const isDir = hseFlags.isDirection;
      if (isDir) {
        const pending = hse.filter((h) => h.status === "EN_ATTENTE").length;
        if (pending > 0) {
          shortcuts.push({
            id: "hse-pending",
            pillarId: "compta_rh",
            moduleId: "demandes-hse",
            href: "/rh?tab=dashboard&section=hse",
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
            moduleId: "demandes-hse",
            label: "Demandes HSE",
            count: pending,
            href: "/rh?tab=dashboard&section=hse",
            detail:
              pending === 1
                ? "1 demande HSE à traiter"
                : `${pending} demandes HSE à traiter`,
          });
        } else {
          shortcuts.push({
            id: "hse",
            pillarId: "compta_rh",
            moduleId: "demandes-hse",
            href: "/rh?tab=dashboard&section=hse",
            label: "Demandes HSE",
          });
        }
      } else {
        shortcuts.push({
          id: "hse",
          pillarId: "compta_rh",
          moduleId: "demandes-hse",
          href: "/rh?tab=dashboard&section=hse",
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
    const nowLocal = parisNowLocalIso();
    const roomNameById = new Map(rooms.map((r) => [r.id, r.name]));
    const todayRes = reservations
      .filter((r) => {
        const startsAt = typeof r.startsAt === "string" ? r.startsAt : "";
        return (
          Boolean(startsAt) &&
          r.status !== "CANCELLED" &&
          startsAt.startsWith(todayKey) &&
          isOwnRoomReservation(r, userId, emailNorm, viewerNames)
        );
      })
      .sort((a, b) => String(a.startsAt || "").localeCompare(String(b.startsAt || "")));
    // Masquer les créneaux déjà terminés (ex. 8h30 une fois 10h30 passée).
    const activeToday = todayRes.filter((r) => nowLocal < reservationEndsAt(r));
    const liveNow = activeToday.filter((r) => isReservationLiveNow(r, nowLocal));

    if (liveNow.length > 0) {
      shortcuts.push({
        id: "rooms-live",
        pillarId: "administratif",
        moduleId: "prof-room",
        href: roomsHome,
        label: "Réservation de salle",
        rich: true,
        badge: liveNow.length === 1 ? "En cours" : `${liveNow.length} en cours`,
        detail:
          liveNow.length === 1
            ? `${liveNow[0]!.roomName || roomNameById.get(liveNow[0]!.roomId) || "Salle"} · ${liveNow[0]!.subject || "Réservée"}`
            : `${liveNow.length} de vos salles en cours`,
        tone: "info",
        slides: liveNow.map((r) => {
          const name = r.roomName || roomNameById.get(r.roomId) || "Salle";
          const subject = (r.subject || "").trim();
          const klass = (r.className || "").trim();
          const colorRaw = subject
            ? roomSubjectColors[subject] ||
              roomSubjectColors[subject.toUpperCase()] ||
              Object.entries(roomSubjectColors).find(
                ([k]) => k.toLowerCase() === subject.toLowerCase(),
              )?.[1]
            : undefined;
          return {
            id: r.id,
            label: name,
            detail: [subject || "Réservée", klass ? `· ${klass}` : ""].filter(Boolean).join(" "),
            badge: slotTimeLabel(r.startsAt),
            colorHex: subjectColorToHex(colorRaw || "bg-slate-600 text-white"),
          };
        }),
      });
    } else if (activeToday.length === 1) {
      const r = activeToday[0]!;
      const name = r.roomName || roomNameById.get(r.roomId) || "Salle";
      shortcuts.push({
        id: "rooms-today-one",
        pillarId: "administratif",
        moduleId: "prof-room",
        href: roomsHome,
        label: "Réservation de salle",
        rich: true,
        badge: slotTimeLabel(r.startsAt),
        detail: `${name}${r.subject ? ` · ${r.subject}` : ""}`,
        tone: "info",
      });
    } else if (activeToday.length > 1) {
      const next = activeToday[0]!;
      const name = next.roomName || roomNameById.get(next.roomId) || "Salle";
      shortcuts.push({
        id: "rooms-today-many",
        pillarId: "administratif",
        moduleId: "prof-room",
        href: roomsHome,
        label: "Réservation de salle",
        rich: true,
        badge: `${activeToday.length} créneaux`,
        detail: `Prochain : ${name} · ${slotTimeLabel(next.startsAt)}`,
        tone: "info",
      });
    } else {
      shortcuts.push({
        id: "rooms-empty",
        pillarId: "administratif",
        moduleId: "prof-room",
        href: roomsHome,
        label: "Réservation de salle",
        rich: true,
        detail: "Aucune réservation pour vous aujourd'hui",
        // Neutre = demi-tuile (pas pleine largeur) tout en gardant le détail.
        tone: "neutral",
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
        pillarId: "administratif",
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
        moduleId: "requests-staff",
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
        pillarId: "administratif",
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
        moduleId: "requests-staff",
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
        pillarId: "administratif",
        moduleId: "requests-staff",
        href: "/faire-une-demande",
        label: "Faire une demande",
      });
      shortcuts.push({
        id: "requests-mine",
        pillarId: "administratif",
        moduleId: "requests-staff",
        href: "/mes-demandes",
        label: "Voir mes demandes",
      });
    }
  }

  // —— Services : Photocopies (alertes direction uniquement — entrée via Boîte à outils) ——
  if (has("photocopies-couleur")) {
    const photoHome = moduleHref("photocopies-couleur");
    const etab = photocopiePendingForDirection(roles, photocopies, establishments);
    if (etab > 0) {
      const pending = etab;
      if (pending > 0) {
        shortcuts.push({
          id: "photo-dir",
          pillarId: "administratif",
          moduleId: "toolbox",
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
          moduleId: "toolbox",
          label: "Photocopies couleur",
          count: pending,
          href: photoHome,
          detail:
            pending === 1
              ? "1 photocopie à traiter"
              : `${pending} photocopies à traiter`,
        });
      }
    }
  }

  // —— Services : Cloud / dossiers partagés ——
  if (has("documents") && unseenSharedFolders.length > 0) {
    const docsHome = moduleHref("documents");
    for (const share of unseenSharedFolders) {
      pushNotif({
        id: `documents-share-${share.id}`,
        moduleId: "documents",
        label: "Dossier partagé",
        count: 1,
        href: `${docsHome}?shareId=${encodeURIComponent(share.id)}`,
        detail: `On vous a partagé « ${share.name} »`,
      });
    }
  }

  // —— Services : stables ——
  const stableServices: Array<{ moduleId: string; label: string; detail?: string }> = [
    { moduleId: "domain-planning", label: "Enseignements transversaux" },
    { moduleId: "documents", label: "Cloud personnel" },
    {
      moduleId: "toolbox",
      label: "Boîte à outils",
      detail: "QR code · Photocopies · outils activables",
    },
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
      ...(s.detail
        ? { rich: true as const, detail: s.detail, tone: "neutral" as const }
        : {}),
    });
  }

  // —— Dossier élève avant notes / santé ——
  if (has("eleve-dossier")) {
    const profDossierOnly =
      isProfesseurScopedDossierViewer({ roles }) && !hasRole(roles, "administratif");
    shortcuts.push({
      id: "eleve-dossier",
      pillarId: moduleIdToPillarId("eleve-dossier") ?? "administratif",
      moduleId: "eleve-dossier",
      href: moduleHref("eleve-dossier"),
      label: "Dossiers élèves",
      rich: true,
      detail: profDossierOnly ? "Vos classes · fiche pédagogique" : "Fiche unique · préinscriptions",
      tone: "info",
    });
  }
  if (has("notes")) {
    shortcuts.push({
      id: "notes",
      pillarId: "administratif",
      moduleId: "notes",
      href: moduleHref("notes"),
      label: "Notes & bulletins",
      rich: true,
      detail: "Saisie, compétences LSU, bulletins PDF",
      tone: "info",
    });
  }
  if (has("vs-appels") || has("vs-absences")) {
    const warnCount =
      (has("vs-appels") ? vsAppelsManquants : 0) +
      (has("vs-absences") ? Math.max(vsAbsencesJustifFamille, vsAbsencesATraiter > 0 ? vsAbsencesATraiter : 0) : 0);
    const detailParts: string[] = [];
    if (has("vs-appels") && vsAppelsManquants > 0) {
      detailParts.push(`${vsAppelsManquants} appel(s) manquant(s)`);
    }
    if (has("vs-absences") && vsAbsencesJustifFamille > 0) {
      detailParts.push(`${vsAbsencesJustifFamille} justificatif(s) famille`);
    } else if (has("vs-absences") && vsAbsencesATraiter > 0) {
      detailParts.push(`${vsAbsencesATraiter} absence(s) à traiter`);
    }
    shortcuts.push({
      id: "vs-presence",
      pillarId: "vie_scolaire",
      moduleId: "vs-appels",
      href:
        has("vs-absences") && vsAbsencesJustifFamille > 0
          ? `${moduleHref("vs-absences")}&filtre=justif_famille`
          : has("vs-appels") && vsAppelsManquants > 0
            ? `${moduleHref("vs-appels")}?tab=appel`
            : moduleHref("vs-appels"),
      label: "Appels & absences",
      rich: true,
      detail: detailParts.length > 0 ? detailParts.join(" · ") : "Présence en classe · justificatifs",
      badge: warnCount > 0 ? String(warnCount) : undefined,
      tone: warnCount > 0 ? "warn" : "info",
    });
    if (has("vs-appels")) {
      pushNotif({
        id: "vs-appels-manquants",
        moduleId: "vs-appels",
        label: "Appels manquants",
        count: vsAppelsManquants,
        href: `${moduleHref("vs-appels")}?tab=appel`,
        detail: "Créneaux commencés sans appel clôturé",
      });
    }
    if (has("vs-absences")) {
      pushNotif({
        id: "vs-absences-a-traiter",
        moduleId: "vs-absences",
        label: "Absences à traiter",
        count: vsAbsencesATraiter,
        href: `${moduleHref("vs-absences")}`,
        detail: "Justificatifs et relances familles",
      });
      pushNotif({
        id: "vs-absences-justif-famille",
        moduleId: "vs-absences",
        label: "Justificatifs famille",
        count: vsAbsencesJustifFamille,
        href: `${moduleHref("vs-absences")}&filtre=justif_famille`,
        detail: "Motifs parents à valider côté CPE",
      });
    }
  }
  if (has("vs-sanctions")) {
    shortcuts.push({
      id: "vs-sanctions",
      pillarId: "vie_scolaire",
      moduleId: "vs-sanctions",
      href: moduleHref("vs-sanctions"),
      label: "Sanctions",
      rich: true,
      detail:
        vsSanctionsAujourdhui > 0
          ? `${vsSanctionsAujourdhui} sanction(s) du jour`
          : "Avertissement, colle, blâme",
      badge: vsSanctionsAujourdhui > 0 ? String(vsSanctionsAujourdhui) : undefined,
      tone: vsSanctionsAujourdhui > 0 ? "warn" : "info",
    });
    pushNotif({
      id: "vs-sanctions-aujourdhui",
      moduleId: "vs-sanctions",
      label: "Sanctions du jour",
      count: vsSanctionsAujourdhui,
      href: moduleHref("vs-sanctions"),
      detail: "Incidents et sanctions enregistrés aujourd'hui",
    });
  }
  if (has("vs-carnet")) {
    shortcuts.push({
      id: "vs-carnet",
      pillarId: "vie_scolaire",
      moduleId: "vs-carnet",
      href: moduleHref("vs-carnet"),
      label: "Carnet",
      rich: true,
      detail:
        vsCarnetNonSignees > 0
          ? `${vsCarnetNonSignees} entrée(s) non signée(s)`
          : "Correspondance → famille",
      badge: vsCarnetNonSignees > 0 ? String(vsCarnetNonSignees) : undefined,
      tone: vsCarnetNonSignees > 0 ? "warn" : "info",
    });
    pushNotif({
      id: "vs-carnet-non-signees",
      moduleId: "vs-carnet",
      label: "Carnet non signé",
      count: vsCarnetNonSignees,
      href: moduleHref("vs-carnet"),
      detail: "Entrées en attente d'accusé famille",
    });
  }
  if (has("sante")) {
    shortcuts.push({
      id: "sante",
      pillarId: "sante",
      moduleId: "sante",
      href: moduleHref("sante"),
      label: "Espace santé",
      rich: true,
      detail: "Infirmerie · PAP",
      tone: "info",
    });
  }

  // —— Établissement : stables ——
  const stableEtab: Array<{ moduleId: string; label: string; detail?: string }> = [
    { moduleId: "organigramme", label: "Annuaire de l'établissement" },
    { moduleId: "evenements", label: "Événements" },
    {
      moduleId: "communication",
      label: "Communication",
      detail: "Documents familles · Simulateur tarifs",
    },
    {
      moduleId: "admin-settings",
      label: "Paramètres",
      detail: "Utilisateurs, établissement, liste des élèves",
    },
    { moduleId: "conformite-rgpd", label: "Conformité RGPD" },
    { moduleId: "chatbot-knowledge", label: "Brain AI" },
  ];
  for (const s of stableEtab) {
    if (!has(s.moduleId)) continue;
    shortcuts.push({
      id: s.moduleId,
      pillarId: "etablissement",
      moduleId: s.moduleId,
      href: moduleHref(s.moduleId),
      label: s.label,
      ...(s.detail
        ? { rich: true as const, detail: s.detail, tone: "neutral" as const }
        : {}),
    });
  }

  const news = buildTodayNewsFromWeekSheet(weekSheet);

  const remapped = shortcuts.map((s) => ({
    ...s,
    pillarId: moduleIdToPillarId(s.moduleId) ?? s.pillarId,
  }));

  return {
    shortcuts: remapped,
    todayNews: news.items,
    hasCurrentWeek: news.hasCurrentWeek,
    notifications,
    anneeScolaireLabel,
  };
}

function dayLabelFr(dayKey: WeekDayKey): string {
  return WEEK_DAYS.find((d) => d.key === dayKey)?.label ?? dayKey;
}
