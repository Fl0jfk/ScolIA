import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { loadAppConfig } from "@/app/lib/app-config";
import { listClerkMembers } from "@/app/lib/clerk-users";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { getPersonnelIndex, getPersonnelRecord } from "@/app/lib/personnel-storage";
import {
  canAccessPersonnelModule,
  canManagePersonnel,
  canViewPersonnelDashboard,
} from "@/app/lib/personnel-types";
import { readRhPlanning, writeRhPlanning } from "@/app/lib/rh/planning-storage";
import {
  defaultStaffModeForCategory,
  emptyStaffPlanning,
  estimateAnnualBalance,
  normalizeStaffPlanning,
  normalizeTeacherPlanning,
  type RhPlanningKind,
  type StaffPlanningDoc,
  type TeacherPlanningDoc,
} from "@/app/lib/rh/planning-types";

function rolesFromUser(user: NonNullable<Awaited<ReturnType<typeof safeCurrentUser>>>) {
  const rolesRaw = user?.publicMetadata?.role;
  return Array.isArray(rolesRaw) ? rolesRaw.map(String) : rolesRaw ? [String(rolesRaw)] : [];
}

function isTeacherRole(roles: string[]) {
  return hasRole(roles, "professeur");
}

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const roles = rolesFromUser(user);
  if (!canAccessPersonnelModule(roles) && !canViewPersonnelDashboard(roles) && !isTeacherRole(roles)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  const canManage = canManagePersonnel(roles);
  const canViewAll = canViewPersonnelDashboard(roles);
  const url = new URL(req.url);
  const listAudience = url.searchParams.get("audience");

  if (listAudience === "teachers") {
    if (!canManage && !canViewAll) {
      return NextResponse.json({ error: "Liste réservée à la RH.", status: 403 });
    }
    const members = await listClerkMembers();
    const people = members
      .filter((m) => m.clerkUserId && !m.pending)
      .filter((m) => normalizeIntranetRoles(m.roles).includes("professeur"))
      .map((m) => ({
        id: m.clerkUserId,
        displayName: m.displayName || m.email,
        category: "professeur",
        jobTitle: null as string | null,
      }))
      .sort((a, b) => a.displayName.localeCompare(b.displayName, "fr"));
    return NextResponse.json({ audience: listAudience, people, canManage });
  }

  if (listAudience === "staff") {
    if (!canManage && !canViewAll) {
      return NextResponse.json({ error: "Liste réservée à la RH.", status: 403 });
    }
    const index = await getPersonnelIndex();
    const people = index
      .filter((e) => e.active !== false)
      .map((e) => ({
        id: e.id,
        displayName: e.displayName || e.email,
        category: e.category,
        jobTitle: null as string | null,
      }))
      .sort((a, b) => a.displayName.localeCompare(b.displayName, "fr"));
    return NextResponse.json({ audience: listAudience, people, canManage });
  }

  const requestedId = url.searchParams.get("personnelId")?.trim() || "";
  const kindParam = url.searchParams.get("kind");
  let kind: RhPlanningKind =
    kindParam === "teacher" || kindParam === "staff"
      ? kindParam
      : isTeacherRole(roles)
        ? "teacher"
        : "staff";

  let subjectId = requestedId;
  let displayName = "";
  let category = kind === "teacher" ? "professeur" : "administratif";

  if (!subjectId) {
    if (kind === "teacher" || isTeacherRole(roles)) {
      kind = "teacher";
      subjectId = gate.ctx.userId;
      displayName = user.fullName || user.primaryEmailAddress?.emailAddress || "Moi";
      category = "professeur";
    } else {
      const index = await getPersonnelIndex();
      const self = index.find((e) => e.clerkUserId === gate.ctx.userId && e.active !== false);
      if (!self) {
        return NextResponse.json({ error: "Aucun dossier RH lié à votre compte." }, { status: 404 });
      }
      kind = "staff";
      subjectId = self.id;
      displayName = self.displayName || self.email;
      category = self.category;
    }
  } else if (kind === "teacher") {
    const isSelf = subjectId === gate.ctx.userId;
    if (!isSelf && !canManage && !canViewAll) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    const members = await listClerkMembers();
    const m = members.find((x) => x.clerkUserId === subjectId);
    displayName = m?.displayName || m?.email || subjectId;
    category = "professeur";
  } else {
    const index = await getPersonnelIndex();
    const entry = index.find((e) => e.id === subjectId && e.active !== false);
    if (!entry) {
      return NextResponse.json({ error: "Collaborateur introuvable." }, { status: 404 });
    }
    const isSelf = entry.clerkUserId === gate.ctx.userId;
    if (!isSelf && !canManage && !canViewAll) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
    displayName = entry.displayName || entry.email;
    category = entry.category;
    kind = "staff";
  }

  let planning = await readRhPlanning(kind, subjectId);

  if (kind === "staff" && planning.kind === "staff") {
    const preferred = defaultStaffModeForCategory(category);
    if (
      planning.fixedSlots.length === 0 &&
      planning.rotations.every((r) => r.slots.length === 0) &&
      planning.mode !== preferred
    ) {
      planning = emptyStaffPlanning(subjectId, preferred);
    }
  }

  const isSelf =
    kind === "teacher"
      ? subjectId === gate.ctx.userId
      : (await getPersonnelRecord(subjectId))?.clerkUserId === gate.ctx.userId;
  const canEdit = canManage || !!isSelf;

  const balance =
    planning.kind === "staff" && planning.mode === "fixed"
      ? estimateAnnualBalance(planning)
      : null;

  let schoolHolidayZone: "A" | "B" | "C" | null = null;
  try {
    const cfg = await loadAppConfig();
    schoolHolidayZone = cfg.identity.schoolHolidayZone ?? null;
  } catch {
    schoolHolidayZone = null;
  }

  return NextResponse.json({
    kind,
    personnelId: subjectId,
    displayName,
    category,
    planning,
    balance,
    canEdit,
    canManage,
    schoolHolidayZone,
  });
}

