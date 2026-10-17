import { NextResponse } from "next/server";
import { requireViewUser } from "@/app/lib/intranet-session";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import { createAdminOfflineSignedConvention } from "@/app/lib/stage-workflow";
import type { StageInternshipKind } from "@/app/lib/stage-types";

export const maxDuration = 60;

function displayName(user: {
  firstName?: string | null;
  lastName?: string | null;
  name?: string | null;
}) {
  const first = user.firstName?.trim() || "";
  const last = user.lastName?.trim() || "";
  const combined = `${first} ${last}`.trim();
  if (combined) return combined;
  const name = user.name?.trim();
  return name || "Administratif";
}

function isPdfFile(file: File) {
  const name = String(file.name || "").toLowerCase();
  const type = String(file.type || "").toLowerCase();
  if (name.endsWith(".pdf")) return true;
  return type === "application/pdf" || type === "application/x-pdf";
}

function parseInternshipKind(raw: string): StageInternshipKind {
  const v = raw.trim().toLowerCase();
  if (v === "pfmp") return "pfmp";
  if (v === "job_ete") return "job_ete";
  if (v === "autre") return "autre";
  return "stage_observation";
}

function storageErrorMessage(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error);
  const lower = raw.toLowerCase();
  if (
    lower.includes("access denied") ||
    lower.includes("accessdenied") ||
    lower.includes("unauthorized") ||
    lower.includes("not authorized") ||
    lower.includes("invalidaccesskeyid") ||
    lower.includes("signaturedoesnotmatch")
  ) {
    return "Envoi du PDF vers le stockage impossible (droits S3). Réessayez ou contactez l'administrateur technique.";
  }
  if (lower.includes("access_key") || lower.includes("secret_access_key") || lower.includes("manquants")) {
    return "Stockage non configuré pour cet établissement (clés S3 manquantes).";
  }
  return raw || "Erreur serveur lors de l'enregistrement hors plateforme.";
}

export async function POST(req: Request) {
  try {
    const gate = await requireViewUser();
    if (!gate.ok) {
      return NextResponse.json(
        {
          error:
            gate.reason === "unavailable"
              ? "Authentification temporairement indisponible."
              : "Session expirée ou non connecté. Rechargez la page, reconnectez-vous, puis réessayez.",
          code: gate.reason === "unavailable" ? "AUTH_UNAVAILABLE" : "AUTH_REQUIRED",
        },
        { status: gate.reason === "unavailable" ? 503 : 401 },
      );
    }

    const user = gate.user;
    const roles = user.roles;
    if (!canReviewPreconvention(roles)) {
      return NextResponse.json(
        {
          error:
            "Réservé à l'administratif, à la direction ou à l'admin de l'établissement.",
          code: "STAGE_REVIEW_FORBIDDEN",
        },
        { status: 403 },
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

    const str = (key: string) => String(formData.get(key) ?? "").trim();
    const pdfBytes = new Uint8Array(await file.arrayBuffer());

    const result = await createAdminOfflineSignedConvention({
      by: user.businessUserId,
      byName: displayName(user),
      student: {
        firstName: str("studentFirstName"),
        lastName: str("studentLastName"),
        className: str("studentClassName"),
        level: str("studentLevel") || undefined,
        dateNaissance: str("studentDateNaissance") || undefined,
        email: str("studentEmail") || undefined,
        matchedEleveIne: str("matchedEleveIne") || undefined,
      },
      company: {
        name: str("companyName"),
        address: str("companyAddress"),
        postalCode: str("companyPostalCode") || undefined,
        city: str("companyCity") || undefined,
        siret: str("companySiret") || undefined,
        activity: str("companyActivity") || undefined,
        tutorName: str("tutorName") || undefined,
        tutorEmail: str("tutorEmail") || undefined,
        tutorPhone: str("tutorPhone") || undefined,
      },
      periodStart: str("periodStart"),
      periodEnd: str("periodEnd"),
      internshipKind: parseInternshipKind(str("internshipKind")),
      stageLabel: str("stageLabel") || undefined,
      note: str("note") || undefined,
      pdfBytes,
      pdfFileName: file.name || "convention-signee.pdf",
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: 400 });
    }

    return NextResponse.json({
      success: true,
      convention: result.convention,
    });
  } catch (error) {
    console.error("[stages/conventions/offline]", error);
    return NextResponse.json({ error: storageErrorMessage(error) }, { status: 500 });
  }
}
