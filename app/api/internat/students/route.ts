import { NextResponse } from "next/server";
import { requireInternatManage, requireInternatAccess } from "@/app/api/internat/_auth";
import type { EleveConfig } from "@/app/lib/eleves-config";
import {
  aliasPhotoIndexAfterInternatMerge,
  photoS3KeysForInternatStudents,
  resolvePhotoUrlsForInternatStudents,
} from "@/app/lib/eleve-photos";
import {
  getInternatRooms,
  getInternatStudents,
  saveInternatStudents,
  validateRoomCapacity,
} from "@/app/lib/internat-storage";
import { normalizeParentContact } from "@/app/lib/internat-outing";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  internatEtablissementFromRaw,
  newId,
  type InternatStudent,
} from "@/app/lib/internat-types";
import {
  dedupeInternatStudents,
  internatStudentMatchesRoster,
} from "@/app/lib/internat-dedupe";

async function dedupeInternatStudentsKeepingPhotos(
  loaded: InternatStudent[],
  by: string,
) {
  const photoS3KeyByStudentId = await photoS3KeysForInternatStudents(loaded).catch(
    () => new Map<string, string>(),
  );
  const deduped = dedupeInternatStudents(loaded, {
    by,
    at: new Date().toISOString(),
    photoS3KeyByStudentId,
  });
  if (deduped.mergedGroups > 0) {
    await aliasPhotoIndexAfterInternatMerge({
      mergeTraces: deduped.mergeTraces,
      photoS3KeyByStudentId,
    }).catch((e) => console.warn("[internat/students] alias photos", e));
  }
  return deduped;
}

/** Recale `classe` depuis le dossier / référentiel élève (source de vérité). */
async function overlayClassesFromEleveDossier(
  students: InternatStudent[],
): Promise<{ students: InternatStudent[]; drifted: number }> {
  try {
    const { resolveCurrentEtablissementId, listElevesFromDb } = await import(
      "@/app/lib/ent-core-db"
    );
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) return { students, drifted: 0 };
    const eleves = await listElevesFromDb(etabId);
    if (!eleves.length) return { students, drifted: 0 };

    const now = new Date().toISOString();
    let drifted = 0;
    const next = students.map((s) => {
      if (!s.actif) return s;
      const match = eleves.find((e) =>
        internatStudentMatchesRoster(s, {
          nom: e.nom,
          prenom: e.prenom,
          ine: e.ine || undefined,
          folderName: e.folderName,
        }),
      );
      const dossierClasse = match?.classe?.trim();
      if (!dossierClasse || dossierClasse === s.classe) return s;
      drifted += 1;
      return {
        ...s,
        classe: dossierClasse,
        updatedAt: now,
        history: [
          ...(s.history || []),
          {
            at: now,
            by: "systeme:dossier",
            action: "SYNC_CLASSE",
            note: `${s.classe || "—"} → ${dossierClasse}`,
          },
        ],
      };
    });
    return { students: next, drifted };
  } catch (e) {
    console.warn("[internat/students] overlay classes dossier", e);
    return { students, drifted: 0 };
  }
}

export async function GET() {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;
  try {
    const [loaded, rooms] = await Promise.all([getInternatStudents(), getInternatRooms()]);
    const deduped = await dedupeInternatStudentsKeepingPhotos(loaded, "systeme:get");
    let students = deduped.students;
    const overlay = await overlayClassesFromEleveDossier(students);
    students = overlay.students;
    if (deduped.mergedGroups > 0 || overlay.drifted > 0) {
      await saveInternatStudents(students);
      students = await getInternatStudents();
    }
    const photoUrls = await resolvePhotoUrlsForInternatStudents(students).catch((e) => {
      console.warn("[internat/students] photoUrls", e);
      return {} as Record<string, string>;
    });
    return NextResponse.json({
      students,
      rooms,
      photoUrls,
      dedupe:
        deduped.mergedGroups > 0
          ? { mergedGroups: deduped.mergedGroups, removedActifs: deduped.removedActifs }
          : undefined,
      classesSynced: overlay.drifted > 0 ? overlay.drifted : undefined,
    });
  } catch (e) {
    console.error("[internat/students] GET", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Chargement des internes impossible." },
      { status: 500 },
    );
  }
}

