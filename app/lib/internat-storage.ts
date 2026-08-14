import { getJson, putJson, listPrefix } from "@/app/lib/s3-storage";
import { INTERNAT_ROSTER_KEY, type InternatRosterFile } from "@/app/lib/internat-import";
import {
  INTERNAT_S3,
  emptyRollCall,
  type InternatActivity,
  type InternatAlert,
  type InternatIncident,
  type InternatJournalEntry,
  type InternatMessage,
  type InternatModuleConfig,
  type InternatOuting,
  type InternatOutingIndexEntry,
  type InternatRollCall,
  type InternatRollCallPeriod,
  type InternatBuilding,
  type InternatRoom,
  type InternatStudent,
  type InternatStudyGroup,
  type InternatSupervisorShift,
} from "@/app/lib/internat-types";
import { outingIndexEntry } from "@/app/lib/internat-outing";

function rollCallKey(date: string, period: InternatRollCallPeriod = "soir") {
  const suffix = period === "matin" ? "-matin" : "";
  return `${INTERNAT_S3.rollCallPrefix}${date}${suffix}.json`;
}

function parseRollCallKey(filename: string): { date: string; period: InternatRollCallPeriod } | null {
  const base = filename.replace(/\.json$/, "");
  if (/^\d{4}-\d{2}-\d{2}-matin$/.test(base)) {
    return { date: base.replace(/-matin$/, ""), period: "matin" };
  }
  if (/^\d{4}-\d{2}-\d{2}$/.test(base)) {
    return { date: base, period: "soir" };
  }
  return null;
}

function alertKey(id: string) {
  return `${INTERNAT_S3.alertsPrefix}${id}.json`;
}

export async function getInternatRoster(): Promise<InternatRosterFile | null> {
  const hit = await getJson<InternatRosterFile>(INTERNAT_ROSTER_KEY);
  return hit?.data || null;
}

export async function saveInternatRoster(roster: InternatRosterFile) {
  await putJson(INTERNAT_ROSTER_KEY, roster);
}

