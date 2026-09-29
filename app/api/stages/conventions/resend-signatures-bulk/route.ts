import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import { notifyAllStageSignatureRequests } from "@/app/lib/stage-notify";
import {
  conventionMatchesStageSecteurs,
  inferStageSecteurFromClass,
  resolveStageViewerSecteurs,
} from "@/app/lib/stage-sector-scope";
import { getConventionsIndex, getStageConvention } from "@/app/lib/stage-storage";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";

const SECTEUR_VALUES = new Set<Secteur>(["ecole", "college", "lycee"]);

/**
 * Relance groupée : envoie un e-mail à tous les signataires encore en attente
 * pour toutes les conventions en statut `signatures_pending` (périmètre
 * établissement / secteur du viewer).
 */
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

    let body: { secteur?: string; className?: string } = {};
    try {
      body = (await req.json()) as { secteur?: string; className?: string };
    } catch {
      body = {};
    }

    const filterSecteurRaw = String(body.secteur ?? "")
      .trim()
      .toLowerCase();
    const filterSecteur =
      filterSecteurRaw && SECTEUR_VALUES.has(filterSecteurRaw as Secteur)
        ? (filterSecteurRaw as Secteur)
        : null;
    const filterClassName = String(body.className ?? "").trim();

    const viewerSecteurs = await resolveStageViewerSecteurs(roles, gate.ctx.userId);
    const index = await getConventionsIndex();
    const loaded = await Promise.all(index.map((e) => getStageConvention(e.id)));

    let targets = loaded.filter(
      (c): c is NonNullable<typeof c> =>
        Boolean(c) && c!.status === "signatures_pending",
    );

    if (viewerSecteurs.length > 0) {
      targets = targets.filter((c) => conventionMatchesStageSecteurs(c, viewerSecteurs));
    }

    if (filterSecteur) {
      targets = targets.filter((c) => {
        const secteur = inferStageSecteurFromClass(c.student.className, c.student.level);
        return secteur === filterSecteur;
      });
    }

    if (filterClassName && filterClassName !== "all") {
      targets = targets.filter(
        (c) => String(c.student.className || "").trim() === filterClassName,
      );
    }

    let conventionsWithPending = 0;
    let sentCount = 0;
    let pendingTotal = 0;
    const failed: Array<{ conventionId: string; studentName: string; reason: string }> = [];

    for (const convention of targets) {
      const pending = convention.signatures.filter((s) => s.status === "en_attente");
      if (pending.length === 0) continue;
      conventionsWithPending += 1;
      pendingTotal += pending.length;

      const mail = await notifyAllStageSignatureRequests(convention);
      sentCount += mail.sentCount;

      if (mail.sentCount < mail.total) {
        const reasons = mail.results
          .filter((r) => !r.sent)
          .map((r) => r.reason || r.error || "échec")
          .join(", ");
        failed.push({
          conventionId: convention.id,
          studentName: `${convention.student.firstName} ${convention.student.lastName}`.trim(),
          reason: reasons || "envoi partiel",
        });
      }
    }

    return NextResponse.json({
      success: true,
      conventionsScanned: targets.length,
      conventionsWithPending,
      pendingTotal,
      sentCount,
      failedCount: failed.length,
      failed: failed.slice(0, 20),
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
