import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessCertificatesModule } from "@/app/lib/certificates-auth";
import { loadCertificateStudentPicker } from "@/app/lib/certificates-students";

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  if (!canAccessCertificatesModule(user)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  try {
    const url = new URL(req.url);
    const q = url.searchParams.get("q") || undefined;
    const classe = url.searchParams.get("classe") || undefined;
    const siteId = url.searchParams.get("siteId") || undefined;
    const { students, sites, classOptions } = await loadCertificateStudentPicker({
      q,
      classe,
      siteId,
    });
    const classes = [
      ...new Set(classOptions.map((c) => c.value).filter(Boolean)),
    ].sort((a, b) => a.localeCompare(b, "fr"));
    return NextResponse.json({ students, sites, classOptions, classes });
  } catch (e) {
    console.error("[certificates/students]", e);
    return NextResponse.json(
      { error: "Impossible de charger les élèves." },
      { status: 500 },
    );
  }
}
