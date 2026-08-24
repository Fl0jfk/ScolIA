import { NextResponse } from "next/server";
import { requireInternatManage } from "@/app/api/internat/_auth";
import { applyElevePhotosBulk } from "@/app/lib/eleve-photos";

export async function POST(req: Request) {
  const gate = await requireInternatManage();
  if (!gate.ok) return gate.response;

  const form = await req.formData().catch(() => null);
  if (!form) {
    return NextResponse.json({ error: "Formulaire invalide." }, { status: 400 });
  }

  const files: { filename: string; bytes: Uint8Array; contentType: string }[] = [];
  for (const [key, value] of form.entries()) {
    if (!(value instanceof File)) continue;
    if (!key.startsWith("file") && key !== "photos" && key !== "photo") continue;
    const name = value.name || "photo.jpg";
    if (!/\.(jpe?g|png|webp|gif)$/i.test(name)) continue;
    const buf = new Uint8Array(await value.arrayBuffer());
    if (!buf.length) continue;
    files.push({
      filename: name,
      bytes: buf,
      contentType: value.type || "image/jpeg",
    });
  }

  const multi = form.getAll("files");
  for (const value of multi) {
    if (!(value instanceof File)) continue;
    const name = value.name || "photo.jpg";
    if (!/\.(jpe?g|png|webp|gif)$/i.test(name)) continue;
    const buf = new Uint8Array(await value.arrayBuffer());
    if (!buf.length) continue;
    files.push({
      filename: name,
      bytes: buf,
      contentType: value.type || "image/jpeg",
    });
  }

  if (!files.length) {
    return NextResponse.json(
      { error: "Aucune image (jpg/png/webp). Nommez les fichiers « NOM Prenom.jpg »." },
      { status: 400 },
    );
  }

  try {
    const result = await applyElevePhotosBulk(files);
    return NextResponse.json({
      ...result,
      message: `${result.updated} photo(s) associée(s), ${result.unmatched.length} non reconnue(s).`,
    });
  } catch (e: unknown) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Import photos impossible." },
      { status: 400 },
    );
  }
}
