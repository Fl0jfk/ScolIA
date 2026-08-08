import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { getTemplateMeta, isDocumentTemplateId } from "@/app/lib/document-templates";

export const runtime = "nodejs";

const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const text = String(body.text || "").trim();
    if (!text) {
      return NextResponse.json({ error: "Texte vide" }, { status: 400 });
    }
    if (text.length > 6000) {
      return NextResponse.json({ error: "Texte trop long (max 6000 caractères)" }, { status: 400 });
    }

    const templateId = body.templateId ? String(body.templateId) : "";
    const fieldKey = body.fieldKey ? String(body.fieldKey) : "";
    const tone = String(body.tone || "professionnel").trim();

    let fieldHint = "paragraphe administratif scolaire";
    if (templateId && isDocumentTemplateId(templateId)) {
      const meta = getTemplateMeta(templateId);
      const field = meta?.fields.find((f) => f.key === fieldKey);
      if (field) fieldHint = `champ « ${field.label} » du modèle « ${meta!.label} »`;
    }

    const apiKey = await getMistralApiKey();
    if (!apiKey) {
      return NextResponse.json(
        { error: "Service IA non configuré (clé Mistral manquante)." },
        { status: 503 },
      );
    }

    const prompt = `Tu es assistant de rédaction pour un établissement scolaire français (intranet Scola).
Reformule le texte suivant pour le ${fieldHint}.
Contraintes :
- français correct, ton ${tone}, clair et bienveillant
- ne change pas les faits, noms, dates ni chiffres
- ne rajoute pas de formules juridiques inventées
- garde une longueur proche (max +20 %)
- réponds UNIQUEMENT avec le texte reformulé, sans guillemets ni préambule

Texte :
${text}`;

    const res = await fetch(MISTRAL_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({
        model: "mistral-small-latest",
        temperature: 0.35,
        messages: [{ role: "user", content: prompt }],
      }),
    });

    if (!res.ok) {
      const errText = await res.text().catch(() => "");
      console.error("[document-templates/reformulate] mistral", res.status, errText.slice(0, 200));
      return NextResponse.json({ error: "Reformulation IA indisponible" }, { status: 502 });
    }

    const data = await res.json();
    const reformulated = String(data?.choices?.[0]?.message?.content || "")
      .trim()
      .replace(/^["«]|["»]$/g, "");

    if (!reformulated) {
      return NextResponse.json({ error: "Réponse IA vide" }, { status: 502 });
    }

    return NextResponse.json({ success: true, text: reformulated });
  } catch (e) {
    console.error("[document-templates/reformulate]", e);
    const msg = e instanceof Error ? e.message : "Erreur";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
