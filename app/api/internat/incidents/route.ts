import { NextResponse } from "next/server";
import { requireInternatAccess, requireInternatManage } from "@/app/api/internat/_auth";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { hasRole } from "@/app/lib/absences-types";
import {
  getInternatIncidents,
  getInternatStudents,
  saveInternatIncidents,
} from "@/app/lib/internat-storage";
import { notifyInternatIncident } from "@/app/lib/internat-notify";
import {
  newId,
  studentDisplayName,
  type InternatIncident,
  type InternatIncidentWitnessStaff,
} from "@/app/lib/internat-types";

function isWitnessStaffRole(roles: string[]) {
  return hasRole(roles, "surveillant") || hasRole(roles, "internat");
}

function staffDisplayName(m: {
  firstName?: string;
  lastName?: string;
  displayName?: string;
  email: string;
}) {
  const n = `${m.lastName || ""} ${m.firstName || ""}`.trim();
  if (n) return n;
  return (m.displayName || m.email || "").trim();
}

async function listInternatWitnessStaff(): Promise<InternatIncidentWitnessStaff[]> {
  const members = await listDirectoryMembers().catch(() => []);
  return members
    .filter((m) => m.externalUserId && !m.pending && isWitnessStaffRole(m.roles || []))
    .sort((a, b) => {
      const la = `${a.lastName || ""} ${a.firstName || ""}`.trim() || a.displayName || a.email;
      const lb = `${b.lastName || ""} ${b.firstName || ""}`.trim() || b.displayName || b.email;
      return la.localeCompare(lb, "fr");
    })
    .map((m) => ({
      userId: m.externalUserId,
      name: staffDisplayName(m),
      email: m.email || undefined,
    }));
}

export async function GET(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;
  const { searchParams } = new URL(req.url);
  const studentId = searchParams.get("studentId") || undefined;
  const from = searchParams.get("from") || undefined;
  const to = searchParams.get("to") || undefined;
  let incidents = await getInternatIncidents();
  if (studentId) {
    incidents = incidents.filter(
      (i) => i.studentId === studentId || (i.studentIds || []).includes(studentId),
    );
  }
  if (from) incidents = incidents.filter((i) => i.occurredAt >= from);
  if (to) incidents = incidents.filter((i) => i.occurredAt <= to);
  incidents.sort((a, b) => b.occurredAt.localeCompare(a.occurredAt) || b.createdAt.localeCompare(a.createdAt));
  const staffWitnesses = await listInternatWitnessStaff();
  return NextResponse.json({ incidents, staffWitnesses });
}

export async function POST(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;
  const body = await req.json().catch(() => ({}));
  const title = String(body.title || "").trim();
  const occurredAt = String(body.occurredAt || "").trim() || new Date().toISOString().slice(0, 10);

  const rawIds = Array.isArray(body.studentIds)
    ? body.studentIds.map((id: unknown) => String(id || "").trim()).filter(Boolean)
    : body.studentId
      ? [String(body.studentId).trim()]
      : [];
  const studentIds = [...new Set(rawIds)];
  if (studentIds.length === 0 || !title) {
    return NextResponse.json({ error: "Au moins un interne et un titre (quoi) sont requis." }, { status: 400 });
  }

  const students = await getInternatStudents();
  const involved = studentIds
    .map((id) => students.find((s) => s.id === id))
    .filter((s): s is NonNullable<typeof s> => !!s);
  if (involved.length === 0) {
    return NextResponse.json({ error: "Interne(s) introuvable(s)." }, { status: 404 });
  }

  const witnessStudentIds = Array.isArray(body.witnessStudentIds)
    ? [...new Set(body.witnessStudentIds.map((id: unknown) => String(id || "").trim()).filter(Boolean))]
    : [];
  const witnessStudents = witnessStudentIds
    .map((id) => students.find((s) => s.id === id))
    .filter((s): s is NonNullable<typeof s> => !!s);

  const staffDirectory = await listInternatWitnessStaff();
  const staffById = new Map(staffDirectory.map((s) => [s.userId, s]));
  const rawStaffIds = Array.isArray(body.witnessStaffIds)
    ? body.witnessStaffIds.map((id: unknown) => String(id || "").trim()).filter(Boolean)
    : [];
  const witnessStaff = rawStaffIds
    .map((id) => staffById.get(id))
    .filter((s): s is InternatIncidentWitnessStaff => !!s);

  const witnessLabels = [
    ...witnessStudents.map((s) => `${studentDisplayName(s)} (interne)`),
    ...witnessStaff.map((s) => s.name),
  ];

  const primary = involved[0]!;
  const now = new Date().toISOString();
  const incident: InternatIncident = {
    id: newId("inc"),
    studentId: primary.id,
    studentName: studentDisplayName(primary),
    studentIds: involved.map((s) => s.id),
    studentNames: involved.map((s) => studentDisplayName(s)),
    kind: "incident",
    title,
    description: String(body.description || "").trim() || undefined,
    location: String(body.location || "").trim() || undefined,
    occurredTime: String(body.occurredTime || "").trim() || undefined,
    witnesses: witnessLabels.length ? witnessLabels.join(", ") : undefined,
    witnessStudentIds: witnessStudents.map((s) => s.id),
    witnessStaff,
    actionsTaken: String(body.actionsTaken || "").trim() || undefined,
    etablissement: involved.map((s) => s.etablissement).filter(Boolean).join(" · ") || primary.etablissement,
    classe: involved.map((s) => s.classe).filter(Boolean).join(", ") || primary.classe,
    occurredAt,
    createdAt: now,
    createdBy: { userId: access.userId, name: access.userName },
  };

  const mail = await notifyInternatIncident({ incident, students: involved });
  if (mail.sent) incident.notifySentAt = now;

  const incidents = await getInternatIncidents();
  incidents.push(incident);
  await saveInternatIncidents(incidents);
  return NextResponse.json({ incident, incidents, mail });
}

export async function DELETE(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;
  const { searchParams } = new URL(req.url);
  const id = String(searchParams.get("id") || "");
  const incidents = (await getInternatIncidents()).filter((i) => i.id !== id);
  await saveInternatIncidents(incidents);
  return NextResponse.json({ ok: true, incidents });
}