export async function POST(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "create");

  if (action === "dedupe") {
    const students = await getInternatStudents();
    const rooms = await getInternatRooms();
    const result = await dedupeInternatStudentsKeepingPhotos(students, access.userName);
    await saveInternatStudents(result.students);
    const photoUrls = await resolvePhotoUrlsForInternatStudents(result.students).catch(
      () => ({} as Record<string, string>),
    );
    return NextResponse.json({
      ok: true,
      mergedGroups: result.mergedGroups,
      removedActifs: result.removedActifs,
      students: result.students,
      rooms,
      photoUrls,
      message:
        result.mergedGroups === 0
          ? "Aucun doublon détecté."
          : `${result.mergedGroups} groupe(s) fusionné(s), ${result.removedActifs} fiche(s) active(s) sortie(s) — photo / chambre / contacts conservés.`,
    });
  }

  if (action === "import") {
    const picks = Array.isArray(body.eleves) ? (body.eleves as EleveConfig[]) : [];
    const students = await getInternatStudents();
    const rooms = await getInternatRooms();
    const bundle = await loadAppConfig();
    const now = new Date().toISOString();
    const added: InternatStudent[] = [];

    for (const e of picks) {
      const key = String(e.folderName || e.ine || `${e.nom}|${e.prenom}`).trim();
      if (!key) continue;
      if (
        students.some((s) =>
          internatStudentMatchesRoster(s, {
            ine: e.ine,
            folderName: e.folderName,
            nom: e.nom,
            prenom: e.prenom,
          }),
        )
      ) {
        continue;
      }
      const student: InternatStudent = {
        id: newId("stu"),
        eleveRef: {
          ine: e.ine || undefined,
          folderName: e.folderName,
          nom: e.nom,
          prenom: e.prenom,
        },
        sexe: body.defaultSexe === "F" ? "F" : "M",
        etablissement:
          internatEtablissementFromRaw(
            e.secteur || e.mef,
            bundle.establishments,
            String(body.defaultClasse || e.folderName.split("—").pop() || "").trim(),
          ) || "Lycée",
        classe: String(body.defaultClasse || e.folderName.split("—").pop() || "").trim() || "—",
        actif: true,
        createdAt: now,
        updatedAt: now,
        history: [{ at: now, by: access.userName, action: "IMPORT_ELEVE", note: e.folderName }],
      };
      added.push(student);
      students.push(student);
    }

    await saveInternatStudents(students);
    return NextResponse.json({ added, students: await getInternatStudents(), rooms });
  }

  const now = new Date().toISOString();
  const students = await getInternatStudents();
  const rooms = await getInternatRooms();
  const nom = String(body.nom || "").trim();
  const prenom = String(body.prenom || "").trim();
  if (!nom || !prenom) {
    return NextResponse.json({ error: "Nom et prénom requis." }, { status: 400 });
  }

  const ine = body.ine ? String(body.ine).trim() : undefined;
  const folderName = `${nom} — ${prenom}`;
  const existing = students.find((s) =>
    internatStudentMatchesRoster(s, { ine, folderName, nom, prenom }),
  );
  if (existing?.actif) {
    return NextResponse.json(
      {
        error: `Doublon : ${existing.eleveRef.nom} ${existing.eleveRef.prenom} est déjà interne.`,
        existingId: existing.id,
      },
      { status: 409 },
    );
  }

  const bundle = await loadAppConfig();
  const etablissement =
    internatEtablissementFromRaw(
      body.etablissement,
      bundle.establishments,
      String(body.classe || "").trim(),
    ) || String(body.etablissement || "").trim();
  if (!etablissement) {
    return NextResponse.json({ error: "Établissement internat requis (configurez un collège, lycée ou site personnalisé)." }, { status: 400 });
  }

  const roomId = body.roomId ? String(body.roomId) : null;

  if (existing && !existing.actif) {
    const idx = students.findIndex((s) => s.id === existing.id);
    const reactivated: InternatStudent = {
      ...existing,
      eleveRef: {
        ine: ine || existing.eleveRef.ine,
        folderName: existing.eleveRef.folderName || folderName,
        nom,
        prenom,
      },
      sexe: body.sexe === "F" ? "F" : body.sexe === "M" ? "M" : existing.sexe,
      etablissement,
      classe: String(body.classe || "").trim() || existing.classe || "—",
      roomId: roomId ?? existing.roomId,
      actif: true,
      sortieAt: undefined,
      sortieMotif: undefined,
      updatedAt: now,
      history: [
        ...(existing.history || []),
        { at: now, by: access.userName, action: "REACTIVATION_MANUELLE", note: "Création — doublon inactif réactivé" },
      ],
    };
    const cap = validateRoomCapacity(students, rooms, reactivated.id, reactivated.roomId ?? null);
    if (!cap.ok) return NextResponse.json({ error: cap.error }, { status: 400 });
    students[idx] = reactivated;
    await saveInternatStudents(students);
    return NextResponse.json({ student: reactivated, students: await getInternatStudents(), reactivated: true });
  }

  const draft: InternatStudent = {
    id: newId("stu"),
    eleveRef: {
      folderName,
      nom,
      prenom,
      ine,
    },
    sexe: body.sexe === "F" ? "F" : "M",
    etablissement,
    classe: String(body.classe || "").trim() || "—",
    roomId,
    actif: true,
    createdAt: now,
    updatedAt: now,
    history: [{ at: now, by: access.userName, action: "CREATION_MANUELLE" }],
  };

  const cap = validateRoomCapacity(students, rooms, draft.id, roomId);
  if (!cap.ok) return NextResponse.json({ error: cap.error }, { status: 400 });

  students.push(draft);
  await saveInternatStudents(students);
  return NextResponse.json({ student: draft, students: await getInternatStudents() });
}

