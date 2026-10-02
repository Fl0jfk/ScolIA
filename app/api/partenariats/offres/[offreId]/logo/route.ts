import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { getPartenariatOffreById, savePartenariatLogo } from "@/app/lib/partenariats-db";

export const runtime = "nodejs";
export const maxDuration = 60;

type Ctx = { params: Promise<{ offreId: string }> };

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { offreId } = await ctx.params;
    const offre = await getPartenariatOffreById(offreId);
    if (!offre) return NextResponse.json({ error: "Offre introuvable." }, { status: 404 });

    const formData = await req.formData();
    const file = formData.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier manquant" }, { status: 400 });
    }
    if (file.size > 8 * 1024 * 1024) {
      return NextResponse.json({ error: "Fichier trop volumineux (max 8 Mo)" }, { status: 400 });
    }
    const type = (file.type || "").toLowerCase();
    const name = (file.name || "").toLowerCase();
    const isSvg = type.includes("svg") || name.endsWith(".svg");
    const isRaster =
      type.includes("png") ||
      type.includes("jpeg") ||
      type.includes("jpg") ||
      type.includes("webp") ||
      name.endsWith(".png") ||
      name.endsWith(".jpg") ||
      name.endsWith(".jpeg") ||
      name.endsWith(".webp");
    if (!isSvg && !isRaster) {
      return NextResponse.json({ error: "PNG, JPEG, WebP ou SVG uniquement" }, { status: 400 });
    }

    const buffer = Buffer.from(await file.arrayBuffer());
    const contentType = isSvg
      ? "image/svg+xml"
      : type.includes("webp") || name.endsWith(".webp")
        ? "image/webp"
        : type.includes("jpeg") ||
            type.includes("jpg") ||
            name.endsWith(".jpg") ||
            name.endsWith(".jpeg")
          ? "image/jpeg"
          : "image/png";

    const { key, url } = await savePartenariatLogo(
      offreId,
      file.name || (isSvg ? "logo.svg" : "logo.png"),
      buffer,
      contentType,
    );
    return NextResponse.json({ success: true, key, url });
  } catch (e) {
    console.error("[partenariats/logo]", e);
    const msg = e instanceof Error ? e.message : "Upload impossible";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
