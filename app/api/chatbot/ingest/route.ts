import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";

import { appendEntryToKnowledgeFile, readKnowledgeIndex, selectDomainByMessage } from "@/app/lib/knowledge";
import { getMistralApiKey } from "@/app/lib/tenant-config";

export const runtime = "nodejs";

type IngestRequest = {
  text?: string;
  source?: string;
  audience?: "public" | "private" | "both";
};

type LlmUpdate = {
  domainId?: unknown;
  title?: unknown;
  content?: unknown;
};

export async function POST(req: Request) {
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const session = await resolveSession();
    const userId = session?.userId;
    if (!userId) {  return NextResponse.json({ error: "Non autorisé" }, { status: 401 })}
    const body = (await req.json()) as IngestRequest;
    const text = (body.text ?? "").trim();
    if (!text) { return NextResponse.json({ error: "text requis" }, { status: 400 })}
    const index = await readKnowledgeIndex();
    const chosenDomain = selectDomainByMessage(index.domains, text);
    const domainsById = new Map(index.domains.map((d) => [d.id, d]));
    const audience: Array<"public" | "private"> = body.audience === "public" ? ["public"] : body.audience === "private" ? ["private"] : ["public", "private"];
    let title = `Ajout ${new Date().toLocaleDateString("fr-FR")}`;
    let content = text.slice(0, 1800);
    let updatesToApply: Array<{ domainId: string; title: string; content: string }> = [];

    const mistralKey = await getMistralApiKey();
    if (mistralKey) {
      const domainsDescription = index.domains
        .map((d) => `- ${d.id}: ${d.label}`)
        .join("\n");
      const prompt =
        `Tu dois analyser un texte OCR et le découper en plusieurs fiches de connaissance.\n` +
        `Chaque fiche doit être rangée dans le BON domaine, pas tout dans un seul domaine.\n` +
        `Domaine possibles (domainId exact obligatoire):\n${domainsDescription}\n\n` +
        `Règles:\n` +
        `1) Crée 0..12 fiches utiles max.\n` +
        `2) Ignore le bruit administratif sans valeur informative.\n` +
        `3) UNE fiche = UN fait/événement/action atomique (ne mélange pas plusieurs cas dans une seule fiche).\n` +
        `4) Si le texte contient plusieurs sujets, répartis-les sur plusieurs domaines.\n` +
        `5) Si une heure/date est visible (ex: 13h, 14:00, 10 avril), elle doit être conservée dans title ou content.\n` +
        `6) Le contenu de chaque fiche doit être court, factuel, clair.\n` +
        `Réponds strictement en JSON au format:\n` +
        `{"updates":[{"domainId":"...","title":"...","content":"..."}]}\n\n` +
        `Texte OCR:\n${text}`;
      const llm = await fetch("https://api.mistral.ai/v1/chat/completions", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Authorization": `Bearer ${mistralKey}`,
        },
        body: JSON.stringify({
          model: "mistral-small-2603",
          temperature: 0,
          response_format: { type: "json_object" },
          messages: [{ role: "user", content: prompt }],
        }),
      });
      if (llm.ok) {
        const data = await llm.json();
        const parsed = JSON.parse(data?.choices?.[0]?.message?.content ?? "{}") as { updates?: LlmUpdate[]; title?: unknown; content?: unknown };
        const updates = Array.isArray(parsed.updates) ? parsed.updates : [];

        updatesToApply = updates
          .map((u) => {
            const domainIdRaw = typeof u.domainId === "string" ? u.domainId.trim() : "";
            const titleRaw = typeof u.title === "string" ? u.title.trim() : u.title != null ? String(u.title).trim() : "";
            const contentRaw =
              typeof u.content === "string"
                ? u.content.trim()
                : u.content != null
                  ? JSON.stringify(u.content).trim()
                  : "";
            return { domainIdRaw, titleRaw, contentRaw };
          })
          .filter((u) => u.domainIdRaw && u.contentRaw.length >= 20 && domainsById.has(u.domainIdRaw))
          .map((u) => ({
            domainId: u.domainIdRaw,
            title: u.titleRaw || `Ajout ${new Date().toLocaleDateString("fr-FR")}`,
            content: u.contentRaw,
          }));

        // Fallback compatibilité: si Mistral renvoie encore un format simple title/content
        if (updatesToApply.length === 0) {
          const nextTitle = typeof parsed.title === "string" ? parsed.title : parsed.title != null ? String(parsed.title) : title;
          const nextContent =
            typeof parsed.content === "string"
              ? parsed.content
              : parsed.content != null
                ? JSON.stringify(parsed.content)
                : content;
          title = nextTitle.trim() || title;
          content = nextContent.trim() || content;
        }
      }
    }

    const source = body.source?.trim() || "Upload admin OCR";
    if (updatesToApply.length > 0) {
      for (const update of updatesToApply) {
        const domain = domainsById.get(update.domainId);
        if (!domain) continue;
        await appendEntryToKnowledgeFile(domain.file, {
          title: update.title,
          content: update.content,
          source,
          audiences: audience,
        });
      }
    } else {
      await appendEntryToKnowledgeFile(chosenDomain.file, {
        title,
        content,
        source,
        audiences: audience,
      });
      updatesToApply = [{ domainId: chosenDomain.id, title, content }];
    }

    return NextResponse.json({
      ok: true,
      updates: updatesToApply.map((u) => ({
        domain: u.domainId,
        file: domainsById.get(u.domainId)?.file,
        title: u.title,
        contentPreview: u.content.slice(0, 320),
      })),
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}