import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { getPosterTemplateMeta, isPosterTemplateId } from "@/app/lib/posters";

export const runtime = "nodejs";

const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const brief = String(body.brief || "").trim();
    if (!brief) {
      return NextResponse.json({ error: "Brief vide" }, { status: 400 });
    }
    if (brief.length > 2000) {
      return NextResponse.json({ error: "Brief trop long" }, { status: 400 });
    }

    const templateId = body.templateId ? String(body.templateId) : "partenariat-sportif";
    const meta =
      (isPosterTemplateId(templateId) && getPosterTemplateMeta(templateId)) ||
      getPosterTemplateMeta("partenariat-sportif");
    const partnerName = String(body.partnerName || "").trim();

    const apiKey = await getMistralApiKey();
    if (!apiKey) {
      return NextResponse.json(
        { error: "Service IA non configuré (clé Mistral manquante)." },
        { status: 503 },
      );
    }

    const prompt = `Tu rédiges les textes d'une affiche scolaire française (${meta?.label || "partenariat"}).
Brief utilisateur :
${brief}
${partnerName ? `Nom du partenaire : ${partnerName}` : ""}

Réponds UNIQUEMENT en JSON valide :
{"title":"...","subtitle":"...","body":"..."}
Contraintes :
- title : max 8 mots, accroche claire
- subtitle : max 12 mots
- body : 1 à 3 phrases courtes, ton bienveillant, pas de jargon marketing agressif
- français correct, pas de guillemets inutiles dans les valeurs`;

    const res = await fetch(MISTRAL_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({
        model: "mistral-small-latest",
        temperature: 0.4,
        response_format: { type: "json_object" },
        messages: [{ role: "user", content: prompt }],
      }),
    });

    if (!res.ok) {
      return NextResponse.json({ error: "Suggestion IA indisponible" }, { status: 502 });
    }

    const data = await res.json();
    const raw = String(data?.choices?.[0]?.message?.content || "").trim();
    let parsed: { title?: string; subtitle?: string; body?: string } = {};
    try {
      parsed = JSON.parse(raw);
    } catch {
      return NextResponse.json({ error: "Réponse IA illisible" }, { status: 502 });
    }

    return NextResponse.json({
      success: true,
      title: String(parsed.title || "").trim().slice(0, 120),
      subtitle: String(parsed.subtitle || "").trim().slice(0, 160),
      body: String(parsed.body || "").trim().slice(0, 800),
    });
  } catch (e) {
    console.error("[posters/suggest-copy]", e);
    const msg = e instanceof Error ? e.message : "Erreur";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
