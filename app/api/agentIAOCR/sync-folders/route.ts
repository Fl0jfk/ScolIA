import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveEleveFolderName } from "@/app/lib/eleves-config";
import {
  filterEnseignantsForSecteurs,
  loadEnseignantsRegistry,
} from "@/app/lib/enseignants-registry";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { listChildFolderNames, ensureChildFolder, ensureFolderPath } from "@/app/lib/graph-onedrive-folders";
import { loadMefSecteurMap } from "@/app/lib/mef-secteurs";
import {
  filterElevesForSecteur,
  resolveEleveSecteur,
} from "@/app/lib/onedrive-eleves";
import { loadPersonnelEntriesForOcr } from "@/app/lib/ocr-personnel-pool";
import {
  resolveOcrCapabilitiesForUserServer,
  resolveOneDriveProfileForUserServer,
} from "@/app/lib/onedrive-user-profiles.server";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import type { OcrResolvedFlux } from "@/app/lib/ocr-flux";

async function syncNamedFolders(
  accessToken: string,
  basePath: string,
  folderNames: string[],
): Promise<{
  created: string[];
  alreadyThere: string[];
  errors: Array<{ folderName: string; error: string }>;
  extraFoldersOnOneDrive: string[];
  existingOnDrive: string[];
}> {
  await ensureFolderPath(accessToken, basePath);
  const existingOnDrive = await listChildFolderNames(accessToken, basePath);
  const existing = new Set(existingOnDrive);
  const created: string[] = [];
  const alreadyThere: string[] = [];
  const errors: Array<{ folderName: string; error: string }> = [];
  const wanted = new Set<string>();

  for (const folderName of folderNames) {
    const name = folderName.trim();
    if (!name) continue;
    wanted.add(name);
    if (existing.has(name)) {
      alreadyThere.push(name);
      continue;
    }
    try {
      const r = await ensureChildFolder(accessToken, basePath, name);
      existing.add(name);
      if (r.created) created.push(name);
      else alreadyThere.push(name);
    } catch (err) {
      errors.push({
        folderName: name,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  const extraFoldersOnOneDrive = [...existingOnDrive]
    .filter((name) => !wanted.has(name))
    .sort((a, b) => a.localeCompare(b, "fr"));

  return { created, alreadyThere, errors, extraFoldersOnOneDrive, existingOnDrive: [...existingOnDrive] };
}

export async function POST(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const like = user
      ? {
          id: user.id,
          lastName: user.lastName,
          emailAddresses: user.emailAddresses?.map((e) => ({ emailAddress: e.emailAddress })),
          primaryEmailAddress: user.primaryEmailAddress
            ? { emailAddress: user.primaryEmailAddress.emailAddress }
            : null,
        }
      : null;
    const caps = like ? await resolveOcrCapabilitiesForUserServer(like) : { fluxes: [], primaryEleves: null };
    const profile =
      caps.primaryEleves ?? (like ? await resolveOneDriveProfileForUserServer(like) : null);
    if (!profile && caps.fluxes.length === 0) {
      return NextResponse.json(
        {
          error:
            "Aucun flux OCR n'est rattaché à votre compte. Configurez le mapping dans Paramètres → Intégrations.",
        },
        { status: 403 },
      );
    }

    const body = await req.json();
    const accessToken = String(body.accessToken ?? "").trim();
    if (!accessToken) {
      return NextResponse.json({ error: "accessToken OneDrive requis" }, { status: 400 });
    }

    const mefMap = await loadMefSecteurMap();
    const mefTableConfigured = mefMap.size > 0;
    const allEleves = await loadElevesRegistry();
    const allEnseignants = await loadEnseignantsRegistry();

    const elevesFluxes = caps.fluxes.filter((f) => f.kind === "eleves" && f.secteur) as Array<
      OcrResolvedFlux & { secteur: Secteur }
    >;
    const fallbackEleves: Array<OcrResolvedFlux & { secteur: Secteur }> =
      elevesFluxes.length === 0 && profile
        ? [
            {
              id: profile.secteur === "ecole" ? "eleves_ecole" : profile.secteur === "college" ? "eleves_college" : "eleves_lycee",
              kind: "eleves",
              secteur: profile.secteur,
              basePath: profile.basePath,
              label: profile.label,
            },
          ]
        : elevesFluxes;

    const created: string[] = [];
    const alreadyThere: string[] = [];
    const ambiguous: Array<{ folderName: string; mef?: string; reason: string }> = [];
    const errors: Array<{ folderName: string; error: string }> = [];
    let extraFoldersOnOneDrive: string[] = [];
    let oneDriveFoldersFound = 0;
    let jsonForYourSecteur = 0;

    for (const flux of fallbackEleves) {
      const scoped = filterElevesForSecteur(allEleves, flux.secteur, mefMap);
      jsonForYourSecteur += scoped.length;
      const names: string[] = [];
      for (const e of scoped) {
        const folderName = resolveEleveFolderName(e);
        const inferred = resolveEleveSecteur(e, mefMap);
        if (!inferred) {
          const mef = String(e.mef ?? "").trim();
          ambiguous.push({
            folderName,
            mef: mef || undefined,
            reason: mefTableConfigured
              ? mef
                ? "MEF inconnu dans la table"
                : "MEF manquant sur l'élève"
              : "Secteur non détecté (table MEF absente et nom de dossier ambigu)",
          });
          continue;
        }
        if (inferred !== flux.secteur) continue;
        names.push(folderName);
      }
      const report = await syncNamedFolders(accessToken, flux.basePath, names);
      created.push(...report.created);
      alreadyThere.push(...report.alreadyThere);
      errors.push(...report.errors);
      extraFoldersOnOneDrive.push(...report.extraFoldersOnOneDrive.map((n) => `${flux.basePath}/${n}`));
      oneDriveFoldersFound += report.existingOnDrive.length;
    }

    // Enseignants : 3 flux (école / collège / lycée) mais souvent le même chemin → sync une fois par chemin.
    const enseignantsByPath = new Map<string, Set<string>>();
    for (const flux of caps.fluxes.filter((f) => f.kind === "enseignants" && f.secteur)) {
      const names = filterEnseignantsForSecteurs(allEnseignants, [flux.secteur!]).map(
        (e) => e.folderName,
      );
      const set = enseignantsByPath.get(flux.basePath) ?? new Set<string>();
      for (const n of names) set.add(n);
      enseignantsByPath.set(flux.basePath, set);
    }
    for (const [basePath, nameSet] of enseignantsByPath) {
      const report = await syncNamedFolders(accessToken, basePath, [...nameSet]);
      created.push(...report.created);
      alreadyThere.push(...report.alreadyThere);
      errors.push(...report.errors);
    }

    if (caps.fluxes.some((f) => f.kind === "personnel")) {
      const personnelFlux = caps.fluxes.find((f) => f.kind === "personnel")!;
      const entries = await loadPersonnelEntriesForOcr();
      const report = await syncNamedFolders(
        accessToken,
        personnelFlux.basePath,
        entries.map((e) => e.folderName),
      );
      created.push(...report.created);
      alreadyThere.push(...report.alreadyThere);
      errors.push(...report.errors);
    }

    const otherSecteurCounts = {
      lycee: filterElevesForSecteur(allEleves, "lycee", mefMap).length,
      college: filterElevesForSecteur(allEleves, "college", mefMap).length,
      ecole: filterElevesForSecteur(allEleves, "ecole", mefMap).length,
    };

    const label = caps.fluxes.map((f) => f.label).join(" · ") || profile?.label || "";
    const basePath = profile?.basePath || caps.fluxes[0]?.basePath || "";

    return NextResponse.json({
      success: true,
      mefTableConfigured,
      mefCodesInTable: mefMap.size,
      secteur: profile?.secteur ?? null,
      secteurLabel: label,
      basePath,
      jsonTotal: allEleves.length,
      jsonForYourSecteur,
      oneDriveFoldersFound,
      created: created.length,
      alreadyThere: alreadyThere.length,
      createdFolders: created.sort((a, b) => a.localeCompare(b, "fr")),
      extraFoldersOnOneDrive,
      extraFoldersCount: extraFoldersOnOneDrive.length,
      ambiguousCount: ambiguous.length,
      ambiguous: ambiguous.slice(0, 30),
      errors,
      otherSecteurCounts,
      message:
        created.length > 0
          ? `${created.length} dossier(s) créé(s), ${alreadyThere.length} déjà présent(s).`
          : alreadyThere.length > 0
            ? `Aucun nouveau dossier : ${alreadyThere.length} dossier(s) existaient déjà.`
            : "Aucun dossier créé — vérifiez les listes (élèves, enseignants, personnel) et la table MEF.",
    });
  } catch (e) {
    console.error("sync-folders:", e);
    return NextResponse.json({ error: e instanceof Error ? e.message : String(e) }, { status: 500 });
  }
}
