import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  getOnboardingRecordByToken,
  submitOnboardingForm,
} from "@/app/lib/rh/onboarding-storage";
import {
  isOnboardingTokenUsable,
  normalizeRhOnboardingForm,
} from "@/app/lib/rh/onboarding-types";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const onboardingPublicLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 20,
});

export async function GET(req: Request) {
  const token = new URL(req.url).searchParams.get("token")?.trim();
  if (!token) {
    return NextResponse.json({ error: "Token manquant." }, { status: 400 });
  }

  const record = await getOnboardingRecordByToken(token);
  if (!record) {
    return NextResponse.json({ error: "Lien invalide ou expiré." }, { status: 404 });
  }

  const config = await loadAppConfig();
  const usable = isOnboardingTokenUsable(record);

  return NextResponse.json({
    schoolName: config.identity.name || config.identity.shortName || "Établissement",
    status: record.status,
    usable,
    alreadySubmitted: record.status !== "awaiting_candidate",
    expiresAt: record.expiresAt,
  });
}

export async function POST(req: Request) {
  try {
    if (!onboardingPublicLimiter.allow(clientIpFromRequest(req))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const body = await req.json();
    const token = String(body?.token || "").trim();
    if (!token) return NextResponse.json({ error: "Token manquant." }, { status: 400 });

    const form = normalizeRhOnboardingForm(body?.form ?? body);
    if (!form) {
      return NextResponse.json(
        { error: "Formulaire incomplet ou invalide. Vérifiez les champs obligatoires." },
        { status: 400 },
      );
    }

    const record = await submitOnboardingForm(token, form);
    return NextResponse.json({ ok: true, status: record.status });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Envoi impossible." },
      { status: 400 },
    );
  }
}
