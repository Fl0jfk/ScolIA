import { NextResponse } from "next/server";
import { loadCampaignConfig, listParentWishes } from "@/app/lib/class-allocation-storage";
import {
  parentSessionCookieOptions,
  sealParentSession,
  verifyParentAuthCode,
} from "@/app/lib/class-allocation-parent-auth";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import {
  findElevesByParentEmail,
  isValidParentEmail,
  normalizeParentEmail,
  toParentLinkedChildren,
} from "@/app/lib/eleves-parent-emails";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const parentVerifyLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 20,
});

export async function POST(req: Request) {
  if (!(await parentVerifyLimiter.allow(clientIpFromRequest(req)))) {
    return NextResponse.json(
      { error: "Trop de tentatives. Réessayez dans quelques minutes." },
      { status: 429 },
    );
  }
  const campaign = await loadCampaignConfig();
  if (!campaign.isOpen) {
    return NextResponse.json({ error: "La campagne est fermée." }, { status: 400 });
  }

  let body: { email?: string; code?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const email = normalizeParentEmail(String(body.email || ""));
  const code = String(body.code || "").trim();
  if (!isValidParentEmail(email)) {
    return NextResponse.json({ error: "Adresse e-mail invalide." }, { status: 400 });
  }

  const verified = await verifyParentAuthCode({ campaignId: campaign.id, email, code });
  if (!verified.ok) {
    return NextResponse.json({ error: verified.error }, { status: 400 });
  }

  const eleves = await loadElevesRegistry();
  const linked = findElevesByParentEmail(eleves, email);
  if (!linked.length) {
    return NextResponse.json(
      { error: "Aucun élève associé à cette adresse. Contactez le secrétariat." },
      { status: 404 },
    );
  }

  const childInes = linked.map((e) => e.ine).filter(Boolean);
  const sealed = sealParentSession({
    email,
    campaignId: campaign.id,
    childInes,
    verifiedAt: new Date().toISOString(),
  });

  const wishes = await listParentWishes(campaign.id);
  const submittedInes = new Set(wishes.map((w) => w.studentIne));

  const res = NextResponse.json({
    ok: true,
    email,
    children: toParentLinkedChildren(linked).map((c) => ({
      ...c,
      wishSubmitted: submittedInes.has(c.ine),
    })),
  });
  res.cookies.set(parentSessionCookieOptions(sealed));
  return res;
}
