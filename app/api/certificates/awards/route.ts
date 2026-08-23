import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  canWorkOnProgram,
  eligibleSignatoryIds,
} from "@/app/lib/certificates-auth";
import {
  certificateUid,
  type StudentAward,
} from "@/app/lib/certificates-types";
import { loadProgram, saveAward, listAwardsForProgram } from "@/app/lib/certificates-storage";
import { findCertificateStudentByKey } from "@/app/lib/certificates-students";
import {
  buildDesignatedSignatories,
  generateVerificationToken,
  resolveDirectoryDisplayName,
} from "@/app/lib/certificates-workflow";

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const programId = new URL(req.url).searchParams.get("programId")?.trim();
  if (!programId) return NextResponse.json({ error: "programId requis." }, { status: 400 });
  const program = await loadProgram(programId);
  if (!program) return NextResponse.json({ error: "Parcours introuvable." }, { status: 404 });
  if (!canWorkOnProgram(program, gate.ctx.userId)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  const awards = await listAwardsForProgram(programId);
  return NextResponse.json({ awards });
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const body = await req.json().catch(() => ({}));
  const programId = String(body.programId || "").trim();
  const studentKey = String(body.studentKey || "").trim();
  const signatoryIds = Array.isArray(body.designatedSignatoryIds)
    ? body.designatedSignatoryIds.map((x: unknown) => String(x).trim()).filter(Boolean)
    : [];
  if (!programId || !studentKey) {
    return NextResponse.json({ error: "programId et studentKey requis." }, { status: 400 });
  }
  const program = await loadProgram(programId);
  if (!program) return NextResponse.json({ error: "Parcours introuvable." }, { status: 404 });
  if (!canWorkOnProgram(program, gate.ctx.userId)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  const existing = await listAwardsForProgram(programId);
  if (existing.some((a) => a.student.key === studentKey)) {
    return NextResponse.json({ error: "Cet élève est déjà dans ce parcours." }, { status: 400 });
  }
  const student = await findCertificateStudentByKey(studentKey);
  if (!student) return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
  if (!signatoryIds.length) {
    return NextResponse.json({ error: "Désignez au moins un prof signataire." }, { status: 400 });
  }
  const nameById = new Map<string, string>();
  for (const id of eligibleSignatoryIds(program)) {
    nameById.set(id, await resolveDirectoryDisplayName(id));
  }
  const now = new Date().toISOString();
  const award: StudentAward = {
    id: certificateUid("award"),
    programId,
    programTitle: program.title,
    schoolYear: program.schoolYear,
    addedBy: gate.ctx.userId,
    addedByName: user?.fullName || "Enseignant",
    student: {
      key: student.key,
      ine: student.ine,
      nom: student.nom,
      prenom: student.prenom,
      classe: student.classe,
      secteur: student.secteur,
    },
    lines: [],
    designatedSignatories: buildDesignatedSignatories(
      program,
      signatoryIds,
      gate.ctx.userId,
      nameById,
    ),
    status: "draft",
    verificationToken: generateVerificationToken(),
    contentHash: "",
    createdAt: now,
    updatedAt: now,
  };
  await saveAward(award);
  return NextResponse.json({ award });
}
