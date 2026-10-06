import { cookies } from "next/headers";
import { NextResponse } from "next/server";
import { loadCampaignConfig, listParentWishes } from "@/app/lib/class-allocation-storage";
import { openParentSession, parentSessionCookieName } from "@/app/lib/class-allocation-parent-auth";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { findElevesByParentEmail, toParentLinkedChildren } from "@/app/lib/eleves-parent-emails";

export async function GET() {
  const campaign = await loadCampaignConfig();
  const jar = await cookies();
  const session = openParentSession(jar.get(parentSessionCookieName())?.value);
  if (!session || session.campaignId !== campaign.id) {
    return NextResponse.json({
      authenticated: false,
      campaign: {
        id: campaign.id,
        label: campaign.label,
        isOpen: campaign.isOpen,
      },
    });
  }

  const eleves = await loadElevesActifsRegistry();
  const linked = findElevesByParentEmail(eleves, session.email).filter((e) =>
    session.childInes.includes(e.ine),
  );
  const wishes = await listParentWishes(campaign.id);
  const submittedInes = new Set(wishes.map((w) => w.studentIne));

  return NextResponse.json({
    authenticated: true,
    email: session.email,
    children: toParentLinkedChildren(linked).map((c) => ({
      ...c,
      wishSubmitted: submittedInes.has(c.ine),
    })),
    campaign: {
      id: campaign.id,
      label: campaign.label,
      isOpen: campaign.isOpen,
    },
  });
}
