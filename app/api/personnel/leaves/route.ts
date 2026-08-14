import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { getPersonnelLeaveRequests, upsertPersonnelLeaveRequest } from "@/app/lib/personnel-leave-storage";
import { getPersonnelRecord } from "@/app/lib/personnel-storage";
import { notifyPersonnelLeaveDecision } from "@/app/lib/personnel-notify";
import {
  canManagePersonnel,
  type PersonnelLeaveRequest,
  type PersonnelLeaveType,
  uid,
} from "@/app/lib/personnel-types";


export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const email = user?.primaryEmailAddress?.emailAddress?.toLowerCase() || "";
  const all = await getPersonnelLeaveRequests();

  if (canManagePersonnel(roles)) {
    return NextResponse.json({ requests: all });
  }

  const mine: PersonnelLeaveRequest[] = [];
  for (const r of all) {
    const rec = await getPersonnelRecord(r.personnelId);
    if (!rec) continue;
    if (rec.clerkUserId === user?.id || rec.email === email) mine.push(r);
  }
  return NextResponse.json({ requests: mine });
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "create");

  if (action === "create") {
    const personnelId = String(body.personnelId || "");
    const record = personnelId ? await getPersonnelRecord(personnelId) : null;
    if (!record) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });

    const email = user?.primaryEmailAddress?.emailAddress?.toLowerCase() || "";
    const isSelf = record.clerkUserId === user?.id || record.email === email;
    if (!isSelf && !canManagePersonnel(roles)) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }

    const startDate = String(body.startDate || "");
    const endDate = String(body.endDate || body.startDate || "");
    if (!startDate || !/^\d{4}-\d{2}-\d{2}$/.test(startDate)) {
      return NextResponse.json({ error: "Date de début invalide." }, { status: 400 });
    }

    const type = (["conge_paye", "rtt", "sans_solde", "autre"] as PersonnelLeaveType[]).includes(body.type)
      ? body.type
      : "conge_paye";

    const request: PersonnelLeaveRequest = {
      id: uid("lv"),
      personnelId: record.id,
      personnelName: record.displayName,
      type,
      startDate,
      endDate,
      reason: body.reason ? String(body.reason) : undefined,
      status: "en_attente",
      createdAt: new Date().toISOString(),
    };
    await upsertPersonnelLeaveRequest(request);
    return NextResponse.json({ request });
  }

  if (action === "decide") {
    if (!canManagePersonnel(roles)) {
      return NextResponse.json({ error: "Réservé à la RH." }, { status: 403 });
    }
    const id = String(body.id || "");
    const approved = body.approved === true;
    const all = await getPersonnelLeaveRequests();
    const idx = all.findIndex((r) => r.id === id);
    if (idx < 0) return NextResponse.json({ error: "Demande introuvable." }, { status: 404 });

    const now = new Date().toISOString();
    const updated: PersonnelLeaveRequest = {
      ...all[idx]!,
      status: approved ? "validee" : "refusee",
      decidedAt: now,
      decidedBy: user?.fullName || "RH",
      decisionNote: body.note ? String(body.note) : undefined,
    };
    await upsertPersonnelLeaveRequest(updated);

    const rec = await getPersonnelRecord(updated.personnelId);
    if (rec?.email) {
      await notifyPersonnelLeaveDecision({
        to: rec.email,
        employeeName: updated.personnelName,
        approved,
        startDate: updated.startDate,
        endDate: updated.endDate,
        note: updated.decisionNote,
      });
    }

    return NextResponse.json({ request: updated });
  }

  if (action === "cancel") {
    const id = String(body.id || "");
    const all = await getPersonnelLeaveRequests();
    const item = all.find((r) => r.id === id);
    if (!item) return NextResponse.json({ error: "Demande introuvable." }, { status: 404 });
    const rec = await getPersonnelRecord(item.personnelId);
    const email = user?.primaryEmailAddress?.emailAddress?.toLowerCase() || "";
    const isSelf = rec && (rec.clerkUserId === user?.id || rec.email === email);
    if (!isSelf && !canManagePersonnel(roles)) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    await upsertPersonnelLeaveRequest({ ...item, status: "annulee" });
    return NextResponse.json({ ok: true });
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}
