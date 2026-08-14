export type InternatEtablissement = "Collège" | "Lycée";
export type InternatSexe = "M" | "F";
export type InternatWing = "garcons" | "filles" | "mixte";
export type InternatRollMark = "present" | "absent" | "excuse" | "activite";

export const INTERNAT_ROLL_MARK_LABELS: Record<InternatRollMark, string> = {
  present: "Présent",
  absent: "Absent",
  excuse: "Excusé",
  activite: "Activité ext.",
};
export type InternatRollCallStatus = "ouverte" | "validee";
export type InternatAlertSeverity = "info" | "urgent" | "critique";

export const INTERNAT_S3 = {
  rooms: "internat/rooms.json",
  buildings: "internat/buildings.json",
  students: "internat/students.json",
  activities: "internat/activities.json",
  rollCallPrefix: "internat/roll-calls/",
  alertsPrefix: "internat/alerts/",
  outingsIndex: "internat/outings-index.json",
  outingsPrefix: "internat/outings/",
  studyGroups: "internat/study-groups.json",
  supervisorShifts: "internat/supervisor-shifts.json",
  incidents: "internat/incidents.json",
  messages: "internat/messages.json",
  journal: "internat/journal.json",
  moduleConfig: "settings/modules/internat.json",
  installationConfig: "internat/installation-config.json",
  installationBookings: "internat/installation-bookings.json",
} as const;

export type InternatFloor = {
  id: string;
  /** Ex. « Rez-de-chaussée », « 1er étage » */
  label: string;
  sortOrder: number;
  /** Étage utilisé pour loger des internes (sinon structure seulement). */
  inUse: boolean;
  notes?: string;
};

export type InternatBuilding = {
  id: string;
  label: string;
  floors: InternatFloor[];
  notes?: string;
  createdAt: string;
  updatedAt: string;
};

export type InternatRoomCapacity = 1 | 2 | 3 | 4;

export const INTERNAT_ROOM_CAPACITY_OPTIONS: InternatRoomCapacity[] = [1, 2, 3, 4];

export function parseInternatRoomCapacity(value: unknown, fallback: InternatRoomCapacity = 2): InternatRoomCapacity {
  const n = Number(value);
  if (n === 1 || n === 2 || n === 3 || n === 4) return n;
  return fallback;
}

export type InternatRoom = {
  id: string;
  label: string;
  capacity: InternatRoomCapacity;
  buildingId?: string;
  floorId?: string;
  wing?: InternatWing;
  notes?: string;
  createdAt: string;
  updatedAt: string;
};

export type InternatEleveRef = {
  ine?: string;
  folderName: string;
  nom: string;
  prenom: string;
};

export type InternatParentContact = {
  nom?: string;
  email?: string;
  telephone?: string;
};

export type InternatStudentMedical = {
  allergies?: string;
  pai?: string;
  treatments?: string;
  notes?: string;
};

export type InternatStudentSpecialAuth = {
  id: string;
  label: string;
  validFrom?: string;
  validUntil?: string;
  notes?: string;
};

export type InternatStudent = {
  id: string;
  eleveRef: InternatEleveRef;
  sexe: InternatSexe;
  etablissement: InternatEtablissement;
  classe: string;
  roomId?: string | null;
  parent1?: InternatParentContact;
  parent2?: InternatParentContact;
  medical?: InternatStudentMedical;
  specialAuthorizations?: InternatStudentSpecialAuth[];
  underWatch?: boolean;
  underWatchNote?: string;
  actif: boolean;
  createdAt: string;
  updatedAt: string;
  history?: Array<{ at: string; by: string; action: string; note?: string }>;
};

export type InternatOutingStatus =
  | "pending_direction"
  | "pending_parents"
  | "authorized"
  | "refused"
  | "cancelled";

export type InternatOutingDirectionStatus = "pending" | "approved" | "refused";

export type InternatOutingParentStatus = "pending" | "authorized" | "refused";

