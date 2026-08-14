import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canModerateOffers } from "@/app/lib/stage-access";
import { ensureOfferCandidatureToken } from "@/app/lib/stage-candidature";
import { getStageOffer, saveStageOffer } from "@/app/lib/stage-storage";
import type { StageOffer, StageOfferStatus } from "@/app/lib/stage-types";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Direction";
}

export async function GET(_req: Request, ctx: { params: Promise<{ id: string }> }) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;
    const { id } = await ctx.params;
    const offer = await getStageOffer(id);
    if (!offer) return NextResponse.json({ error: "Offre introuvable." }, { status: 404 });
    return NextResponse.json({ offer });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function PATCH(req: Request, ctx: { params: Promise<{ id: string }> }) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canModerateOffers(roles)) {
      return NextResponse.json({ error: "Réservé à la direction." }, { status: 403 });
    }

    const { id } = await ctx.params;
    const offer = await getStageOffer(id);
    if (!offer) return NextResponse.json({ error: "Offre introuvable." }, { status: 404 });

    const body = await req.json();
    const status = body.status as StageOfferStatus;
    if (!["approved", "rejected", "filled", "archived"].includes(status)) {
      return NextResponse.json({ error: "Statut invalide." }, { status: 400 });
    }

    const now = new Date().toISOString();
    let next: StageOffer = {
      ...offer,
      status,
      reviewedAt: now,
      reviewedBy: displayName(user),
      reviewNote: String(body.reviewNote ?? "").trim() || undefined,
      updatedAt: now,
    };
    if (status === "approved") {
      next = await ensureOfferCandidatureToken(next);
    }
    await saveStageOffer(next);
    return NextResponse.json({
      success: true,
      offer: next,
      candidatureLink: next.candidatureToken
        ? `/stages/candidater?token=${encodeURIComponent(next.candidatureToken)}`
        : null,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
