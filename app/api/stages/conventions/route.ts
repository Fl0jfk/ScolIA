import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  canCreateConventionAsStaff,
  canCreateOffer,
  canReviewPreconvention,
  canViewAllConventions,
  canViewReferentConventions,
} from "@/app/lib/stage-access";
import { conventionVisibleToUser } from "@/app/lib/stage-referent";
import { ensureConventionReferent } from "@/app/lib/stage-referents-config";
import { defaultStageSchedule } from "@/app/lib/stage-schedule";
import {
  ensureStudentAccessToken,
  normalizeConventionInput,
} from "@/app/lib/stage-workflow";
import {
  getConventionsIndex,
  getStageConvention,
  getStageOffer,
  listConventionsForDossier,
  saveStageConvention,
} from "@/app/lib/stage-storage";
import {
  currentStageSchoolYear,
  stageUid,
  studentDossierKey,
  type StageConvention,
} from "@/app/lib/stage-types";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Utilisateur";
}

export async function GET(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const { searchParams } = new URL(req.url);
    const dossier = searchParams.get("dossier");

    if (dossier === "1" && searchParams.get("firstName") && searchParams.get("lastName") && searchParams.get("className")) {
      const conventions = await listConventionsForDossier({
        firstName: searchParams.get("firstName")!,
        lastName: searchParams.get("lastName")!,
        className: searchParams.get("className")!,
      });
      return NextResponse.json({
        dossierKey: studentDossierKey({
          firstName: searchParams.get("firstName")!,
          lastName: searchParams.get("lastName")!,
          className: searchParams.get("className")!,
        }),
        conventions,
      });
    }

    if (!canViewAllConventions(roles) && !canViewReferentConventions(roles) && !canCreateOffer(roles)) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const index = await getConventionsIndex();
    const all = await Promise.all(index.map((e) => getStageConvention(e.id)));
    const userEmail = user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
    const conventions = all
      .filter((c): c is NonNullable<typeof c> => Boolean(c))
      .filter((c) => conventionVisibleToUser(c, roles, userEmail, gate.ctx.userId));
    return NextResponse.json({ conventions });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const isParent = canCreateOffer(roles);
    const isStaff = canCreateConventionAsStaff(roles);
    if (!isParent && !isStaff) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const body = await req.json();
    const now = new Date().toISOString();
    let convention: StageConvention = {
      id: stageUid("conv"),
      schoolYear: currentStageSchoolYear(),
      status: "draft",
      internshipKind: "pfmp",
      student: {
        firstName: "",
        lastName: "",
        className: "",
        level: "3e",
      },
      company: {
        name: "",
        address: "",
        activity: "",
        tutorName: "",
        tutorEmail: "",
      },
      schedule: defaultStageSchedule("uniform_week"),
      teacherReferent: { name: "", email: "" },
      signatures: [],
      createdAt: now,
      updatedAt: now,
      createdBy: {
        role: isParent ? "parent" : "staff",
        userId: gate.ctx.userId,
        name: displayName(user),
      },
      history: [{ at: now, by: displayName(user), action: "CREATION" }],
    };

    if (body.offerId) {
      const offer = await getStageOffer(String(body.offerId));
      if (offer && offer.status === "approved") {
        convention.offerId = offer.id;
        convention.internshipKind = offer.kind === "job_ete" ? "job_ete" : offer.kind;
        convention.company = {
          name: offer.companyName,
          address: offer.companyAddress || "",
          siret: offer.companySiret,
          activity: offer.sector || offer.description.slice(0, 120),
          tutorName: offer.contactName,
          tutorEmail: offer.contactEmail,
          tutorPhone: offer.contactPhone,
        };
        if (offer.periodStart && offer.periodEnd) {
          convention.schedule = {
            ...convention.schedule,
            periodStart: offer.periodStart,
            periodEnd: offer.periodEnd,
          };
        }
      }
    }

    convention = normalizeConventionInput(body, convention);

    if (roles.includes("professeur") && !canReviewPreconvention(roles)) {
      const email = user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
      const name = displayName(user);
      convention.teacherReferent = {
        name: convention.teacherReferent.name?.trim() || name,
        email: convention.teacherReferent.email?.trim() || email,
        userId: gate.ctx.userId,
      };
    }

    convention = await ensureConventionReferent(convention);
    convention = await ensureStudentAccessToken(convention);
    await saveStageConvention(convention);

    return NextResponse.json({
      success: true,
      convention,
      studentLink: convention.studentAccessToken
        ? `/stages/eleve?token=${encodeURIComponent(convention.studentAccessToken)}`
        : null,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
