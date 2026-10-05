import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { canManageFichesDialogue } from "@/app/lib/fiches-dialogue-access";
import { isEleveScolarise } from "@/app/lib/eleves-config";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { niveauFromClasse } from "@/app/lib/internat-level";
import { isFdNiveau } from "@/app/lib/fiches-dialogue-templates";
import {
  filterObservedClassesForCurrentYearUi,
  loadOfficialSchoolClasses,
} from "@/app/lib/nomenclature-classes";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import { requireTenantId } from "@/app/lib/tenant-scope";

/**
 * Élèves scolarisés filtrables par niveau pour le wizard fiches de dialogue.
 * Classes = Siècle (toutes les divisions du niveau) ∪ observées, pas seulement
 * celles déjà présentes dans le registry filtré trop strictement.
 * Query: ?niveau=3e
 */
export async function GET(req: Request) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canManageFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  try {
    const url = new URL(req.url);
    const niveauRaw = url.searchParams.get("niveau")?.trim() || "";
    const niveau = niveauRaw && isFdNiveau(niveauRaw) ? niveauRaw : null;

    const [elevesRaw, official] = await Promise.all([
      loadElevesActifsRegistry(),
      loadOfficialSchoolClasses(scope.ctx.etablissementId),
    ]);

    let eleves = elevesRaw.filter(isEleveScolarise);
    if (niveau) {
      eleves = eleves.filter((e) => niveauFromClasse(e.classe) === niveau);
    }

    const observedClasses = eleves
      .map((e) => String(e.classe || "").trim())
      .filter(Boolean);

    const siecleForNiveau = (official.lockedClasses || []).filter(
      (c) => !niveau || niveauFromClasse(c) === niveau,
    );

    const mergedRaw = [...siecleForNiveau, ...observedClasses];
    const classes = filterObservedClassesForCurrentYearUi(mergedRaw, official).filter(
      (c) => !niveau || niveauFromClasse(c) === niveau,
    );

    // Si le filtre Siècle a tout mangé, repli : classes observées du niveau
    const finalClasses =
      classes.length > 0
        ? classes
        : [...new Set(observedClasses)]
            .filter((c) => !niveau || niveauFromClasse(c) === niveau)
            .sort((a, b) => a.localeCompare(b, "fr", { numeric: true }));

    await writeDataAccessAudit({
      etablissementId: scope.ctx.etablissementId,
      userId: appUser.user.businessUserId || appUser.user.id,
      resourceType: "eleves_registry",
      action: "list",
      req,
      metadata: {
        count: eleves.length,
        classes: finalClasses.length,
        source: "fiches-dialogue-eleves-picker",
        niveau,
      },
    });

    return NextResponse.json({
      eleves: eleves.map((e) => ({
        id: e.id,
        nom: e.nom,
        prenom: e.prenom,
        classe: e.classe,
        ine: e.ine,
        dateNaissance: e.dateNaissance,
        photoKey: e.photoKey,
        lv1: e.lv1,
        lv2: e.lv2,
      })),
      classes: finalClasses,
      niveau,
      count: eleves.length,
    });
  } catch (err: unknown) {
    console.error("[fiches-dialogue/eleves-picker]", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Chargement élèves impossible." },
      { status: 500 },
    );
  }
}
