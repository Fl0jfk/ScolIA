import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { extractTextFromUpload } from "@/app/lib/personnel-document-text";
import {
  extractPersonnelFieldsFromText,
  matchPersonnelLocally,
  resolvePersonnelWithAi,
} from "@/app/lib/personnel-match";
import {
  getAllPersonnelRecords,
  getPersonnelRecord,
  savePersonnelRecord,
} from "@/app/lib/personnel-storage";
import {
  computeNextEntretienDue,
  mapExtractedDocCategory,
  normalizeMedecineTravail,
  syncMedecineDerivedFields,
} from "@/app/lib/personnel-rh-cycles";
import {
  canManagePersonnel,
  normalizePersonnelRecord,
  uid,
  type PersonnelDocument,
} from "@/app/lib/personnel-types";
import { s3Key } from "@/app/lib/s3-path";
import { getBucketName } from "@/app/lib/s3-storage";
import { publicS3UrlForKey } from "@/app/lib/travels-s3";
import { requireAuth } from "@/app/lib/intranet-auth";


function safeFileName(name: string) {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 120) || "document";
}

function parseIsoDateInput(raw?: string): string | null {
  if (!raw) return null;
  const s = raw.trim().slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) return null;
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? null : s;
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canManagePersonnel(roles)) {
    return NextResponse.json({ error: "Réservé à la RH / comptabilité." }, { status: 403 });
  }

  try {
    const form = await req.formData();
    const file = form.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier manquant." }, { status: 400 });
    }

    const forcedStaffId = String(form.get("staffId") || "").trim();

    const bytes = new Uint8Array(await file.arrayBuffer());
    const text = await extractTextFromUpload(bytes, file.name, file.type);
    const extracted = await extractPersonnelFieldsFromText(text);

    const staff = await getAllPersonnelRecords();
    if (staff.length === 0) {
      return NextResponse.json(
        { error: "Aucun dossier personnel. Créez d'abord une fiche." },
        { status: 400 },
      );
    }

    let record = forcedStaffId ? staff.find((s) => s.id === forcedStaffId) || null : null;
    let candidates: typeof staff = [];
    let matchedBy: string | undefined = forcedStaffId ? "staffId" : undefined;

    if (!record) {
      const match = matchPersonnelLocally(extracted, staff);
      record = match.record;
      candidates = match.candidates;
      matchedBy = match.matchedBy;
      if (!record && candidates.length > 0) {
        record = await resolvePersonnelWithAi(text, extracted, candidates);
        if (record) matchedBy = matchedBy || "ia";
      }
    }

    if (!record) {
      return NextResponse.json(
        {
          error: "Collaborateur non identifié automatiquement.",
          extracted,
          candidates: candidates.map((c) => ({
            id: c.id,
            displayName: c.displayName,
            email: c.email,
          })),
        },
        { status: 422 },
      );
    }

    const staffId = record.id;
    const fileKey = s3Key(
      `personnel-ogec/${staffId}/documents/${Date.now()}-${safeFileName(file.name)}`,
    );
    const s3Client = await getTenantDataS3Client();
    await s3Client.send(
      new PutObjectCommand({
        Bucket: await getBucketName(),
        Key: fileKey,
        Body: Buffer.from(bytes),
        ContentType: file.type || "application/octet-stream",
      }),
    );
    const fileUrl = await publicS3UrlForKey(fileKey);

    const full = await getPersonnelRecord(staffId);
    if (!full) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });

    const category = mapExtractedDocCategory(extracted.type_document);
    const docId = uid("doc");

    const doc: PersonnelDocument = {
      id: docId,
      name: file.name,
      fileUrl,
      s3Key: fileKey,
      category,
      visibility: "establishment",
      uploadedAt: new Date().toISOString(),
      uploadedBy: user?.fullName || user?.id || "RH",
    };

    const autoCreated: string[] = [];
    const eventDate = parseIsoDateInput(extracted.date_document);

    if (category === "medecine" && eventDate) {
      const med = normalizeMedecineTravail(full.medecineTravail);
      full.medecineTravail = syncMedecineDerivedFields({
        ...med,
        visits: [
          ...(med.visits || []),
          {
            id: uid("medv"),
            visitedAt: eventDate,
            visitType: extracted.type_document || "",
            documentId: docId,
            notes: "Créé automatiquement depuis le dépôt IA",
            createdAt: new Date().toISOString(),
          },
        ],
      });
      autoCreated.push(`visite médecine du ${eventDate}`);
    }

    if (category === "entretien" && eventDate) {
      full.entretiens = [
        ...full.entretiens,
        {
          id: uid("ent"),
          status: "realise",
          scheduledAt: eventDate,
          completedAt: eventDate,
          reminderAt: null,
          nextDueAt: computeNextEntretienDue(eventDate),
          documentId: docId,
          notes: "Créé automatiquement depuis le dépôt IA",
        },
      ];
      autoCreated.push(`entretien du ${eventDate}`);
    }

    const saved = await savePersonnelRecord(
      normalizePersonnelRecord({
        ...full,
        documents: [...full.documents, doc],
      }),
    );

    return NextResponse.json({
      success: true,
      matched: {
        id: saved.id,
        displayName: saved.displayName,
        email: saved.email,
        by: matchedBy,
      },
      extracted,
      document: doc,
      autoCreated,
    });
  } catch (e) {
    console.error("[personnel/deposit]", e);
    const msg = e instanceof Error ? e.message : "Erreur lors du dépôt.";
    return NextResponse.json({ error: msg }, { status: 500 });
  }
}
