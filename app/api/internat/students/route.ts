import { NextResponse } from "next/server";
import { requireInternatManage, requireInternatAccess } from "@/app/api/internat/_auth";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { resolvePhotoUrlsForInternatStudents } from "@/app/lib/eleve-photos";
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

export async function GET() {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;
  try {
    const [students, rooms] = await Promise.all([getInternatStudents(), getInternatRooms()]);
    const photoUrls = await resolvePhotoUrlsForInternatStudents(students).catch((e) => {
      console.warn("[internat/students] photoUrls", e);
      return {} as Record<string, string>;
    });
    return NextResponse.json({ students, rooms, photoUrls });
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

  if (action === "import") {
    const picks = Array.isArray(body.eleves) ? (body.eleves as EleveConfig[]) : [];
    const students = await getInternatStudents();
    const rooms = await getInternatRooms();
    const bundle = await loadAppConfig();
    const now = new Date().toISOString();
    const added: InternatStudent[] = [];

    for (const e of picks) {
      const key = String(e.folderName || e.ine || "").trim();
      if (!key) continue;
      if (
        students.some(
          (s) =>
            s.eleveRef.folderName === e.folderName ||
            (e.ine && s.eleveRef.ine && s.eleveRef.ine === e.ine),
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
          internatEtablissementFromRaw(e.secteur || e.mef, bundle.establishments) || "Lycée",
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
    return NextResponse.json({ added, students, rooms });
  }

  const now = new Date().toISOString();
  const students = await getInternatStudents();
  const rooms = await getInternatRooms();
  const nom = String(body.nom || "").trim();
  const prenom = String(body.prenom || "").trim();
  if (!nom || !prenom) {
    return NextResponse.json({ error: "Nom et prénom requis." }, { status: 400 });
  }

  const bundle = await loadAppConfig();
  const etablissement =
    internatEtablissementFromRaw(body.etablissement, bundle.establishments) ||
    String(body.etablissement || "").trim();
  if (!etablissement) {
    return NextResponse.json({ error: "Établissement internat requis (configurez un collège, lycée ou site personnalisé)." }, { status: 400 });
  }

  const roomId = body.roomId ? String(body.roomId) : null;
  const draft: InternatStudent = {
    id: newId("stu"),
    eleveRef: {
      folderName: `${nom} — ${prenom}`,
      nom,
      prenom,
      ine: body.ine ? String(body.ine) : undefined,
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
  return NextResponse.json({ student: draft, students });
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
        ? internatEtablissementFromRaw(body.etablissement, (await loadAppConfig()).establishments) ||
          prev.etablissement
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
