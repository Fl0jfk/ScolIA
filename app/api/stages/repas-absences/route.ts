import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention, canViewAllConventions } from "@/app/lib/stage-access";
import { expandStagePresenceDates } from "@/app/lib/stage-schedule";
import {
  getStageWatchersConfig,
  listWatcherAssignmentsForUser,
  conventionMatchesWatcherAssignments,
} from "@/app/lib/stage-watchers-config";
import {
  conventionMatchesStageSecteurs,
  resolveStageViewerSecteurs,
} from "@/app/lib/stage-sector-scope";
import { getConventionsIndex, getStageConvention } from "@/app/lib/stage-storage";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

/**
 * Jours d'absence stage (repas) — pour CPE / restauration / administratif.
 * Query: ?from=YYYY-MM-DD&to=YYYY-MM-DD (défaut: 14 jours à venir)
 */
export async function GET(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const year = currentStageSchoolYear();
    const watchers = await getStageWatchersConfig(year);
    const myWatch = listWatcherAssignmentsForUser(watchers, gate.ctx.userId);
    const restaurationWatch = myWatch.filter((a) => a.kind === "restauration");
    const cpeWatch = myWatch.filter((a) => a.kind === "cpe");

    const allowed =
      canReviewPreconvention(roles) ||
      canViewAllConventions(roles) ||
      restaurationWatch.length > 0 ||
      cpeWatch.length > 0 ||
      roles.includes("cpe");
    if (!allowed) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const { searchParams } = new URL(req.url);
    const today = new Date();
    const defaultFrom = today.toISOString().slice(0, 10);
    const end = new Date(today);
    end.setDate(end.getDate() + 14);
    const from = searchParams.get("from")?.trim() || defaultFrom;
    const to = searchParams.get("to")?.trim() || end.toISOString().slice(0, 10);

    const index = await getConventionsIndex();
    const all = await Promise.all(index.map((e) => getStageConvention(e.id)));
    const scopeAssignments =
      canReviewPreconvention(roles) || canViewAllConventions(roles)
        ? null
        : [...restaurationWatch, ...cpeWatch];
    const viewerSecteurs = await resolveStageViewerSecteurs(roles, gate.ctx.userId);

    type Row = {
      conventionId: string;
      studentName: string;
      className: string;
      companyName: string;
      date: string;
      status: string;
    };
    const rows: Row[] = [];

    for (const c of all) {
      if (!c) continue;
      if (c.status !== "signatures_pending" && c.status !== "signed" && c.status !== "convention_ready") {
        continue;
      }
      if (scopeAssignments && !conventionMatchesWatcherAssignments(c, scopeAssignments)) {
        continue;
      }
      if (!conventionMatchesStageSecteurs(c, viewerSecteurs)) {
        continue;
      }
      const dates = expandStagePresenceDates(c.schedule).filter((d) => d >= from && d <= to);
      for (const date of dates) {
        rows.push({
          conventionId: c.id,
          studentName: `${c.student.firstName} ${c.student.lastName}`.trim(),
          className: c.student.className,
          companyName: c.company.name,
          date,
          status: c.status,
        });
      }
    }

    rows.sort((a, b) => a.date.localeCompare(b.date) || a.studentName.localeCompare(b.studentName, "fr"));

    const byDate = new Map<string, Row[]>();
    for (const row of rows) {
      const list = byDate.get(row.date) ?? [];
      list.push(row);
      byDate.set(row.date, list);
    }

    return NextResponse.json({
      from,
      to,
      totalAbsences: rows.length,
      days: [...byDate.entries()].map(([date, absences]) => ({ date, absences })),
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
