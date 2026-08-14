import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { loadCampaignConfig } from "@/app/lib/class-allocation-storage";
import {
  issueParentAuthCode,
  sendParentAuthCodeEmail,
} from "@/app/lib/class-allocation-parent-auth";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import {
  findElevesByParentEmail,
  isValidParentEmail,
  normalizeParentEmail,
} from "@/app/lib/eleves-parent-emails";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const parentAuthRequestLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 10,
});

const GENERIC_OK =
  "Si cette adresse est enregistrée comme responsable légal, vous recevrez un code par e-mail dans quelques instants.";

export async function POST(req: Request) {
  if (!parentAuthRequestLimiter.allow(clientIpFromRequest(req))) {
    return NextResponse.json(
      { error: "Trop de tentatives. Réessayez dans quelques minutes." },
      { status: 429 },
    );
  }

  const campaign = await loadCampaignConfig();
  if (!campaign.isOpen) {
    return NextResponse.json({ error: "La campagne est fermée." }, { status: 400 });
  }

  let body: { email?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const email = normalizeParentEmail(String(body.email || ""));
  if (!isValidParentEmail(email)) {
    return NextResponse.json({ error: "Adresse e-mail invalide." }, { status: 400 });
  }

  const [eleves, app] = await Promise.all([loadElevesRegistry(), loadAppConfig()]);
  const linked = findElevesByParentEmail(eleves, email);

  if (linked.length === 0) {
    return NextResponse.json({ ok: true, message: GENERIC_OK });
  }

  const issued = await issueParentAuthCode({ campaignId: campaign.id, email });
  if (!issued.ok) {
    return NextResponse.json(
      { error: `Patientez ${issued.retryAfterSec} s avant de redemander un code.` },
      { status: 429 },
    );
  }

  const schoolName = app.identity.shortName || app.identity.name || "Établissement";
  const sent = await sendParentAuthCodeEmail({
    to: email,
    code: issued.code,
    schoolName,
    campaignLabel: campaign.label,
  });

  if (!sent) {
    return NextResponse.json(
      {
        error:
          "Envoi du code impossible (messagerie non configurée). Contactez le secrétariat de l'établissement.",
      },
      { status: 503 },
    );
  }

  return NextResponse.json({ ok: true, message: GENERIC_OK });
}
