import { cookies } from "next/headers";
import { NextResponse } from "next/server";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { loadSchoolRoster } from "@/app/lib/school-roster";
import {
  levelFromClasse,
  loadCampaignConfig,
  saveParentWish,
} from "@/app/lib/class-allocation-storage";
import type { ParentWish } from "@/app/lib/class-allocation-types";
import { appendClassAllocationAudit } from "@/app/lib/class-allocation-audit";
import {
  openParentSession,
  parentSessionCookieName,
} from "@/app/lib/class-allocation-parent-auth";
import {
  resolvePeerWishInputs,
  resolveTeacherWishInput,
} from "@/app/lib/class-allocation-wish-match";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const parentSubmitLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 20,
});

type Body = {
  studentIne?: string;
  preferredStudents?: string[];
  avoidStudents?: string[];
  preferredTeacher?: string;
  avoidTeacher?: string;
};

function normalizeTexts(v: unknown, max: number): string[] {
  if (!Array.isArray(v)) return [];
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of v) {
    const s = String(raw || "").trim();
    if (!s) continue;
    const key = s.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(s);
    if (out.length >= max) break;
  }
  return out;
}

export async function POST(req: Request) {
  if (!parentSubmitLimiter.allow(clientIpFromRequest(req))) {
    return NextResponse.json(
      { error: "Trop de tentatives. Réessayez dans quelques minutes." },
      { status: 429 },
    );
  }
  const campaign = await loadCampaignConfig();
  if (!campaign.isOpen) {
    return NextResponse.json({ error: "La campagne est fermée." }, { status: 400 });
  }

  const jar = await cookies();
  const session = openParentSession(jar.get(parentSessionCookieName())?.value);
  if (!session || session.campaignId !== campaign.id) {
    return NextResponse.json({ error: "Connexion requise. Validez votre e-mail parent." }, { status: 401 });
  }

  let body: Body;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }
  const studentIne = String(body.studentIne || "").trim();
  if (!studentIne) return NextResponse.json({ error: "Élève requis." }, { status: 400 });

  const [all, roster] = await Promise.all([loadElevesRegistry(), loadSchoolRoster()]);
  const student = all.find((s) => s.ine === studentIne);
  if (!student) return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
  if (!session.childInes.includes(studentIne)) {
    return NextResponse.json({ error: "Vous ne pouvez saisir des vœux que pour vos enfants." }, { status: 403 });
  }

  const level = levelFromClasse(student.classe, campaign);
  if (!level) return NextResponse.json({ error: "Niveau non éligible à la campagne." }, { status: 400 });

  const preferredInputs = normalizeTexts(body.preferredStudents, 3);
  const avoidInputs = normalizeTexts(body.avoidStudents, 3);
  const preferredTeacherInput = String(body.preferredTeacher || "").trim();
  const avoidTeacherInput = String(body.avoidTeacher || "").trim();

  const teacherCatalog =
    roster.teacherCatalog.length > 0 ? roster.teacherCatalog : campaign.teacherCatalog;

  const [preferredResolved, avoidResolved, preferredTeacherHit, avoidTeacherHit] = await Promise.all([
    resolvePeerWishInputs({
      inputs: preferredInputs,
      max: 3,
      level,
      campaign,
      excludeIne: studentIne,
      allEleves: all,
    }),
    resolvePeerWishInputs({
      inputs: avoidInputs,
      max: 3,
      level,
      campaign,
      excludeIne: studentIne,
      allEleves: all,
    }),
    preferredTeacherInput
      ? resolveTeacherWishInput(preferredTeacherInput, teacherCatalog)
      : Promise.resolve(null),
    avoidTeacherInput ? resolveTeacherWishInput(avoidTeacherInput, teacherCatalog) : Promise.resolve(null),
  ]);

  const preferred = preferredResolved.resolved.map((r) => r.ine);
  const avoid = avoidResolved.resolved.map((r) => r.ine);
  const overlap = preferred.filter((ine) => avoid.includes(ine));
  if (overlap.length) {
    return NextResponse.json(
      { error: "Un même élève ne peut pas être à la fois souhaité et refusé." },
      { status: 400 },
    );
  }

  const unresolvedPeer = [...preferredResolved.unresolved, ...avoidResolved.unresolved];
  const unresolvedTeacher = [
    preferredTeacherInput && !preferredTeacherHit ? preferredTeacherInput : "",
    avoidTeacherInput && !avoidTeacherHit ? avoidTeacherInput : "",
  ].filter(Boolean);

  const wish: ParentWish = {
    campaignId: campaign.id,
    studentIne,
    studentName: `${student.nom} ${student.prenom}`.trim(),
    parentEmail: session.email,
    level,
    preferredStudentInes: preferred,
    avoidStudentInes: avoid,
    preferredStudentInputs: preferredInputs,
    avoidStudentInputs: avoidInputs,
    preferredTeacher: preferredTeacherHit?.matchedName,
    avoidTeacher: avoidTeacherHit?.matchedName,
    preferredTeacherInput: preferredTeacherInput || undefined,
    avoidTeacherInput: avoidTeacherInput || undefined,
    unresolvedPeerInputs: unresolvedPeer.length ? unresolvedPeer : undefined,
    unresolvedTeacherInputs: unresolvedTeacher.length ? unresolvedTeacher : undefined,
    submittedAt: new Date().toISOString(),
  };
  await saveParentWish(wish);
  await appendClassAllocationAudit({
    at: new Date().toISOString(),
    action: "parent_wish_submitted",
    actor: `parent:${session.email}`,
    details: {
      campaignId: campaign.id,
      level,
      resolvedPeers: preferred.length + avoid.length,
      unresolvedPeers: unresolvedPeer,
      unresolvedTeachers: unresolvedTeacher,
    },
  });

  const warnings: string[] = [];
  if (unresolvedPeer.length) {
    warnings.push(
      `Certain(s) nom(s) d'élève n'ont pas pu être identifiés et ne seront pas pris en compte : ${unresolvedPeer.join(", ")}.`,
    );
  }
  if (unresolvedTeacher.length) {
    warnings.push(
      `Certain(s) nom(s) de professeur n'ont pas pu être identifiés : ${unresolvedTeacher.join(", ")}.`,
    );
  }

  return NextResponse.json({
    ok: true,
    warnings,
    resolved: {
      preferredStudents: preferredResolved.resolved.length,
      avoidStudents: avoidResolved.resolved.length,
      preferredTeacher: Boolean(preferredTeacherHit),
      avoidTeacher: Boolean(avoidTeacherHit),
    },
  });
}
