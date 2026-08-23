import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canCreateOffer, canModerateOffers } from "@/app/lib/stage-access";
import { getOffersIndex, getStageOffer, saveStageOffer } from "@/app/lib/stage-storage";
import {
  currentStageSchoolYear,
  stageUid,
  type StageOffer,
  type StageOfferKind,
} from "@/app/lib/stage-types";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  const full = `${first} ${last}`.trim();
  return full || user?.primaryEmailAddress?.emailAddress?.split("@")[0] || "Parent";
}

function userEmail(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  return user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
}

function parseKind(raw: unknown): StageOfferKind {
  if (raw === "pfmp" || raw === "stage_observation" || raw === "job_ete" || raw === "autre") return raw;
  return "pfmp";
}

export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const index = await getOffersIndex();

    if (canModerateOffers(roles)) {
      const offers = await Promise.all(index.map((e) => getStageOffer(e.id)));
      return NextResponse.json({ offers: offers.filter(Boolean) });
    }

    if (canCreateOffer(roles)) {
      const mine = await Promise.all(
        index.filter(() => true).map(async (e) => {
          const o = await getStageOffer(e.id);
          if (!o || o.submittedBy.externalUserId !== gate.ctx.userId) return null;
          return o;
        }),
      );
      const approved = await Promise.all(
        index
          .filter((e) => e.status === "approved")
          .map((e) => getStageOffer(e.id)),
      );
      return NextResponse.json({
        myOffers: mine.filter(Boolean),
        approvedOffers: approved.filter(Boolean),
      });
    }

    const approved = await Promise.all(
      index.filter((e) => e.status === "approved").map((e) => getStageOffer(e.id)),
    );
    return NextResponse.json({ approvedOffers: approved.filter(Boolean) });
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
    if (!canCreateOffer(roles)) {
      return NextResponse.json({ error: "Réservé aux parents." }, { status: 403 });
    }

    const body = await req.json();
    const now = new Date().toISOString();
    const levels = Array.isArray(body.targetLevels)
      ? body.targetLevels.map(String).filter(Boolean)
      : [];

    const offer: StageOffer = {
      id: stageUid("offer"),
      kind: parseKind(body.kind),
      status: "pending",
      schoolYear: currentStageSchoolYear(),
      submittedBy: {
        externalUserId: gate.ctx.userId,
        displayName: displayName(user),
        email: userEmail(user),
      },
      companyName: String(body.companyName ?? "").trim(),
      companyAddress: String(body.companyAddress ?? "").trim() || undefined,
      companySiret: String(body.companySiret ?? "").trim() || undefined,
      sector: String(body.sector ?? "").trim() || undefined,
      description: String(body.description ?? "").trim(),
      positionsCount: Math.max(1, Number(body.positionsCount) || 1),
      targetLevels: levels.length ? levels : ["3e", "2de"],
      periodStart: String(body.periodStart ?? "").slice(0, 10) || undefined,
      periodEnd: String(body.periodEnd ?? "").slice(0, 10) || undefined,
      contactName: String(body.contactName ?? "").trim(),
      contactEmail: String(body.contactEmail ?? "").trim().toLowerCase(),
      contactPhone: String(body.contactPhone ?? "").trim() || undefined,
      createdAt: now,
      updatedAt: now,
    };

    if (!offer.companyName || !offer.description || !offer.contactName || !offer.contactEmail) {
      return NextResponse.json({ error: "Champs obligatoires manquants." }, { status: 400 });
    }

    await saveStageOffer(offer);
    return NextResponse.json({ success: true, offer });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
