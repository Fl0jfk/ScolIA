import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canViewAllConventions, canViewReferentConventions } from "@/app/lib/stage-access";
import { buildStageClassRoster } from "@/app/lib/stage-class-roster";
import {
  classNameMatchesStageSecteurs,
  resolveStageViewerSecteurs,
} from "@/app/lib/stage-sector-scope";
import {
  classKey,
  findReferentAssignment,
  getStageReferentsConfig,
  listClassesForReferentUser,
  listStageReferentClassNames,
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
      const fromConfig = await listStageReferentClassNames();
      availableClasses = [...new Set([...fromConfig, ...referentClasses])].sort((a, b) =>
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
        message:
          "Aucune classe ne vous est assignée. L'administratif doit vous désigner comme professeur référent / principal dans Stages → Professeurs référents par classe.",
      });
    }

    const className = requestedClass || availableClasses[0] || "";
    if (!className) {
      return NextResponse.json({
        schoolYear,
        availableClasses,
        roster: null,
      });
    }

    if (!isAdmin && !referentClasses.some((c) => classKey(c) === classKey(className))) {
      return NextResponse.json({ error: "Classe non autorisée." }, { status: 403 });
    }

    const config = await getStageReferentsConfig(schoolYear);
    const assignment = findReferentAssignment(config, className);

    const roster = await buildStageClassRoster(className, schoolYear);

    return NextResponse.json({
      schoolYear,
      availableClasses,
      referent: assignment
        ? { name: assignment.name, email: assignment.email }
        : null,
      roster,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
