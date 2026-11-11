import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import { getBienEtreConfig } from "@/app/lib/bien-etre-config";
import { requireEleveAuth } from "@/app/lib/bien-etre-auth";
import { checkAndIncrementSignalementRate } from "@/app/lib/bien-etre-rate-limit";
import { notifyPsychologistSignalement } from "@/app/lib/bien-etre-notify";
import { severityFromDistress } from "@/app/lib/bien-etre-prompt";
import {
  bienEtreSessionCookieName,
  clearBienEtreSessionCookieOptions,
  openBienEtreSession,
} from "@/app/lib/bien-etre-session";
import {
  createSignalement,
  newSignalementId,
  summarizeConversationForSignalement,
} from "@/app/lib/bien-etre-storage";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { BIEN_ETRE_IA_ACTIVE } from "@/app/lib/ia-donnees-eleves-flags";
import type { BienEtreSignalement } from "@/app/lib/bien-etre-types";

export const runtime = "nodejs";

type SignalerBody = {
  prenom?: string;
  classe?: string;
  complement?: string;
};

export async function POST(req: Request) {
  const gate = await requireEleveAuth();
  if (!gate.ok) return gate.response;

  const config = await getBienEtreConfig();
  if (!config.enabled) {
    return NextResponse.json({ error: "Le bot bien-être n'est pas activé." }, { status: 503 });
  }
  if (!config.psychologistEmail.trim()) {
    return NextResponse.json({ error: "Signalement indisponible (psychologue non configuré)." }, { status: 503 });
  }

  const rate = await checkAndIncrementSignalementRate(gate.ctx.userId);
  if (!rate.ok) {
    return NextResponse.json({ error: rate.error }, { status: 429 });
  }

  const body = (await req.json()) as SignalerBody;
  const prenom = String(body.prenom || "").trim();
  if (!prenom || prenom.length < 2) {
    return NextResponse.json({ error: "Indique ton prénom pour le signalement." }, { status: 400 });
  }

  const jar = await cookies();
  const session = openBienEtreSession(jar.get(bienEtreSessionCookieName())?.value);
  const messages = session?.messages ?? [];
  const apiKey = BIEN_ETRE_IA_ACTIVE ? await getMistralApiKey() : null; // NJ-03 : pas de résumé par IA externe
  const summary = apiKey
    ? await summarizeConversationForSignalement(messages, apiKey)
    : "Signalement initié par l'élève.";

  const distress = session?.analysis?.distressLevel ?? 5;
  const now = new Date();
  const retentionMs = config.retentionDays * 24 * 60 * 60 * 1000;

  const record: BienEtreSignalement = {
    id: newSignalementId(),
    createdAt: now.toISOString(),
    prenom,
    classe: String(body.classe || "").trim() || undefined,
    complement: String(body.complement || "").trim() || undefined,
    summary,
    categories: session?.analysis?.categories?.length
      ? session.analysis.categories
      : ["autre"],
    severity: severityFromDistress(distress),
    status: "nouveau",
    retentionExpiresAt: new Date(now.getTime() + retentionMs).toISOString(),
  };

  await createSignalement(record);

  try {
    await notifyPsychologistSignalement(config, record);
  } catch (e) {
    console.error("[bien-etre/signaler] mail", e);
    return NextResponse.json(
      { error: "Signalement enregistré mais l'e-mail n'a pas pu être envoyé. Préviens un adulte de confiance." },
      { status: 502 },
    );
  }

  const res = NextResponse.json({
    ok: true,
    message:
      "Ton signalement a été transmis au psychologue de l'établissement. Tu peux être recontacté·e. Prends soin de toi.",
  });
  res.cookies.set(clearBienEtreSessionCookieOptions());
  return res;
}
