import { NextResponse } from "next/server";
import { requireInternatManage } from "@/app/api/internat/_auth";
import { requireAnyModule } from "@/app/lib/intranet-auth";
import { getAppSession } from "@/app/lib/intranet-session";
import {
  isValidElevePhotoJobId,
  newElevePhotoJobId,
  stageElevePhotoFiles,
} from "@/app/lib/eleve-photos-batch";

/** Lots d’upload uniquement (pas d’association) — le client peut enchaîner puis quitter. */
export const maxDuration = 120;

async function requirePhotosBulkAccess() {
  const admin = await requireAnyModule(["admin-settings"]);
  if (admin.ok) return { ok: true as const };
  return requireInternatManage();
}

export async function POST(req: Request) {
  const gate = await requirePhotosBulkAccess();
  if (!gate.ok) return gate.response;

  const session = await getAppSession();
  const userId = session?.user?.id?.trim();
  if (!userId) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const form = await req.formData().catch(() => null);
  if (!form) {
    return NextResponse.json({ error: "Formulaire invalide." }, { status: 400 });
  }

  const jobIdRaw = String(form.get("jobId") || "").trim();
  const jobId = jobIdRaw && isValidElevePhotoJobId(jobIdRaw) ? jobIdRaw : newElevePhotoJobId();

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

  for (const value of form.getAll("files")) {
    if (value instanceof File) await pushFile(value);
  }
  for (const [key, value] of form.entries()) {
    if (!(value instanceof File)) continue;
    if (!key.startsWith("file") && key !== "photos" && key !== "photo") continue;
    await pushFile(value);
  }

  if (!files.length) {
    return NextResponse.json(
      { error: "Aucune image (jpg/png/webp/gif)." },
      { status: 400 },
    );
  }

  try {
    const job = await stageElevePhotoFiles(jobId, userId, files);
    return NextResponse.json({
      jobId: job.jobId,
      received: files.length,
      totalStaged: job.items.length,
      status: job.status,
      label: job.label,
    });
  } catch (e: unknown) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Upload impossible." },
      { status: 400 },
    );
  }
}
