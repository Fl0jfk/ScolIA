import { NextResponse } from "next/server";
import {
  applyMatchAcceptance,
  applyMatchDecline,
  otherPartyId,
} from "@/app/lib/covoiturage-matching";
import { notifyCovoiturageContactRevealed } from "@/app/lib/covoiturage-notify";
import {
  getCovoiturageMatches,
  getCovoiturageProfiles,
  saveCovoiturageMatches,
} from "@/app/lib/covoiturage-storage";
import { requireAuth } from "@/app/lib/intranet-auth";
import { isCovoiturageToolEnabled } from "@/app/lib/toolbox-config";

type RouteCtx = { params: Promise<{ id: string }> };

export async function POST(req: Request, ctx: RouteCtx) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  if (!(await isCovoiturageToolEnabled())) {
    return NextResponse.json(
      { error: "Le covoiturage n’est pas activé pour cet établissement." },
      { status: 403 },
    );
  }

  const { id } = await ctx.params;
  const body = (await req.json()) as Record<string, unknown>;
  const action = String(body.action ?? "").trim();

  const matches = await getCovoiturageMatches();
  const idx = matches.findIndex((m) => m.id === id);
  if (idx < 0) return NextResponse.json({ error: "Match introuvable." }, { status: 404 });

  const match = matches[idx]!;
  if (match.profileA !== userId && match.profileB !== userId) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  if (match.status !== "pending") {
    return NextResponse.json({ error: "Ce match n'est plus actif." }, { status: 400 });
  }

  if (action === "accept") {
    const updated = applyMatchAcceptance(match, userId);
    matches[idx] = updated;
    await saveCovoiturageMatches(matches);

    if (updated.status === "revealed") {
      const profiles = await getCovoiturageProfiles();
      const profileA = profiles.find((p) => p.externalUserId === updated.profileA);
      const profileB = profiles.find((p) => p.externalUserId === updated.profileB);
      if (profileA && profileB) {
        await Promise.all([
          notifyCovoiturageContactRevealed({ profile: profileA, other: profileB, match: updated }).catch(() => {}),
          notifyCovoiturageContactRevealed({ profile: profileB, other: profileA, match: updated }).catch(() => {}),
        ]);
      }
    }

    return NextResponse.json({ ok: true, match: updated });
  }

  if (action === "decline") {
    matches[idx] = applyMatchDecline(match, userId);
    await saveCovoiturageMatches(matches);
    return NextResponse.json({ ok: true, match: matches[idx] });
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}
