import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { PutObjectCommand } from "@aws-sdk/client-s3";
import {
  canIngestFromUser,
  newJobId,
  writeIngestJob,
  type IngestJob,
} from "./ingest-job";
import { mapIngestFailureMessage } from "@/app/lib/absence-ingest-process";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";

export const maxDuration = 60;

function isPdfFile(file: File) {
  const name = String(file.name || "").toLowerCase();
  const type = String(file.type || "").toLowerCase();
  if (name.endsWith(".pdf")) return true;
  return type === "application/pdf" || type === "application/x-pdf";
}

export async function POST(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return new NextResponse("Non autorisé", { status: 401 });

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canIngestFromUser(roles)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }

  try {
    const formData = await req.formData();
    const file = formData.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier PDF requis." }, { status: 400 });
    }
    if (!isPdfFile(file)) {
      return NextResponse.json(
        {
          error: "Seuls les PDF sont autorisés (.pdf). Si vous glissez-déposez, vérifiez l'extension du fichier.",
          code: "INVALID_FILE_TYPE",
        },
        { status: 400 },
      );
    }
    if (file.size > 15 * 1024 * 1024) {
      return NextResponse.json({ error: "Le PDF dépasse 15 Mo." }, { status: 400 });
    }

    const safeName = file.name.replace(/[^\w.\-]+/g, "_");
    const key = `absences/pdfs/${Date.now()}_${safeName}`;
    const buffer = Buffer.from(await file.arrayBuffer());
    const s3Client = await getTenantDataS3Client();
    await s3Client.send(
      new PutObjectCommand({
        Bucket: await getBucketName(),
        Key: key,
        Body: buffer,
        ContentType: "application/pdf",
      }),
    );

    const jobId = newJobId();
    const startedAt = new Date().toISOString();
    const job: IngestJob = {
      jobId,
      userId,
      creatorName: user?.fullName || user?.firstName || "Administrateur",
      creatorEmail: user?.primaryEmailAddress?.emailAddress || "",
      creatorRoles: roles,
      status: "pending",
      startedAt,
      updatedAt: startedAt,
      sourceFileName: file.name,
      documentKey: key,
    };
    await writeIngestJob(job);

    return NextResponse.json(
      {
        accepted: true,
        jobId,
        status: "pending",
        detail:
          "PDF enregistré. L'analyse démarre au prochain appel (suivi automatique). Ne fermez pas la page.",
      },
      { status: 202 },
    );
  } catch (error) {
    console.error("Absences ingest error:", error);
    const mapped = mapIngestFailureMessage(error);
    return NextResponse.json(mapped, { status: 500 });
  }
}
