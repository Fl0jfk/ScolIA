import { NextResponse } from "next/server";
import { requireInternatAccess, requireInternatManage } from "@/app/api/internat/_auth";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  buildDirectionDecisions,
  buildOutingParticipants,
  computeOutingStatus,
  isDateInRange,
  nextWeekDateRange,
} from "@/app/lib/internat-outing";
import { applyOutingDecision, notifyParentsIfReady } from "@/app/lib/internat-outing-decision";
import { notifyInternatOutingDirection } from "@/app/lib/internat-notify";
import {
  getInternatOuting,
  getInternatStudents,
  listInternatOutings,
  saveInternatOuting,
} from "@/app/lib/internat-storage";
import { newId, type InternatOuting } from "@/app/lib/internat-types";

export async function GET(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;

  const { searchParams } = new URL(req.url);
  const view = searchParams.get("view");

  if (view === "authorized") {
    const scope = searchParams.get("scope") === "week" ? "week" : "today";
    const today = new Date();
    const todayStr = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
    const week = nextWeekDateRange(today);
    const outings = await listInternatOutings();
    const authorized = outings
      .filter((o) => o.status === "authorized")
      .filter((o) =>
        scope === "today"
          ? o.outingDate === todayStr
          : isDateInRange(o.outingDate, week.start, week.end),
      )
      .flatMap((o) =>
        o.participants
          .filter((p) => p.parentStatus === "authorized")
          .map((p) => ({
            studentId: p.studentId,
            studentName: p.studentName,
            classe: p.classe,
            etablissement: p.etablissement,
            outingId: o.id,
            outingTitle: o.title,
            outingDate: o.outingDate,
            departureTime: o.departureTime,
            returnTime: o.returnTime,
          })),
      );
    return NextResponse.json({ authorized, scope, ...(scope === "week" ? { week } : { date: todayStr }) });
  }

  const outings = await listInternatOutings();
  return NextResponse.json({ outings });
}

export async function POST(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;

  const body = await req.json().catch(() => ({}));
  const title = String(body.title || "").trim();
  const activity = String(body.activity || "").trim();
  const accompanists = String(body.accompanists || "").trim();
  const outingDate = String(body.outingDate || "").trim();
  const studentIds = Array.isArray(body.studentIds) ? body.studentIds.map(String) : [];

  if (!title || !activity || !accompanists || !outingDate || !/^\d{4}-\d{2}-\d{2}$/.test(outingDate)) {
    return NextResponse.json(
      { error: "Titre, activité, accompagnement, date et au moins un interne sont requis." },
      { status: 400 },
    );
  }
  if (!studentIds.length) {
    return NextResponse.json({ error: "Sélectionnez au moins un interne." }, { status: 400 });
  }

  const students = await getInternatStudents();
  const { participants, missingParents } = buildOutingParticipants(studentIds, students);
  if (!participants.length) {
    return NextResponse.json({ error: "Aucun interne valide sélectionné." }, { status: 400 });
  }
  if (missingParents.length) {
    return NextResponse.json(
      {
        error: `E-mail parent manquant pour : ${missingParents.join(", ")}. Complétez la fiche interne avant de créer la sortie.`,
      },
      { status: 400 },
    );
  }

  const bundle = await loadAppConfig();
  const now = new Date().toISOString();
  const outing: InternatOuting = {
    id: newId("out"),
    title,
    activity,
    destination: String(body.destination || "").trim() || undefined,
    accompanists,
    outingDate,
    departureTime: String(body.departureTime || "").trim() || undefined,
    returnTime: String(body.returnTime || "").trim() || undefined,
    participants,
    directionDecisions: buildDirectionDecisions(participants, bundle.establishments),
    status: "pending_direction",
    createdAt: now,
    updatedAt: now,
    createdBy: { userId: access.userId, name: access.userName },
  };

  const emailWarnings: string[] = [];
  for (let i = 0; i < outing.directionDecisions.length; i++) {
    const d = outing.directionDecisions[i]!;
    if (!d.directorEmail) {
      emailWarnings.push(`Pas d'e-mail direction pour ${d.etablissement}.`);
      continue;
    }
    const result = await notifyInternatOutingDirection({ outing, decisionIndex: i });
    if (result.sent) {
      outing.directionDecisions[i] = { ...d, emailSentAt: new Date().toISOString() };
    } else {
      emailWarnings.push(`E-mail direction ${d.etablissement} non envoyé (${result.reason}).`);
    }
  }

  outing.updatedAt = new Date().toISOString();
  await saveInternatOuting(outing);

  return NextResponse.json({
    outing,
    emailWarnings: emailWarnings.length ? emailWarnings : undefined,
  });
}

export async function PATCH(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;

  const body = await req.json().catch(() => ({}));
  const id = String(body.id || "");
  const action = String(body.action || "");

  const outing = await getInternatOuting(id);
  if (!outing) return NextResponse.json({ error: "Sortie introuvable." }, { status: 404 });

  if (action === "cancel") {
    if (outing.status === "cancelled") {
      return NextResponse.json({ error: "Sortie déjà annulée." }, { status: 400 });
    }
    const updated: InternatOuting = {
      ...outing,
      status: "cancelled",
      cancelledAt: new Date().toISOString(),
      cancelledBy: access.userName,
      updatedAt: new Date().toISOString(),
    };
    await saveInternatOuting(updated);
    return NextResponse.json({ outing: updated });
  }

  if (action === "resend_parents") {
    let updated = await notifyParentsIfReady(outing);
    updated = { ...updated, status: computeOutingStatus(updated), updatedAt: new Date().toISOString() };
    await saveInternatOuting(updated);
    return NextResponse.json({ outing: updated });
  }

  const decision = body.decision === "refuse" ? "refuse" : body.decision === "approve" ? "approve" : null;

  if (action === "direction_decision") {
    if (!decision) return NextResponse.json({ error: "Décision requise." }, { status: 400 });
    const etablissement = String(body.etablissement || "").trim() || null;
    if (!etablissement) return NextResponse.json({ error: "Établissement requis." }, { status: 400 });
    try {
      const { outing: updated } = await applyOutingDecision({
        kind: "direction",
        etablissement,
        decision,
        decidedBy: access.userName,
        note: String(body.note || "").trim() || undefined,
        outing,
      });
      await saveInternatOuting(updated);
      return NextResponse.json({ outing: updated });
    } catch (e: unknown) {
      return NextResponse.json({ error: e instanceof Error ? e.message : "Erreur" }, { status: 400 });
    }
  }

  if (action === "parent_decision") {
    if (!decision) return NextResponse.json({ error: "Décision requise." }, { status: 400 });
    const studentId = String(body.studentId || "");
    if (!studentId) return NextResponse.json({ error: "Interne requis." }, { status: 400 });
    try {
      const { outing: updated } = await applyOutingDecision({
        kind: "parent",
        studentId,
        decision,
        decidedBy: access.userName,
        note: String(body.note || "").trim() || undefined,
        outing,
      });
      await saveInternatOuting(updated);
      return NextResponse.json({ outing: updated });
    } catch (e: unknown) {
      return NextResponse.json({ error: e instanceof Error ? e.message : "Erreur" }, { status: 400 });
    }
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}
