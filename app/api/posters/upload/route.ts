import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { getSignedReadUrl } from "@/app/lib/s3-storage";
import { savePosterAsset } from "@/app/lib/posters";

export const runtime = "nodejs";
export const maxDuration = 60;

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const formData = await req.formData();
    const file = formData.get("file");
    const kindRaw = String(formData.get("kind") || "partner-logo");
    const kind =
      kindRaw === "background"
        ? "background"
        : kindRaw === "image"
          ? "image"
          : "partner-logo";

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
      name.endsWith(".png") ||
      name.endsWith(".jpg") ||
      name.endsWith(".jpeg");
    if (!isSvg && !isRaster) {
      return NextResponse.json(
        { error: "PNG, JPEG ou SVG uniquement" },
        { status: 400 },
      );
    }

    const buffer = Buffer.from(await file.arrayBuffer());
    const contentType = isSvg
      ? "image/svg+xml"
      : type.includes("jpeg") || type.includes("jpg") || name.endsWith(".jpg") || name.endsWith(".jpeg")
        ? "image/jpeg"
        : "image/png";
    const { key } = await savePosterAsset(
      kind,
      file.name || (isSvg ? "logo.svg" : "image.png"),
      buffer,
      contentType,
    );    const url = (await getSignedReadUrl(key, 3600)) || key;

    return NextResponse.json({ success: true, key, url });
  } catch (e) {
    console.error("[posters/upload]", e);
    const msg = e instanceof Error ? e.message : "Upload impossible";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