export type InternatOutingDirectionDecision = {
  etablissement: InternatEtablissement;
  token: string;
  status: InternatOutingDirectionStatus;
  directorEmail?: string;
  emailSentAt?: string;
  decidedAt?: string;
  decidedBy?: string;
  note?: string;
};

export type InternatOutingParticipant = {
  studentId: string;
  studentName: string;
  etablissement: InternatEtablissement;
  classe: string;
  parentToken: string;
  parent1Email?: string;
  parent2Email?: string;
  parentStatus: InternatOutingParentStatus;
  parentRespondedAt?: string;
  parentRespondedBy?: string;
  parentResponseIp?: string;
  parentEmailsSentAt?: string;
};

export type InternatOuting = {
  id: string;
  title: string;
  activity: string;
  destination?: string;
  accompanists: string;
  outingDate: string;
  departureTime?: string;
  returnTime?: string;
  participants: InternatOutingParticipant[];
  directionDecisions: InternatOutingDirectionDecision[];
  status: InternatOutingStatus;
  createdAt: string;
  updatedAt: string;
  createdBy: { userId: string; name: string };
  cancelledAt?: string;
  cancelledBy?: string;
};

export type InternatOutingIndexEntry = {
  id: string;
  title: string;
  outingDate: string;
  status: InternatOutingStatus;
  participantCount: number;
  createdAt: string;
};

export type InternatRollSection = {
  completed: boolean;
  completedBy?: string;
  completedAt?: string;
  marks: Record<string, InternatRollMark>;
};

export type InternatRollCallPeriod = "soir" | "matin";

export type InternatRollCall = {
  date: string;
  period?: InternatRollCallPeriod;
  status: InternatRollCallStatus;
  boys: InternatRollSection;
  girls: InternatRollSection;
  validatedAt?: string;
  validatedBy?: string;
  emailSentAt?: string;
  /** Rappel mail si l'appel n'est pas finalisé à l'heure configurée. */
  reminderSentAt?: string;
  updatedAt: string;
};

export type InternatActivity = {
  id: string;
  date: string;
  title: string;
  description?: string;
  participantIds?: string[];
  createdAt: string;
  createdBy: { userId: string; name: string };
};

export type InternatAlert = {
  id: string;
  createdAt: string;
  severity: InternatAlertSeverity;
  message: string;
  location?: string;
  studentIds?: string[];
  createdBy: { userId: string; name: string; email?: string };
  sentAt?: string;
  recipients?: string[];
};

export type InternatStudyGroup = {
  id: string;
  label: string;
  room?: string;
  weekday: number;
  startTime: string;
  endTime: string;
  studentIds: string[];
  supervisorName?: string;
  createdAt: string;
  updatedAt: string;
};

export type InternatSupervisorShift = {
  id: string;
  date: string;
  startTime: string;
  endTime: string;
  supervisorName: string;
  wing?: InternatWing;
  notes?: string;
  createdAt: string;
};

export type InternatIncidentKind = "incident" | "remarque" | "sanction" | "valorisation";

export type InternatIncident = {
  id: string;
  studentId: string;
  studentName: string;
  kind: InternatIncidentKind;
  title: string;
  description?: string;
  occurredAt: string;
  createdAt: string;
  createdBy: { userId: string; name: string };
};

export type InternatMessage = {
  id: string;
  threadId: string;
  subject: string;
  body: string;
  audience: "equipe" | "direction" | "cpe" | "surveillants";
  createdAt: string;
  createdBy: { userId: string; name: string };
  readBy?: string[];
};

export type InternatJournalEntry = {
  id: string;
  date: string;
  category: string;
  content: string;
  createdAt: string;
  createdBy: { userId: string; name: string };
};

