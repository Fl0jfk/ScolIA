import { NextResponse } from "next/server";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import {
  levelFromClasse,
  listScores,
  listStaffWishes,
  loadCampaignConfig,
  saveStaffWish,
  upsertScores,
} from "@/app/lib/class-allocation-storage";
import type { StaffWish, StudentScoreEntry } from "@/app/lib/class-allocation-types";
import { appendClassAllocationAudit } from "@/app/lib/class-allocation-audit";
import {
  canManageAllClassAllocationStudents,
  listClassesForTeacherUser,
  studentInAssignedClasses,
} from "@/app/lib/class-allocation-teachers";

function actorRole(roles: string[]): StaffWish["actorRole"] {
  if (roles.some((r) => r.startsWith("direction_"))) return "direction";
  if (roles.includes("admin")) return "admin";
  return "professeur";
}

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  const role = actorRole(roles);
  const fullAccess = canManageAllClassAllocationStudents(roles);
  const campaign = await loadCampaignConfig();
  const assignedClasses = fullAccess ? [] : await listClassesForTeacherUser(gate.ctx.userId, campaign.id);

  if (!fullAccess && assignedClasses.length === 0) {
    return NextResponse.json({
      campaign,
      role,
      assignedClasses: [],
      wishes: [],
      students: [],
      scores: [],
      message:
        "Aucune classe ne vous est assignée. L'administrateur doit vous rattacher dans Paramètres généraux → Référentiel scolaire.",
    });
  }

  const { searchParams } = new URL(req.url);
  const classFilter = searchParams.get("className")?.trim() || "";

  const [wishes, allEleves, scores] = await Promise.all([
    listStaffWishes(campaign.id),
    loadElevesActifsRegistry(),
    listScores(campaign.id),
  ]);

  let students = allEleves
    .map((s) => ({
      ine: s.ine,
      nom: s.nom,
      prenom: s.prenom,
      classe: s.classe,
      level: levelFromClasse(s.classe, campaign),
    }))
    .filter((s) => s.level);

  if (!fullAccess) {
    students = students.filter((s) => studentInAssignedClasses(s.classe, assignedClasses));
  }
  if (classFilter) {
    students = students.filter((s) => studentInAssignedClasses(s.classe, [classFilter]));
  }

  const studentInes = new Set(students.map((s) => s.ine));
  const visibleWishes = wishes.filter((w) => studentInes.has(w.studentIne));
  const visibleScores = scores.filter((s) => studentInes.has(s.studentIne));

  const availableClasses = fullAccess
    ? [...new Set(students.map((s) => s.classe).filter(Boolean) as string[])].sort((a, b) =>
        a.localeCompare(b, "fr", { sensitivity: "base" }),
      )
    : assignedClasses;

  return NextResponse.json({
    campaign,
    role,
    fullAccess,
    assignedClasses: availableClasses,
    wishes: visibleWishes,
    students,
    scores: visibleScores,
  });
}

export async function PUT(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  const role = actorRole(roles);
  const fullAccess = canManageAllClassAllocationStudents(roles);
  const campaign = await loadCampaignConfig();
  const assignedClasses = fullAccess ? [] : await listClassesForTeacherUser(gate.ctx.userId, campaign.id);

  const body = (await req.json()) as {
    studentIne?: string;
    separateFromInes?: string[];
    willingToTake?: boolean | null;
    note?: string;
    score?: number;
    gender?: "F" | "M" | "X";
  };
  const studentIne = String(body.studentIne || "").trim();
  if (!studentIne) return NextResponse.json({ error: "Élève requis." }, { status: 400 });

  const allEleves = await loadElevesActifsRegistry();
  const student = allEleves.find((s) => s.ine === studentIne);
  if (!student) return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });

  if (!fullAccess && !studentInAssignedClasses(student.classe, assignedClasses)) {
    return NextResponse.json({ error: "Vous ne pouvez saisir que les élèves de votre classe." }, { status: 403 });
  }

  const level = levelFromClasse(student.classe, campaign);
  if (!level) return NextResponse.json({ error: "Niveau non éligible." }, { status: 400 });

  const wish: StaffWish = {
    campaignId: campaign.id,
    studentIne,
    level,
    actorUserId: gate.ctx.userId,
    actorRole: role,
    separateFromInes: Array.isArray(body.separateFromInes)
      ? body.separateFromInes.map(String).map((s) => s.trim()).filter(Boolean).slice(0, 3)
      : [],
    willingToTake: role === "professeur" ? body.willingToTake ?? null : null,
    note: String(body.note || "").trim() || undefined,
    submittedAt: new Date().toISOString(),
  };
  await saveStaffWish(wish);

  if (typeof body.score === "number" && Number.isFinite(body.score)) {
    const scoreEntry: StudentScoreEntry = {
      studentIne,
      level,
      score: Math.min(100, Math.max(0, Number(body.score))),
      gender: body.gender,
      source: "manual",
      updatedAt: new Date().toISOString(),
    };
    await upsertScores(campaign.id, [scoreEntry]);
  }

  await appendClassAllocationAudit({
    at: new Date().toISOString(),
    action: "staff_wish_upserted",
    actor: gate.ctx.userId,
    details: { campaignId: campaign.id, studentIne, role, classe: student.classe },
  });

  return NextResponse.json({ ok: true });
}
