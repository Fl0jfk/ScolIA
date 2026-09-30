import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  getInvitationPageBySlug,
  sumOuiPresentCount,
} from "@/app/lib/invitation-db";
import type { InvitationPagePublic } from "@/app/lib/invitation-types";

type Ctx = { params: Promise<{ slug: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  try {
    const { slug } = await ctx.params;
    const page = await getInvitationPageBySlug(slug);
    if (!page || !page.enabled) {
      return NextResponse.json({ error: "Invitation introuvable." }, { status: 404 });
    }
    const used = await sumOuiPresentCount(page.id);
    const placesRemaining = Math.max(0, page.maxTotalPersons - used);
    const bundle = await loadAppConfig();
    const schoolName = bundle.identity.shortName || bundle.identity.name || "";
    const publicPage: InvitationPagePublic = {
      slug: page.slug,
      title: page.title,
      intro: page.intro,
      theme: page.theme,
      startsAt: page.startsAt,
      endsAt: page.endsAt,
      location: page.location,
      diplomaMode: page.diplomaMode,
      maxPersonsPerEleve: page.maxPersonsPerEleve,
      placesRemaining,
      schoolName,
    };
    return NextResponse.json({ page: publicPage });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
