import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { listPublicSlotsForDirection } from "@/app/lib/rdv-inscription-service";

const slotsLimiter = createMemoryRateLimiter({
  windowMs: 5 * 60 * 1000,
  max: 60,
});

type Ctx = { params: Promise<{ direction: string }> };

/** Liste publique des créneaux libres pour une direction. */
export async function GET(req: Request, ctx: Ctx) {
  try {
    if (!(await slotsLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de requêtes. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const { direction } = await ctx.params;
    const slug = decodeURIComponent(direction || "").trim().toLowerCase();
    if (!slug) {
      return NextResponse.json({ error: "Direction requise." }, { status: 400 });
    }

    const result = await listPublicSlotsForDirection(slug);
    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    return NextResponse.json({
      title: result.configTitle,
      intro: result.intro,
      consentLabel: result.consentLabel,
      location: result.location,
      directionLabel: result.directionLabel,
      directriceDisplayName: result.directriceDisplayName,
      levels: result.levels,
      slots: result.slots.map((s) => ({
        eventId: s.eventId,
        startAt: s.startAt,
        endAt: s.endAt,
        title: s.title,
      })),
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
