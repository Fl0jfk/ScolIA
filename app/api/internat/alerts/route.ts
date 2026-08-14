import { NextResponse } from "next/server";
import { requireInternatAccess } from "@/app/api/internat/_auth";
import { getInternatStudents, listInternatAlerts, saveInternatAlert } from "@/app/lib/internat-storage";
import { notifyInternatEmergency } from "@/app/lib/internat-notify";
import { studentDisplayName, newId, type InternatAlert, type InternatAlertSeverity, type InternatStudent } from "@/app/lib/internat-types";

export async function GET() {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;
  const alerts = await listInternatAlerts();
  return NextResponse.json({ alerts });
}

export async function POST(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;

  const body = await req.json().catch(() => ({}));
  const message = String(body.message || "").trim();
  if (!message) return NextResponse.json({ error: "Message requis." }, { status: 400 });

  const severity: InternatAlertSeverity =
    body.severity === "critique" || body.severity === "urgent" ? body.severity : "info";
  const studentIds: string[] = Array.isArray(body.studentIds) ? body.studentIds.map(String) : [];
  const students: InternatStudent[] = await getInternatStudents();
  const studentNames = studentIds
    .map((id: string) => students.find((s: InternatStudent) => s.id === id))
    .filter((s): s is InternatStudent => Boolean(s))
    .map((s) => studentDisplayName(s));

  const now = new Date().toISOString();
  const mail = await notifyInternatEmergency({
    message,
    severity,
    location: String(body.location || "").trim() || undefined,
    createdBy: access.userName,
    studentNames,
  });

  const alert: InternatAlert = {
    id: newId("alert"),
    createdAt: now,
    severity,
    message,
    location: String(body.location || "").trim() || undefined,
    studentIds: studentIds.length ? studentIds : undefined,
    createdBy: {
      userId: access.userId,
      name: access.userName,
      email: access.user?.primaryEmailAddress?.emailAddress,
    },
    sentAt: mail.sent ? now : undefined,
    recipients: mail.sent && "recipients" in mail ? mail.recipients : undefined,
  };

  await saveInternatAlert(alert);
  return NextResponse.json({ alert, mail });
}