export type InternatModuleConfig = {
  rollCallDeadlineHour?: number;
  /** Heure (Paris) du rappel si l'appel du soir n'est pas finalisé. */
  rollCallReminderHour?: number;
  rollCallReminderEnabled?: boolean;
  weeklySummaryEnabled?: boolean;
  /** Récap hebdo aux parents : sorties prévues la semaine suivante. */
  weeklyParentDigestEnabled?: boolean;
  /** Jour d'envoi (0 = dimanche). */
  weeklyParentDigestWeekday?: number;
  weeklyParentDigestLastSent?: string;
};

export type InternatRollCallRecipients = {
  appelContact?: string;
  directionLycee?: string;
  cpeLycee?: string;
  cpeCollege?: string;
};

export function newId(prefix: string) {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

export function etablissementFromSecteur(secteur?: string): InternatEtablissement {
  const s = String(secteur || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
  if (s.includes("college") || s.includes("colleg")) return "Collège";
  return "Lycée";
}

function emptyRollSection(): InternatRollSection {
  return { completed: false, marks: {} };
}

export function emptyRollCall(date: string, period: InternatRollCallPeriod = "soir"): InternatRollCall {
  const now = new Date().toISOString();
  return {
    date,
    period,
    status: "ouverte",
    boys: emptyRollSection(),
    girls: emptyRollSection(),
    updatedAt: now,
  };
}

export function studentDisplayName(s: InternatStudent) {
  return `${s.eleveRef.prenom} ${s.eleveRef.nom}`.trim();
}

export function sortInternatFloors(floors: InternatFloor[]): InternatFloor[] {
  return [...floors].sort(
    (a, b) => a.sortOrder - b.sortOrder || a.label.localeCompare(b.label, "fr"),
  );
}

function findInternatBuilding(
  buildings: InternatBuilding[],
  buildingId: string | undefined | null,
): InternatBuilding | undefined {
  if (!buildingId) return undefined;
  return buildings.find((b) => b.id === buildingId);
}

export function findInternatFloor(
  buildings: InternatBuilding[],
  buildingId: string | undefined | null,
  floorId: string | undefined | null,
): InternatFloor | undefined {
  const building = findInternatBuilding(buildings, buildingId);
  if (!building || !floorId) return undefined;
  return building.floors.find((f) => f.id === floorId);
}

export function roomLocationLabel(buildings: InternatBuilding[], room: InternatRoom): string {
  const building = findInternatBuilding(buildings, room.buildingId);
  const floor = findInternatFloor(buildings, room.buildingId, room.floorId);
  const parts = [building?.label, floor?.label].filter(Boolean);
  return parts.length ? parts.join(" · ") : "Non classée";
}

export function usableInternatFloors(building: InternatBuilding): InternatFloor[] {
  return sortInternatFloors(building.floors).filter((f) => f.inUse);
}

/** Jour d’ouverture pour la campagne d’installation (prise de chambre). */
export type InternatInstallationDay = {
  /** YYYY-MM-DD */
  date: string;
  /** HH:mm */
  openTime: string;
  /** HH:mm */
  closeTime: string;
};

export type InternatInstallationConfig = {
  enabled: boolean;
  title: string;
  intro?: string;
  /** Lieu affiché aux parents (mail / ICS). */
  location?: string;
  slotDurationMinutes: number;
  maxFamiliesPerSlot: number;
  days: InternatInstallationDay[];
  /** Clés `YYYY-MM-DDTHH:mm` bloquées manuellement. */
  closedSlots: string[];
};

export type InternatInstallationBooking = {
  id: string;
  /** Clé créneau `YYYY-MM-DDTHH:mm` (heure murale Europe/Paris). */
  slotStart: string;
  studentFirstName: string;
  studentLastName: string;
  parentPhone: string;
  parentEmail: string;
  createdAt: string;
  /** Absent = confirmé (rétrocompat des RDV déjà pris). */
  status?: "pending" | "confirmed";
  confirmToken?: string;
  expiresAt?: string;
};

export type InternatInstallationPublicSlot = {
  slotStart: string;
  date: string;
  time: string;
  remaining: number;
  capacity: number;
};
