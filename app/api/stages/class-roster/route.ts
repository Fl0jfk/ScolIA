import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { listDirectoryMembers } from "@/app/lib/directory-members";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention, canViewAllConventions, canViewReferentConventions } from "@/app/lib/stage-access";
import { buildStageClassRoster, listStageRosterClassNames } from "@/app/lib/stage-class-roster";
import {
  classNameMatchesStageSecteurs,
  resolveStageViewerSecteurs,
} from "@/app/lib/stage-sector-scope";
import {
  classKey,
  findReferentAssignments,
  getStageReferentsConfig,
  listClassesForReferentUser,
  userCanAssignStageReferentForClass,
} from "@/app/lib/stage-referents-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

export async function GET(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const isAdmin = canViewAllConventions(roles);
    const isReferent = canViewReferentConventions(roles);

    if (!isAdmin && !isReferent) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const { searchParams } = new URL(req.url);
    const schoolYear = searchParams.get("schoolYear")?.trim() || currentStageSchoolYear();
    const requestedClass = searchParams.get("className")?.trim() || "";
    const viewerSecteurs = await resolveStageViewerSecteurs(roles, gate.ctx.userId);

    const referentClasses = user
      ? await listClassesForReferentUser(gate.ctx.userId, schoolYear)
      : [];

    let availableClasses: string[];
    if (isAdmin) {
      const fromRoster = await listStageRosterClassNames(schoolYear);
      availableClasses = [...new Set([...fromRoster, ...referentClasses])].sort((a, b) =>
        a.localeCompare(b, "fr", { sensitivity: "base" }),
      );
    } else {
      availableClasses = referentClasses;
    }

    if (viewerSecteurs.length > 0) {
      availableClasses = availableClasses.filter((c) =>
        classNameMatchesStageSecteurs(c, viewerSecteurs),
      );
    }

    if (availableClasses.length === 0 && !isAdmin) {
      return NextResponse.json({
        schoolYear,
        availableClasses: [],
        roster: null,
        canAssignReferent: false,
        teachers: [],
        message:
          "Aucune classe ne vous est assignée. L'administratif doit vous désigner comme professeur principal / référent dans Stages → Réglages.",
      });
    }

    const className = requestedClass || availableClasses[0] || "";
    if (!className) {
      return NextResponse.json({
        schoolYear,
        availableClasses,
        roster: null,
        canAssignReferent: false,
        teachers: [],
      });
    }

    if (!isAdmin && !referentClasses.some((c) => classKey(c) === classKey(className))) {
      return NextResponse.json({ error: "Classe non autorisée." }, { status: 403 });
    }

    const config = await getStageReferentsConfig(schoolYear);
    const assignments = findReferentAssignments(config, className);
    const canAssignReferent =
      canReviewPreconvention(roles) ||
      (await userCanAssignStageReferentForClass(gate.ctx.userId, className, schoolYear));

    let teachers: Array<{
      externalUserId: string;
      email: string;
      displayName: string;
    }> = [];
    if (canAssignReferent) {
      const members = await listDirectoryMembers();
      teachers = members
        .filter((m) => m.externalUserId && !m.pending)
        .filter(
          (m) =>
            m.roles.includes("professeur") ||
            m.roles.includes("surveillant") ||
            m.roles.includes("cpe"),
        )
        .map((m) => ({
          externalUserId: m.externalUserId,
          email: m.email,
          displayName:
            m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim() || m.email,
        }))
        .sort((a, b) => a.displayName.localeCompare(b.displayName, "fr", { sensitivity: "base" }));
    }

    const roster = await buildStageClassRoster(className, schoolYear);

    return NextResponse.json({
      schoolYear,
      availableClasses,
      referents: assignments.map((a) => ({
        name: a.name,
        email: a.email,
        role: a.role,
      })),
      canAssignReferent,
      teachers,
      roster,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