export async function getInternatRooms(): Promise<InternatRoom[]> {
  const hit = await getJson<InternatRoom[]>(INTERNAT_S3.rooms);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInternatRooms(rooms: InternatRoom[]) {
  await putJson(INTERNAT_S3.rooms, rooms);
}

export async function getInternatBuildings(): Promise<InternatBuilding[]> {
  const hit = await getJson<InternatBuilding[]>(INTERNAT_S3.buildings);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInternatBuildings(buildings: InternatBuilding[]) {
  await putJson(INTERNAT_S3.buildings, buildings);
}

export async function getInternatStudents(): Promise<InternatStudent[]> {
  const hit = await getJson<InternatStudent[]>(INTERNAT_S3.students);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInternatStudents(students: InternatStudent[]) {
  await putJson(INTERNAT_S3.students, students);
}

export async function getInternatActivities(): Promise<InternatActivity[]> {
  const hit = await getJson<InternatActivity[]>(INTERNAT_S3.activities);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInternatActivities(activities: InternatActivity[]) {
  await putJson(INTERNAT_S3.activities, activities);
}

export async function getInternatRollCall(
  date: string,
  period: InternatRollCallPeriod = "soir",
): Promise<InternatRollCall> {
  const hit = await getJson<InternatRollCall>(rollCallKey(date, period));
  if (hit?.data) return { period, ...hit.data };
  return emptyRollCall(date, period);
}

export async function saveInternatRollCall(rollCall: InternatRollCall) {
  const period = rollCall.period || "soir";
  await putJson(rollCallKey(rollCall.date, period), { ...rollCall, period });
}

async function listInternatRollCallDates(): Promise<Array<{ date: string; period: InternatRollCallPeriod }>> {
  const keys = await listPrefix(INTERNAT_S3.rollCallPrefix);
  return keys
    .map((k) => {
      const name = k.split("/").pop() || "";
      return parseRollCallKey(name);
    })
    .filter((x): x is { date: string; period: InternatRollCallPeriod } => !!x)
    .sort((a, b) => b.date.localeCompare(a.date) || (a.period === "matin" ? 1 : 0) - (b.period === "matin" ? 1 : 0));
}

export async function listValidatedRollCalls(limit = 30, period?: InternatRollCallPeriod): Promise<InternatRollCall[]> {
  const entries = (await listInternatRollCallDates())
    .filter((e) => !period || e.period === period)
    .slice(0, limit * 2);
  const out: InternatRollCall[] = [];
  for (const { date, period: p } of entries) {
    const rc = await getInternatRollCall(date, p);
    if (rc.status === "validee") out.push(rc);
    if (out.length >= limit) break;
  }
  return out;
}

export async function listRollCallHistory(params: {
  from?: string;
  to?: string;
  studentId?: string;
  period?: InternatRollCallPeriod;
  limit?: number;
}) {
  const limit = params.limit ?? 60;
  const entries = await listInternatRollCallDates();
  const filtered = entries.filter((e) => {
    if (params.period && e.period !== params.period) return false;
    if (params.from && e.date < params.from) return false;
    if (params.to && e.date > params.to) return false;
    return true;
  });

  const out: Array<{
    date: string;
    period: InternatRollCallPeriod;
    status: InternatRollCall["status"];
    validatedAt?: string;
    validatedBy?: string;
    marks: Array<{ studentId: string; mark: string }>;
  }> = [];

  for (const { date, period } of filtered) {
    if (out.length >= limit) break;
    const rc = await getInternatRollCall(date, period);
    if (rc.status !== "validee") continue;
    const marks: Array<{ studentId: string; mark: string }> = [];
    for (const [studentId, mark] of Object.entries(rc.boys.marks)) {
      marks.push({ studentId, mark });
    }
    for (const [studentId, mark] of Object.entries(rc.girls.marks)) {
      marks.push({ studentId, mark });
    }
    const scoped =
      params.studentId ? marks.filter((m) => m.studentId === params.studentId) : marks;
    if (params.studentId && scoped.length === 0) continue;
    out.push({
      date,
      period,
      status: rc.status,
      validatedAt: rc.validatedAt,
      validatedBy: rc.validatedBy,
      marks: scoped,
    });
  }
  return out;
}

export async function saveInternatAlert(alert: InternatAlert) {
  await putJson(alertKey(alert.id), alert);
}

export async function listInternatAlerts(limit = 50): Promise<InternatAlert[]> {
  const keys = (await listPrefix(INTERNAT_S3.alertsPrefix))
    .filter((k) => k.endsWith(".json"))
    .sort()
    .reverse()
    .slice(0, limit);
  const out: InternatAlert[] = [];
  for (const key of keys) {
    const file = key.split("/").pop() || "";
    const hit = await getJson<InternatAlert>(`${INTERNAT_S3.alertsPrefix}${file}`);
    if (hit?.data) out.push(hit.data);
  }
  return out;
}

async function getInternatModuleConfig(): Promise<InternatModuleConfig> {
  const hit = await getJson<InternatModuleConfig>(INTERNAT_S3.moduleConfig);
  return hit?.data || { rollCallDeadlineHour: 22, weeklySummaryEnabled: true };
}

async function saveInternatModuleConfig(config: InternatModuleConfig) {
  await putJson(INTERNAT_S3.moduleConfig, config);
}

function outingKey(id: string) {
  return `${INTERNAT_S3.outingsPrefix}${id}.json`;
}

async function getInternatOutingsIndex(): Promise<InternatOutingIndexEntry[]> {
  const hit = await getJson<InternatOutingIndexEntry[]>(INTERNAT_S3.outingsIndex);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function saveInternatOutingsIndex(entries: InternatOutingIndexEntry[]) {
  const sorted = [...entries].sort((a, b) => b.outingDate.localeCompare(a.outingDate) || b.createdAt.localeCompare(a.createdAt));
  await putJson(INTERNAT_S3.outingsIndex, sorted);
}

export async function getInternatOuting(id: string): Promise<InternatOuting | null> {
  const hit = await getJson<InternatOuting>(outingKey(id));
  return hit?.data || null;
}

export async function saveInternatOuting(outing: InternatOuting) {
  await putJson(outingKey(outing.id), outing);
  const index = await getInternatOutingsIndex();
  const entry = outingIndexEntry(outing);
  const idx = index.findIndex((e) => e.id === outing.id);
  if (idx >= 0) index[idx] = entry;
  else index.push(entry);
  await saveInternatOutingsIndex(index);
}

export async function listInternatOutings(limit = 100): Promise<InternatOuting[]> {
  const index = (await getInternatOutingsIndex()).slice(0, limit);
  const out: InternatOuting[] = [];
  for (const e of index) {
    const o = await getInternatOuting(e.id);
    if (o) out.push(o);
  }
  return out;
}

export async function findOutingByToken(token: string): Promise<InternatOuting | null> {
  const outings = await listInternatOutings(200);
  for (const outing of outings) {
    if (outing.status === "cancelled") continue;
    if (outing.directionDecisions.some((d) => d.token === token)) return outing;
    if (outing.participants.some((p) => p.parentToken === token)) return outing;
  }
  return null;
}

export function countStudentsInRoom(students: InternatStudent[], roomId: string) {
  return students.filter((s) => s.actif && s.roomId === roomId).length;
}

export function validateRoomCapacity(
  students: InternatStudent[],
  rooms: InternatRoom[],
  studentId: string,
  roomId: string | null | undefined,
) {
  if (!roomId) return { ok: true as const };
  const room = rooms.find((r) => r.id === roomId);
  if (!room) return { ok: false as const, error: "Chambre introuvable." };
  const others = students.filter((s) => s.actif && s.roomId === roomId && s.id !== studentId).length;
  if (others >= room.capacity) {
    return { ok: false as const, error: `La chambre ${room.label} est pleine (${room.capacity} places).` };
  }
  return { ok: true as const };
}

export async function getInternatStudyGroups(): Promise<InternatStudyGroup[]> {
  const hit = await getJson<InternatStudyGroup[]>(INTERNAT_S3.studyGroups);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInternatStudyGroups(groups: InternatStudyGroup[]) {
  await putJson(INTERNAT_S3.studyGroups, groups);
}

export async function getInternatSupervisorShifts(): Promise<InternatSupervisorShift[]> {
  const hit = await getJson<InternatSupervisorShift[]>(INTERNAT_S3.supervisorShifts);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInternatSupervisorShifts(shifts: InternatSupervisorShift[]) {
  await putJson(INTERNAT_S3.supervisorShifts, shifts);
}

export async function getInternatIncidents(): Promise<InternatIncident[]> {
  const hit = await getJson<InternatIncident[]>(INTERNAT_S3.incidents);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInternatIncidents(incidents: InternatIncident[]) {
  await putJson(INTERNAT_S3.incidents, incidents);
}

export async function getInternatMessages(): Promise<InternatMessage[]> {
  const hit = await getJson<InternatMessage[]>(INTERNAT_S3.messages);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInternatMessages(messages: InternatMessage[]) {
  await putJson(INTERNAT_S3.messages, messages);
}

export async function getInternatJournal(): Promise<InternatJournalEntry[]> {
  const hit = await getJson<InternatJournalEntry[]>(INTERNAT_S3.journal);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveInternatJournal(entries: InternatJournalEntry[]) {
  await putJson(INTERNAT_S3.journal, entries);
}
