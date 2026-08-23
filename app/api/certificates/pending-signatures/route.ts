import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessCertificatesModule, canSignAwardAsDirectionForUserId } from "@/app/lib/certificates-auth";
import { loadAwardsIndex, loadAward, loadProgram } from "@/app/lib/certificates-storage";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  if (!canAccessCertificatesModule(user)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  const userId = gate.ctx.userId;
  const index = await loadAwardsIndex();
  const pending: Array<{
    awardId: string;
    programId: string;
    programTitle: string;
    studentName: string;
    status: string;
  }> = [];

  for (const entry of index) {
    if (entry.status !== "submitted" && entry.status !== "prof_signed") continue;
    const award = await loadAward(entry.id);
    if (!award) continue;
    const sig = award.designatedSignatories.find((s) => s.externalUserId === userId);
    if (!sig || sig.status === "signed") continue;
    const program = await loadProgram(award.programId);
    pending.push({
      awardId: award.id,
      programId: award.programId,
      programTitle: award.programTitle,
      studentName: entry.studentName,
      status: award.status,
    });
  }

  const directionPending: Array<{
    awardId: string;
    programId: string;
    programTitle: string;
    studentName: string;
    secteur: string;
  }> = [];

  for (const entry of index) {
    if (entry.status !== "prof_signed") continue;
    const award = await loadAward(entry.id);
    if (!award || award.directionSignature) continue;
    const allSigned = award.designatedSignatories.every((s) => s.status === "signed");
    if (!allSigned) continue;
    if (!(await canSignAwardAsDirectionForUserId(userId, award))) continue;
    directionPending.push({
      awardId: award.id,
      programId: award.programId,
      programTitle: award.programTitle,
      studentName: entry.studentName,
      secteur: award.student.secteur,
    });
  }

  return NextResponse.json({ prof: pending, direction: directionPending });
}