export async function PUT(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const roles = rolesFromUser(user);
  const canManage = canManagePersonnel(roles);

  let body: { personnelId?: unknown; planning?: unknown };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const personnelId = typeof body.personnelId === "string" ? body.personnelId.trim() : "";
  if (!personnelId) {
    return NextResponse.json({ error: "personnelId requis." }, { status: 400 });
  }

  const raw = body.planning;
  if (!raw || typeof raw !== "object") {
    return NextResponse.json({ error: "planning requis." }, { status: 400 });
  }

  const kindHint = (raw as { kind?: string }).kind;
  const kind: RhPlanningKind = kindHint === "staff" ? "staff" : "teacher";

  let allowed = false;
  if (kind === "teacher") {
    allowed = canManage || personnelId === gate.ctx.userId;
  } else {
    const record = await getPersonnelRecord(personnelId);
    if (!record || record.active === false) {
      return NextResponse.json({ error: "Collaborateur introuvable." }, { status: 404 });
    }
    allowed = canManage || record.clerkUserId === gate.ctx.userId;
  }

  if (!allowed) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  const updatedBy = user.fullName || user.primaryEmailAddress?.emailAddress || gate.ctx.userId;

  let next: TeacherPlanningDoc | StaffPlanningDoc;
  if (kind === "teacher") {
    const incoming = normalizeTeacherPlanning(raw, personnelId);
    const existing = (await readRhPlanning("teacher", personnelId)) as TeacherPlanningDoc;
    // Remplacements : écriture réservée RH ; le collab conserve l’existant.
    const replacements = canManage ? incoming.replacements : existing.replacements || [];
    next = {
      ...incoming,
      replacements,
      updatedBy,
      updatedAt: new Date().toISOString(),
    };
  } else {
    next = {
      ...normalizeStaffPlanning(raw, personnelId),
      updatedBy,
      updatedAt: new Date().toISOString(),
    };
  }

  const saved = await writeRhPlanning(next);
  const balance =
    saved.kind === "staff" && saved.mode === "fixed" ? estimateAnnualBalance(saved) : null;
  return NextResponse.json({ ok: true, planning: saved, balance });
}
