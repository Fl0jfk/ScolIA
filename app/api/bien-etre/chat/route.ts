import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import { getBienEtreConfig } from "@/app/lib/bien-etre-config";
import { requireEleveAuth } from "@/app/lib/bien-etre-auth";
import { checkAndIncrementMessageRate } from "@/app/lib/bien-etre-rate-limit";
import { mistralChatText, mistralJsonObject } from "@/app/lib/bien-etre-mistral";
import {
  analysisFromClassifier,
  bienEtreSystemPrompt,
  BIEN_ETRE_OFF_TOPIC_REPLY,
  classifierPrompt,
  isOffTopicUserMessage,
} from "@/app/lib/bien-etre-prompt";
import {
  bienEtreSessionCookieName,
  bienEtreSessionCookieOptions,
  emptyBienEtreSession,
  openBienEtreSession,
  sealBienEtreSession,
} from "@/app/lib/bien-etre-session";
import { getMistralApiKey } from "@/app/lib/tenant-config";

export const runtime = "nodejs";

type ChatBody = {
  message?: string;
  reset?: boolean;
};

export async function POST(req: Request) {
  const gate = await requireEleveAuth();
  if (!gate.ok) return gate.response;

  const config = await getBienEtreConfig();
  if (!config.enabled) {
    return NextResponse.json({ error: "Le bot bien-être n'est pas activé." }, { status: 503 });
  }

  const rate = await checkAndIncrementMessageRate(gate.ctx.userId);
  if (!rate.ok) {
    return NextResponse.json({ error: rate.error }, { status: 429 });
  }

  const apiKey = await getMistralApiKey();
  if (!apiKey) {
    return NextResponse.json({ error: "Service IA non configuré." }, { status: 503 });
  }

  const body = (await req.json()) as ChatBody;
  const jar = await cookies();
  const cookieName = bienEtreSessionCookieName();

  if (body.reset) {
    const sealed = sealBienEtreSession(emptyBienEtreSession());
    const res = NextResponse.json({
      answer: config.welcomeMessage || "Bonjour, comment te sens-tu ?",
      suggestSignalement: false,
      messages: [],
    });
    res.cookies.set(bienEtreSessionCookieOptions(sealed));
    return res;
  }

  const message = String(body.message || "").trim();
  if (!message) {
    return NextResponse.json({ error: "Message requis." }, { status: 400 });
  }

  let session = openBienEtreSession(jar.get(cookieName)?.value) || emptyBienEtreSession();

  if (isOffTopicUserMessage(message)) {
    session = {
      messages: [
        ...session.messages,
        { role: "user", content: message },
        { role: "assistant", content: BIEN_ETRE_OFF_TOPIC_REPLY },
      ],
      analysis: {
        offTopic: true,
        distressLevel: session.analysis?.distressLevel ?? 0,
        suggestSignalement: session.analysis?.suggestSignalement ?? false,
        categories: session.analysis?.categories ?? [],
      },
      updatedAt: new Date().toISOString(),
    };
    const sealed = sealBienEtreSession(session);
    const res = NextResponse.json({
      answer: BIEN_ETRE_OFF_TOPIC_REPLY,
      suggestSignalement: session.analysis?.suggestSignalement ?? false,
      messages: session.messages,
    });
    res.cookies.set(bienEtreSessionCookieOptions(sealed));
    return res;
  }

  const history = session.messages.slice(-12);
  const mistralMessages: Array<{ role: "system" | "user" | "assistant"; content: string }> = [
    { role: "system", content: bienEtreSystemPrompt() },
    ...history.map((m) => ({ role: m.role, content: m.content })),
    { role: "user", content: message },
  ];

  const answer =
    (await mistralChatText(apiKey, mistralMessages, 0.45)) ||
    "Je suis là pour t'écouter. Peux-tu me dire un peu plus ?";

  const classified = await mistralJsonObject<Record<string, unknown>>(
    apiKey,
    classifierPrompt(message, answer),
  );
  const analysis = analysisFromClassifier(classified);

  session = {
    messages: [...session.messages, { role: "user", content: message }, { role: "assistant", content: answer }],
    analysis,
    updatedAt: new Date().toISOString(),
  };

  const sealed = sealBienEtreSession(session);
  const res = NextResponse.json({
    answer,
    suggestSignalement: analysis.suggestSignalement,
    distressLevel: analysis.distressLevel,
    messages: session.messages,
  });
  res.cookies.set(bienEtreSessionCookieOptions(sealed));
  return res;
}

export async function GET() {
  const gate = await requireEleveAuth();
  if (!gate.ok) return gate.response;

  const config = await getBienEtreConfig();
  const jar = await cookies();
  const session = openBienEtreSession(jar.get(bienEtreSessionCookieName())?.value);

  return NextResponse.json({
    enabled: config.enabled,
    welcomeMessage: config.welcomeMessage,
    messages: session?.messages ?? [],
    suggestSignalement: session?.analysis?.suggestSignalement ?? false,
  });
}
