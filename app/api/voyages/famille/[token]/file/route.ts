import { NextResponse } from "next/server";
import { NO_STORE, jsonNoStore, withPublicContext } from "@/app/lib/travels-v2-public-api";
import { readFamilyFile } from "@/app/lib/travels-v2-registration-store";

type Params = { params: Promise<{ token: string }> };

/** Accusé (PDF signé) et fichier agenda (.ics) de CETTE invitation, après vérification du jeton. */
export async function GET(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withPublicContext(req, token);
  if (!g.ok) return g.response;
  const kind = new URL(req.url).searchParams.get("kind");
  if (kind !== "pdf" && kind !== "ics") return jsonNoStore({ error: "Requête invalide." }, 400);
  try {
    const file = await readFamilyFile(g.ctx, kind);
    if (!file) return jsonNoStore({ error: "Fichier introuvable." }, 404);
    const body = typeof file.bytes === "string" ? file.bytes : new Uint8Array(file.bytes);
    return new NextResponse(body, {
      status: 200,
      headers: {
        ...NO_STORE,
        "Content-Type": file.contentType,
        "Content-Disposition": `attachment; filename="${file.fileName}"`,
        "X-Content-Type-Options": "nosniff",
      },
    });
  } catch (e) {
    console.error("[voyages-famille] file", e);
    return jsonNoStore({ error: "Téléchargement impossible." }, 500);
  }
}
