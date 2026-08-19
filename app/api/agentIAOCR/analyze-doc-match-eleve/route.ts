import { NextResponse } from "next/server";
import { safeCurrentUser, resolveSession } from "@/app/lib/intranet-session";
import { requireAuth } from "@/app/lib/intranet-auth";
import { analyzeDocMatchEleve } from "@/app/lib/ocr-analyze-eleve";
import { analyzeDocForOcr } from "@/app/lib/ocr-analyze-unified";
import { ocrHasExtraFluxes } from "@/app/lib/ocr-flux";
import {
  resolveOcrCapabilitiesForClerkUserServer,
  resolveOneDriveProfileForClerkUserServer,
} from "@/app/lib/onedrive-user-profiles.server";

export const maxDuration = 120;

export async function POST(req: Request) {
  try {
    const session = await resolveSession();
    const userId = session?.userId;
    if (!userId) return NextResponse.json({ error: "Non autorisé" }, { status: 401 });

    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const like = user
      ? {
          id: user.id,
          lastName: user.lastName,
          emailAddresses: user.emailAddresses?.map((e) => ({ emailAddress: e.emailAddress })),
          primaryEmailAddress: user.primaryEmailAddress
            ? { emailAddress: user.primaryEmailAddress.emailAddress }
            : null,
        }
      : null;
    const caps = like ? await resolveOcrCapabilitiesForClerkUserServer(like) : null;
    const odProfile = caps?.primaryEleves ?? (like ? await resolveOneDriveProfileForClerkUserServer(like) : null);

    const { text } = await req.json();
    if (!text) return NextResponse.json({ error: "text requis" }, { status: 400 });

    const result = ocrHasExtraFluxes(caps)
      ? await analyzeDocForOcr(text, odProfile, caps)
      : await analyzeDocMatchEleve(text, odProfile);
    return NextResponse.json(result);
  } catch (error) {
    console.error("Erreur analyse Mistral:", error);
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
