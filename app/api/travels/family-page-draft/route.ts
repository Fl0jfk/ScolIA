import { NextResponse } from "next/server";
import { getJson } from "@/app/lib/s3-storage";
import { requireMistralApiKey } from "@/app/lib/tenant-config";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import {
  buildFamilyPageSystemPrompt,
  buildFamilyPageUserPrompt,
  ensureFinancialDifficultySentence,
  stripMarkdown,
} from "@/app/lib/travels-family-page";
import { isFamilyPageEditableStatus, isFamilyPageOpened, isWorkflowV2, textMentionsPrice } from "@/app/lib/travels-v2";
import type { TravelsTrip } from "@/app/lib/travels-types";

/**
 * Brouillon IA du texte de la page des familles (circuit v2 uniquement).
 * Lit le dossier côté serveur ; ne modifie rien (le texte est renvoyé, l'organisateur l'enregistre).
 */
export async function POST(req: Request) {
  try {
    const { tripId } = (await req.json().catch(() => ({}))) as { tripId?: string };
    if (!tripId || typeof tripId !== "string") {
      return NextResponse.json({ error: "Identifiant du séjour manquant." }, { status: 400 });
    }
    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Séjour introuvable." }, { status: 404 });
    if (!isWorkflowV2(trip)) {
      return NextResponse.json({ error: "Ce séjour n'utilise pas le circuit des familles." }, { status: 400 });
    }
    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });
    if (isFamilyPageOpened(trip.data.familyPage)) {
      return NextResponse.json({ error: "La page est ouverte : elle n'est plus modifiable." }, { status: 403 });
    }
    if (!isFamilyPageEditableStatus(String(trip.status))) {
      return NextResponse.json(
        { error: "La page des familles se rédige après la validation pédagogique." },
        { status: 400 },
      );
    }

    const mistralKey = await requireMistralApiKey();
    const system = buildFamilyPageSystemPrompt();
    const user = buildFamilyPageUserPrompt(trip);

    const callModel = async (extraReminder?: string): Promise<string | null> => {
      const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
        method: "POST",
        headers: { "Content-Type": "application/json", Authorization: `Bearer ${mistralKey}` },
        body: JSON.stringify({
          model: "mistral-small-2603",
          messages: [
            { role: "system", content: system },
            { role: "user", content: extraReminder ? `${user}\n\n${extraReminder}` : user },
          ],
          temperature: 0.5,
        }),
      });
      if (!res.ok) {
        console.error("[family-page-draft] Mistral", res.status, await res.text().catch(() => ""));
        return null;
      }
      const json = await res.json();
      const content = json?.choices?.[0]?.message?.content;
      return typeof content === "string" ? content : null;
    };

    let raw = await callModel();
    if (raw && textMentionsPrice(raw)) {
      raw = (await callModel("RAPPEL : ta réponse précédente mentionnait un montant. Réécris le texte sans aucun prix ni somme en euros.")) ?? raw;
    }
    if (!raw) return NextResponse.json({ error: "Échec de la génération (service IA)." }, { status: 502 });

    const text = ensureFinancialDifficultySentence(stripMarkdown(raw));
    return NextResponse.json({ text, mentionsPrice: textMentionsPrice(text) });
  } catch (error) {
    console.error("[family-page-draft]", error);
    return NextResponse.json({ error: "Échec de la génération." }, { status: 500 });
  }
}
