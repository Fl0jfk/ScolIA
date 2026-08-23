import { NextResponse } from "next/server";
import { getToolboxConfig } from "@/app/lib/toolbox-config";
import { buildSuppliesListPdf } from "@/app/lib/fournitures-supplies-pdf";
import { parseSuppliesPdfRequest } from "@/app/lib/fournitures-print-payload";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const suppliesPdfLimiter = createMemoryRateLimiter({
  windowMs: 60 * 60 * 1000,
  max: 20,
});

/** Génère le PDF liste de fournitures (JSON ou formulaire `payload`). */
export async function POST(req: Request) {
  try {
    if (!(await suppliesPdfLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez plus tard." },
        { status: 429 },
      );
    }

    const toolbox = await getToolboxConfig();
    if (!toolbox.tools["simulateur-fournitures"].enabled) {
      return NextResponse.json({ error: "Simulateur fournitures désactivé." }, { status: 404 });
    }

    const { children, suppliesByChild } = await parseSuppliesPdfRequest(req);
    const { buffer, filename } = await buildSuppliesListPdf({ children, suppliesByChild });

    return new NextResponse(new Uint8Array(buffer), {
      status: 200,
      headers: {
        "Content-Type": "application/pdf",
        "Content-Disposition": `inline; filename="${filename}"`,
        "Cache-Control": "no-store",
      },
    });
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    if (message === "Ajoutez au moins un enfant." || message === "Payload manquant.") {
      return NextResponse.json({ error: message }, { status: 400 });
    }
    console.error("supplies/pdf error:", message);
    return NextResponse.json({ error: "Échec de la génération du PDF.", details: message }, { status: 500 });
  }
}
