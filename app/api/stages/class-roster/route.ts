import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { listDirectoryMembers } from "@/app/lib/directory-members";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention, canBrowseStageConventions, canViewReferentConventions } from "@/app/lib/stage-access";
import { buildStageClassRoster, listStageRosterClassNames, searchStageConventionsGlobal } from "@/app/lib/stage-class-roster";
import {
  classNameMatchesStageSecteurs,
  resolveStageViewerSecteurs,
} from "@/app/lib/stage-sector-scope";
import {
  classKey,
  findReferentAssignments,
  getStageReferentsConfig,
  listClassesForReferentUser,
  listPrincipalClassesForUser,
  userCanAssignStageReferentForClass,
} from "@/app/lib/stage-referents-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

export async function GET(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const canBrowseAll = canBrowseStageConventions(roles);
    const isReferent = canViewReferentConventions(roles);

    if (!canBrowseAll && !isReferent) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const { searchParams } = new URL(req.url);
    const schoolYear = searchParams.get("schoolYear")?.trim() || currentStageSchoolYear();
    const requestedClass = searchParams.get("className")?.trim() || "";
    const globalQuery = searchParams.get("q")?.trim() || "";
    const viewerSecteurs = await resolveStageViewerSecteurs(roles, gate.ctx.userId);

    const userEmail = user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
    const [referentClasses, principalClasses] = user
      ? await Promise.all([
          listClassesForReferentUser(gate.ctx.userId, schoolYear),
          listPrincipalClassesForUser(gate.ctx.userId, schoolYear),
        ])
      : [[], []];

    let availableClasses: string[];
    if (canBrowseAll) {
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

    if (globalQuery.length >= 2) {
      const globalResults = await searchStageConventionsGlobal(globalQuery, {
        schoolYear,
        allowedClasses: canBrowseAll ? null : availableClasses,
      });
      return NextResponse.json({
        schoolYear,
        availableClasses,
        globalResults,
        roster: null,
      });
    }

    if (availableClasses.length === 0 && !canBrowseAll) {
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

    if (!canBrowseAll && !referentClasses.some((c) => classKey(c) === classKey(className))) {
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
        .filter((m) => m.roles.includes("professeur"))
        .map((m) => {
          const lastName = String(m.lastName ?? "").trim();
          const firstName = String(m.firstName ?? "").trim();
          const byLastName = [lastName, firstName].filter(Boolean).join(" ");
          return {
            externalUserId: m.externalUserId,
            email: m.email,
            displayName: byLastName || m.displayName || m.email,
            sortKey: `${lastName} ${firstName} ${m.email}`.trim(),
          };
        })
        .sort((a, b) =>
          a.sortKey.localeCompare(b.sortKey, "fr", { sensitivity: "base" }),
        )
        .map(({ externalUserId, email, displayName }) => ({
          externalUserId,
          email,
          displayName,
        }));
    }

    const roster = await buildStageClassRoster(className, schoolYear);
    const isPrincipalForClass = principalClasses.some(
      (c) => classKey(c) === classKey(className),
    );

    /** Référent (non PP) : uniquement les élèves dont il est teacherReferent. */
    const scopedStudents =
      canBrowseAll || isPrincipalForClass
        ? roster.students
        : roster.students
            .map((s) => ({
              ...s,
              conventions: s.conventions.filter((conv) => {
                const refEmail = String(conv.teacherReferentEmail || "")
                  .trim()
                  .toLowerCase();
                if (userEmail && refEmail && refEmail === userEmail) return true;
                return false;
              }),
            }))
            .filter((s) => s.conventions.length > 0)
            .map((s) => {
              const hasValide = s.conventions.some((c) => c.status === "signed");
              const hasEnCours = s.conventions.some(
                (c) =>
                  c.status === "signatures_pending" ||
                  c.status === "convention_ready" ||
                  c.status === "admin_review" ||
                  c.status === "preconvention_submitted" ||
                  c.status === "convention_deposited",
              );
              const rosterStatus =
                s.conventions.length > 1
                  ? ("plusieurs" as const)
                  : hasValide && !hasEnCours
                    ? ("valide" as const)
                    : hasEnCours
                      ? ("en_cours" as const)
                      : ("sans_stage" as const);
              return { ...s, rosterStatus };
            });

    const scopedRoster = {
      ...roster,
      students: scopedStudents,
      summary: {
        total: scopedStudents.length,
        sansStage: scopedStudents.filter((s) => s.rosterStatus === "sans_stage").length,
        enCours: scopedStudents.filter((s) => s.rosterStatus === "en_cours").length,
        valide: scopedStudents.filter((s) => s.rosterStatus === "valide").length,
        plusieurs: scopedStudents.filter((s) => s.rosterStatus === "plusieurs").length,
      },
      note:
        !canBrowseAll && !isPrincipalForClass
          ? "Vue référent : uniquement les stagiaires dont vous êtes le professeur référent."
          : roster.note,
    };

    const { resolvePhotoUrlsForEleves } = await import("@/app/lib/eleve-photos");
    const photoIds = scopedRoster.students
      .filter((s) => Boolean(s.eleveId))
      .map((s) => ({
        id: s.eleveId!,
        nom: s.nom,
        prenom: s.prenom,
        ine: s.ine,
        photoKey: s.photoKey,
      }));
    const photoUrls: Record<string, string> =
      photoIds.length > 0
        ? await resolvePhotoUrlsForEleves(photoIds).catch(
            (): Record<string, string> => ({}),
          )
        : {};
    const rosterWithPhotos = {
      ...scopedRoster,
      students: scopedRoster.students.map((s) => ({
        ...s,
        photoUrl: s.eleveId ? photoUrls[s.eleveId] ?? null : null,
      })),
    };

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
      roster: rosterWithPhotos,
      viewerScope: canBrowseAll
        ? "all"
        : isPrincipalForClass
          ? "principal"
          : "referent",
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
