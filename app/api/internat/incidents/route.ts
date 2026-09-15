import { NextResponse } from "next/server";
import { requireInternatAccess, requireInternatManage } from "@/app/api/internat/_auth";
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
  type InternatIncidentKind,
  type InternatIncidentSeverity,
} from "@/app/lib/internat-types";

const KINDS: InternatIncidentKind[] = ["incident", "remarque", "sanction", "valorisation"];
const SEVERITIES: InternatIncidentSeverity[] = ["faible", "moyenne", "grave"];

export async function GET(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;
  const { searchParams } = new URL(req.url);
  const studentId = searchParams.get("studentId") || undefined;
  const from = searchParams.get("from") || undefined;
  const to = searchParams.get("to") || undefined;
  const kind = searchParams.get("kind") || undefined;
  let incidents = await getInternatIncidents();
  if (studentId) incidents = incidents.filter((i) => i.studentId === studentId);
  if (from) incidents = incidents.filter((i) => i.occurredAt >= from);
  if (to) incidents = incidents.filter((i) => i.occurredAt <= to);
  if (kind && KINDS.includes(kind as InternatIncidentKind)) {
    incidents = incidents.filter((i) => i.kind === kind);
  }
  incidents.sort((a, b) => b.occurredAt.localeCompare(a.occurredAt) || b.createdAt.localeCompare(a.createdAt));
  return NextResponse.json({ incidents });
}

export async function POST(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;
  const body = await req.json().catch(() => ({}));
  const studentId = String(body.studentId || "");
  const title = String(body.title || "").trim();
  const kind = KINDS.includes(body.kind) ? (body.kind as InternatIncidentKind) : "incident";
  const severity = SEVERITIES.includes(body.severity)
    ? (body.severity as InternatIncidentSeverity)
    : kind === "incident"
      ? "moyenne"
      : undefined;
  const occurredAt = String(body.occurredAt || "").trim() || new Date().toISOString().slice(0, 10);
  if (!studentId || !title) {
    return NextResponse.json({ error: "Interne et titre requis." }, { status: 400 });
  }
  const students = await getInternatStudents();
  const student = students.find((s) => s.id === studentId);
  if (!student) return NextResponse.json({ error: "Interne introuvable." }, { status: 404 });

  const now = new Date().toISOString();
  const incident: InternatIncident = {
    id: newId("inc"),
    studentId,
    studentName: studentDisplayName(student),
    kind,
    title,
    description: String(body.description || "").trim() || undefined,
    location: String(body.location || "").trim() || undefined,
    occurredTime: String(body.occurredTime || "").trim() || undefined,
    witnesses: String(body.witnesses || "").trim() || undefined,
    otherPeople: String(body.otherPeople || "").trim() || undefined,
    actionsTaken: String(body.actionsTaken || "").trim() || undefined,
    severity,
    etablissement: student.etablissement,
    classe: student.classe,
    occurredAt,
    createdAt: now,
    createdBy: { userId: access.userId, name: access.userName },
  };

  let mail: { sent: boolean; reason?: string } | null = null;
  if (kind === "incident" || kind === "sanction") {
    mail = await notifyInternatIncident({ incident, student });
    if (mail.sent) incident.notifySentAt = now;
  }

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
