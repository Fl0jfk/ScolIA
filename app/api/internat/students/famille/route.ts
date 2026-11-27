import { NextResponse } from "next/server";
import { requireInternatAccess } from "@/app/api/internat/_auth";
import { canOpenEleveDossierDetail } from "@/app/lib/accueil-access";
import { loadInternatFamilles } from "@/app/lib/internat-famille-db";
import { isOrgAdminMetadata } from "@/app/lib/internat-rbac";
import { getInternatStudents } from "@/app/lib/internat-storage";

export const runtime = "nodejs";

/** Famille d'un interne, lue à la demande dans le dossier élève (lecture seule). */
export async function GET(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;
  try {
    const studentId = new URL(req.url).searchParams.get("studentId") || "";
    const student = (await getInternatStudents()).find((s) => s.id === studentId);
    if (!student) return NextResponse.json({ error: "Interne introuvable." }, { status: 404 });
    const famille = (await loadInternatFamilles([student])).get(student.id);
    return NextResponse.json(
      {
        famille: famille ?? { eleveId: null, foyers: [] },
        peutOuvrirDossier: canOpenEleveDossierDetail({
          roles: access.roles,
          orgAdmin: isOrgAdminMetadata(access.user?.publicMetadata),
        }),
      },
      { headers: { "Cache-Control": "private, no-store" } },
    );
  } catch (e) {
    console.error("[internat/students/famille]", e);
    return NextResponse.json({ error: "Chargement de la famille impossible." }, { status: 500 });
  }
}
