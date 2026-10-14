import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import { findEleveByIne } from "@/app/lib/eleves-registry";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import { collectEleveParentEmails } from "@/app/lib/eleves-parent-emails";
import { sanitizeElevePersonalEmail } from "@/app/lib/eleve-direction-email";
import { createAdminPlatformConvention } from "@/app/lib/stage-workflow";
import type { StageInternshipKind } from "@/app/lib/stage-types";

export const maxDuration = 90;

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Administratif";
}

function parseInternshipKind(raw: unknown): StageInternshipKind {
  const v = String(raw ?? "").trim().toLowerCase();
  if (v === "pfmp") return "pfmp";
  if (v === "job_ete") return "job_ete";
  if (v === "autre") return "autre";
  return "stage_observation";
}

/** Préremplit contacts élève depuis le registre (INE). */
export async function GET(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles)) {
      return NextResponse.json(
        { error: "Réservé à l'administratif / direction." },
        { status: 403 },
      );
    }

    const ine = new URL(req.url).searchParams.get("ine")?.trim().toUpperCase() || "";
    if (!ine) {
      return NextResponse.json({ error: "Paramètre ine requis." }, { status: 400 });
    }

    const eleve = await findEleveByIne(ine);
    if (!eleve) {
      return NextResponse.json({ found: false });
    }

    const parents = collectEleveParentEmails(eleve);
    return NextResponse.json({
      found: true,
      student: {
        firstName: eleve.prenom,
        lastName: eleve.nom,
        className: eleve.classe || "",
        dateNaissance: normalizeEleveDateNaissance(eleve.dateNaissance ?? "") || undefined,
        email: sanitizeElevePersonalEmail(eleve.email) || undefined,
        ine: eleve.ine,
        parent1Email: parents[0] || eleve.parent1Email || eleve.parentEmail || undefined,
        parent2Email: parents[1] || eleve.parent2Email || undefined,
      },
    });
  } catch (error) {
    console.error("[stages/conventions/platform GET]", error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : String(error) },
      { status: 500 },
    );
  }
}

export async function POST(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles)) {
      return NextResponse.json(
        { error: "Réservé à l'administratif / direction." },
        { status: 403 },
      );
    }

    const body = (await req.json()) as Record<string, unknown>;
    const studentRaw =
      body.student && typeof body.student === "object"
        ? (body.student as Record<string, unknown>)
        : {};
    const companyRaw =
      body.company && typeof body.company === "object"
        ? (body.company as Record<string, unknown>)
        : {};

    const str = (v: unknown) => String(v ?? "").trim();

    const result = await createAdminPlatformConvention({
      by: gate.ctx.userId,
      byName: displayName(user),
      student: {
        firstName: str(studentRaw.firstName),
        lastName: str(studentRaw.lastName),
        className: str(studentRaw.className),
        level: str(studentRaw.level) || undefined,
        dateNaissance: str(studentRaw.dateNaissance) || undefined,
        email: str(studentRaw.email) || undefined,
        matchedEleveIne: str(studentRaw.matchedEleveIne || studentRaw.ine) || undefined,
      },
      parent1Email: str(body.parent1Email) || undefined,
      parent2Email: str(body.parent2Email) || undefined,
      company: {
        name: str(companyRaw.name),
        address: str(companyRaw.address),
        postalCode: str(companyRaw.postalCode),
        city: str(companyRaw.city),
        siret: str(companyRaw.siret) || undefined,
        activity: str(companyRaw.activity) || undefined,
        tutorName: str(companyRaw.tutorName),
        tutorEmail: str(companyRaw.tutorEmail),
        tutorPhone: str(companyRaw.tutorPhone) || undefined,
        rhExtraSigner: Boolean(companyRaw.rhExtraSigner),
        rhFirstName: str(companyRaw.rhFirstName) || undefined,
        rhLastName: str(companyRaw.rhLastName) || undefined,
        rhEmail: str(companyRaw.rhEmail) || undefined,
      },
      schedule: body.schedule,
      periodStart: str(body.periodStart) || undefined,
      periodEnd: str(body.periodEnd) || undefined,
      internshipKind: parseInternshipKind(body.internshipKind),
      stageLabel: str(body.stageLabel) || undefined,
      note: str(body.note) || undefined,
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: 400 });
    }

    return NextResponse.json({
      success: true,
      convention: result.convention,
    });
  } catch (error) {
    console.error("[stages/conventions/platform POST]", error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : String(error) },
      { status: 500 },
    );
  }
}