export async function PATCH(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;

  const body = await req.json().catch(() => ({}));
  const id = String(body.id || "");
  const students = await getInternatStudents();
  const rooms = await getInternatRooms();
  const idx = students.findIndex((s) => s.id === id);
  if (idx < 0) return NextResponse.json({ error: "Interne introuvable." }, { status: 404 });

  const now = new Date().toISOString();
  const prev = students[idx];
  const roomId = body.roomId !== undefined ? (body.roomId ? String(body.roomId) : null) : prev.roomId;

  const cap = validateRoomCapacity(students, rooms, id, roomId);
  if (!cap.ok) return NextResponse.json({ error: cap.error }, { status: 400 });

  const updated: InternatStudent = {
    ...prev,
    sexe: body.sexe === "F" || body.sexe === "M" ? body.sexe : prev.sexe,
    etablissement:
      body.etablissement !== undefined
        ? internatEtablissementFromRaw(
            body.etablissement,
            (await loadAppConfig()).establishments,
            body.classe !== undefined ? String(body.classe || "").trim() : prev.classe,
          ) || prev.etablissement
        : prev.etablissement,
    classe: body.classe !== undefined ? String(body.classe || "").trim() || prev.classe : prev.classe,
    roomId,
    parent1: body.parent1 !== undefined ? normalizeParentContact(body.parent1) : prev.parent1,
    parent2: body.parent2 !== undefined ? normalizeParentContact(body.parent2) : prev.parent2,
    medical:
      body.medical !== undefined
        ? {
            allergies: String(body.medical?.allergies || "").trim() || undefined,
            pai: String(body.medical?.pai || "").trim() || undefined,
            treatments: String(body.medical?.treatments || "").trim() || undefined,
            notes: String(body.medical?.notes || "").trim() || undefined,
          }
        : prev.medical,
    specialAuthorizations: Array.isArray(body.specialAuthorizations)
      ? body.specialAuthorizations
      : prev.specialAuthorizations,
    underWatch: body.underWatch !== undefined ? Boolean(body.underWatch) : prev.underWatch,
    underWatchNote:
      body.underWatchNote !== undefined
        ? String(body.underWatchNote || "").trim() || undefined
        : prev.underWatchNote,
    actif: body.actif !== undefined ? Boolean(body.actif) : prev.actif,
    sortieAt:
      body.actif === false
        ? now
        : body.actif === true
          ? undefined
          : prev.sortieAt,
    sortieMotif:
      body.actif === false
        ? String(body.sortieMotif || body.note || "Désactivation manuelle").trim() ||
          "Désactivation manuelle"
        : body.actif === true
          ? undefined
          : prev.sortieMotif,
    updatedAt: now,
    history: [
      ...(prev.history || []),
      {
        at: now,
        by: access.userName,
        action:
          body.actif === false
            ? "SORTIE_MANUELLE"
            : body.actif === true && !prev.actif
              ? "REACTIVATION_MANUELLE"
              : "MODIFICATION",
        note: String(body.note || body.sortieMotif || "") || undefined,
      },
    ],
  };
  students[idx] = updated;
  await saveInternatStudents(students);
  return NextResponse.json({ student: updated, students });
}

export async function DELETE(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;
  const { searchParams } = new URL(req.url);
  const id = String(searchParams.get("id") || "");
  const students = await getInternatStudents();
  await saveInternatStudents(students.filter((s) => s.id !== id));
  return NextResponse.json({ ok: true });
}
