import { NextResponse } from "next/server";
import {
  processConventionPdfDeposit,
  StageDepositRejectedError,
} from "@/app/lib/stage-convention-ingest";
import { resolveStagesConventionTemplateUrl } from "@/app/lib/stage-config";
import {
  notifyStageConventionDeposited,
  notifyStageDepositPaperRejected,
} from "@/app/lib/stage-notify";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

export const maxDuration = 120;

const stageDepositLimiter = createMemoryRateLimiter({
  windowMs: 60 * 60 * 1000,
  max: 5,
});

export async function GET() {
  const templateUrl = await resolveStagesConventionTemplateUrl();
  return NextResponse.json({ templateUrl: templateUrl || null });
}

function isPdfFile(file: File) {
  const name = String(file.name || "").toLowerCase();
  const type = String(file.type || "").toLowerCase();
  if (name.endsWith(".pdf")) return true;
  return type === "application/pdf" || type === "application/x-pdf";
}

export async function POST(req: Request) {
  try {
    if (!stageDepositLimiter.allow(clientIpFromRequest(req))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez plus tard." },
        { status: 429 },
      );
    }

    const formData = await req.formData();
    const file = formData.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier PDF requis." }, { status: 400 });
    }
    if (!isPdfFile(file)) {
      return NextResponse.json({ error: "Seuls les fichiers PDF sont acceptés." }, { status: 400 });
    }
    if (file.size > 15 * 1024 * 1024) {
      return NextResponse.json({ error: "Le PDF dépasse 15 Mo." }, { status: 400 });
    }

    const { convention, warnings } = await processConventionPdfDeposit({ file });
    void notifyStageConventionDeposited(convention).catch((e) =>
      console.error("[stages] notify deposit:", e),
    );

    return NextResponse.json({
      success: true,
      conventionId: convention.id,
      student: convention.student,
      company: {
        name: convention.company.name,
        siret: convention.company.siret,
        tutorName: convention.company.tutorName,
        tutorEmail: convention.company.tutorEmail,
        tutorPhone: convention.company.tutorPhone,
      },
      warnings,
    });
  } catch (error: unknown) {
    if (error instanceof StageDepositRejectedError) {
      void notifyStageDepositPaperRejected({
        studentLabel: error.studentLabel,
        missingSignatures: error.missingSignatures,
        missingFields: error.missingFields,
        notifyEmails: error.notifyEmails,
      }).catch((e) => console.error("[stages] notify paper reject:", e));

      return NextResponse.json(
        {
          error: error.message,
          missingSignatures: error.missingSignatures,
          missingFields: error.missingFields,
          rejected: true,
        },
        { status: 422 },
      );
    }
    console.error("[stages/public/deposer]", error);
    const message = error instanceof Error ? error.message : "Échec du traitement.";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
