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
  findStudentReferentAssignmentForStudent,
  getStageReferentsConfig,
  listClassesForReferentUser,
  listPrincipalClassesForUser,
  userCanAssignStageReferentForClass,
} from "@/app/lib/stage-referents-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";
import { createPerfTimer } from "@/app/lib/perf-timer";

export async function GET(req: Request) {
  const perf = createPerfTimer();
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;
    perf.mark("auth");

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
    perf.mark("secteurs");

    const userEmail = user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
    const userId = gate.ctx.userId;
    const [referentClasses, principalClasses] = user
      ? await Promise.all([
          listClassesForReferentUser(gate.ctx.userId, schoolYear),
          listPrincipalClassesForUser(gate.ctx.userId, schoolYear),
        ])
      : [[], []];
    perf.mark("referent_classes");

    let availableClasses: string[];
    if (canBrowseAll) {
      const fromRoster = await listStageRosterClassNames(schoolYear);
      availableClasses = [...new Set([...fromRoster, ...referentClasses])].sort((a, b) =>
        a.localeCompare(b, "fr", { sensitivity: "base" }),
      );
    } else {
      availableClasses = referentClasses;
    }
    perf.mark("available_classes");

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
      perf.mark("global_search");
      return NextResponse.json({
        schoolYear,
        availableClasses,
        globalResults,
        roster: null,
        perf: perf.snapshot(),
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
        perf: perf.snapshot(),
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
        perf: perf.snapshot(),
      });
    }

    if (!canBrowseAll && !referentClasses.some((c) => classKey(c) === classKey(className))) {
      return NextResponse.json({ error: "Classe non autorisée." }, { status: 403 });
    }

    const canAssignReferent =
      canReviewPreconvention(roles) ||
      (await userCanAssignStageReferentForClass(gate.ctx.userId, className, schoolYear));
    perf.mark("can_assign");

    const [config, roster, members] = await Promise.all([
      getStageReferentsConfig(schoolYear),
      buildStageClassRoster(className, schoolYear),
      canAssignReferent ? listDirectoryMembers() : Promise.resolve(null),
    ]);
    perf.mark("roster_build");

    const assignments = findReferentAssignments(config, className);

    const rosterWithReferents = {
      ...roster,
      students: roster.students.map((s) => {
        const hit = findStudentReferentAssignmentForStudent(config, {
          className,
          studentKey: s.key,
          nom: s.nom,
          prenom: s.prenom,
          ine: s.ine,
          eleveId: s.eleveId,
        });
        const fromConv = s.conventions[0];
        return {
          ...s,
          assignedReferentName: hit?.name || fromConv?.teacherReferentName,
          assignedReferentEmail: hit?.email || fromConv?.teacherReferentEmail,
          assignedReferentUserId: hit?.externalUserId || fromConv?.teacherReferentUserId,
        };
      }),
    };

    const teachers =
      members == null
        ? []
        : members
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

    const isPrincipalForClass = principalClasses.some(
      (c) => classKey(c) === classKey(className),
    );

    /** Référent (non PP) : élèves qui lui sont affectés (même sans convention) + dossiers teacherReferent. */
    const scopedStudents =
      canBrowseAll || isPrincipalForClass
        ? rosterWithReferents.students
        : rosterWithReferents.students
            .map((s) => {
              const assignedToMe =
                (userId && s.assignedReferentUserId === userId) ||
                Boolean(userEmail && s.assignedReferentEmail?.toLowerCase() === userEmail);
              const conventions = s.conventions.filter((conv) => {
                const refEmail = String(conv.teacherReferentEmail || "")
                  .trim()
                  .toLowerCase();
                if (userEmail && refEmail && refEmail === userEmail) return true;
                if (userId && conv.teacherReferentUserId === userId) return true;
                return false;
              });
              if (!assignedToMe && conventions.length === 0) return null;
              const next = { ...s, conventions: assignedToMe ? s.conventions : conventions };
              const hasValide = next.conventions.some((c) => c.status === "signed");
              const hasEnCours = next.conventions.some(
                (c) =>
                  c.status === "signatures_pending" ||
                  c.status === "convention_ready" ||
                  c.status === "admin_review" ||
                  c.status === "preconvention_submitted" ||
                  c.status === "convention_deposited",
              );
              const rosterStatus =
                next.conventions.length === 0
                  ? ("sans_stage" as const)
                  : next.conventions.length > 1
                    ? ("plusieurs" as const)
                    : hasValide && !hasEnCours
                      ? ("valide" as const)
                      : hasEnCours
                        ? ("en_cours" as const)
                        : ("sans_stage" as const);
              return { ...next, rosterStatus };
            })
            .filter((s): s is NonNullable<typeof s> => s !== null);

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
          ? "Vue référent : uniquement les stagiaires qui vous sont affectés (suivi, sans signature de la convention)."
          : roster.note,
    };

    const { loadElevePhotoIndex, resolveElevePhotoS3Key } = await import(
      "@/app/lib/eleve-photos"
    );
    const photoIndex = await loadElevePhotoIndex().catch(
      (): Awaited<ReturnType<typeof loadElevePhotoIndex>> => ({}),
    );
    const rosterWithPhotos = {
      ...scopedRoster,
      students: scopedRoster.students.map((s) => {
        if (!s.eleveId) return { ...s, photoUrl: null as string | null };
        const key = resolveElevePhotoS3Key(photoIndex, {
          nom: s.nom,
          prenom: s.prenom,
          ine: s.ine,
          photoKey: s.photoKey,
        });
        return {
          ...s,
          photoUrl: key
            ? `/api/stages/eleve-photo?eleveId=${encodeURIComponent(s.eleveId)}`
            : null,
        };
      }),
    };
    perf.mark("photos");

    const { isValkeyConfigured, getValkey } = await import("@/app/lib/valkey");
    const vk = getValkey();

    return NextResponse.json({
      schoolYear,
      availableClasses,
      referents: assignments.map((a) => ({
        name: a.name,
        email: a.email,
        role: a.role,
        externalUserId: a.externalUserId,
      })),
      canAssignReferent,
      teachers,
      roster: rosterWithPhotos,
      viewerScope: canBrowseAll
        ? "all"
        : isPrincipalForClass
          ? "principal"
          : "referent",
      cache: {
        valkey: {
          configured: isValkeyConfigured(),
          ready: vk?.status === "ready",
          status: vk?.status ?? (isValkeyConfigured() ? "connecting" : "absent"),
        },
      },
      perf: {
        ...perf.snapshot(),
        className,
        students: scopedStudents.length,
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
