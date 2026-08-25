import { NextResponse } from "next/server";
import { requireInternatManage } from "@/app/api/internat/_auth";
import { requireAnyModule } from "@/app/lib/intranet-auth";
import { applyElevePhotosBulk } from "@/app/lib/eleve-photos";

/** Lots volumineux (jusqu’à ~40 photos / requête côté client). */
export const maxDuration = 300;

async function requirePhotosBulkAccess() {
  const admin = await requireAnyModule(["admin-settings"]);
  if (admin.ok) return { ok: true as const };
  return requireInternatManage();
}

export async function POST(req: Request) {
  const gate = await requirePhotosBulkAccess();
  if (!gate.ok) return gate.response;

  const form = await req.formData().catch(() => null);
  if (!form) {
    return NextResponse.json({ error: "Formulaire invalide." }, { status: 400 });
  }

  const files: { filename: string; bytes: Uint8Array; contentType: string }[] = [];
  const seen = new Set<string>();

  const pushFile = async (value: File) => {
    const name = value.name || "photo.jpg";
    if (!/\.(jpe?g|png|webp|gif)$/i.test(name)) return;
    const dedupe = `${name}:${value.size}`;
    if (seen.has(dedupe)) return;
    seen.add(dedupe);
    const buf = new Uint8Array(await value.arrayBuffer());
    if (!buf.length) return;
    files.push({
      filename: name,
      bytes: buf,
      contentType: value.type || "image/jpeg",
    });
  };

  for (const [key, value] of form.entries()) {
    if (!(value instanceof File)) continue;
    if (!key.startsWith("file") && key !== "photos" && key !== "photo" && key !== "files") continue;
    await pushFile(value);
  }

  for (const value of form.getAll("files")) {
    if (value instanceof File) await pushFile(value);
  }

  if (!files.length) {
    return NextResponse.json(
      { error: "Aucune image (jpg/png/webp/gif). Nommez les fichiers « NOM Prenom.jpg »." },
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
