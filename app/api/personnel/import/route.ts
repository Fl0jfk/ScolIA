import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { collectOcrEmails } from "@/app/lib/ocr-email-match";
import { parsePersonnelExcelBuffer } from "@/app/lib/ocr-personnel-import";
import { findDirectoryMemberByEmail } from "@/app/lib/personnel-directory";
import { findPersonnelByEmail, savePersonnelRecord } from "@/app/lib/personnel-storage";
import {
  defaultMedecineTravail,
  normalizePersonnelRecord,
  uid,
  type PersonnelCategory,
  type PersonnelRecord,
} from "@/app/lib/personnel-types";

function isExcelFile(file: File) {
  const name = file.name.toLowerCase();
  return (
    name.endsWith(".xlsx") ||
    name.endsWith(".xls") ||
    file.type.includes("spreadsheet") ||
    file.type.includes("excel")
  );
}

async function findExistingPersonnel(row: {
  emailPerso?: string;
  emailPro?: string;
}): Promise<PersonnelRecord | null> {
  for (const email of collectOcrEmails(row.emailPro, row.emailPerso)) {
    const hit = await findPersonnelByEmail(email);
    if (hit) return hit;
  }
  return null;
}

async function resolveExternalUserId(row: { emailPerso?: string; emailPro?: string }) {
  for (const email of collectOcrEmails(row.emailPro, row.emailPerso)) {
    const directoryUser = await findDirectoryMemberByEmail(email);
    if (directoryUser?.externalUserId) return directoryUser.externalUserId;
  }
  return null;
}

export async function POST(req: Request) {
  try {
    const gate = await requireAdmin();
    if (!gate.ok) return gate.response;

    const formData = await req.formData();
    const file = formData.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier requis." }, { status: 400 });
    }
    if (!isExcelFile(file)) {
      return NextResponse.json({ error: "Format Excel (.xlsx, .xls) requis." }, { status: 400 });
    }

    const parsed = parsePersonnelExcelBuffer(await file.arrayBuffer());
    if (!parsed.ok) {
      return NextResponse.json({ error: parsed.error }, { status: 400 });
    }

    const now = new Date().toISOString();
    let created = 0;
    let updated = 0;
    const errors: string[] = [];

    for (const row of parsed.rows) {
      const label = row.emailPro || row.emailPerso || `${row.lastName} ${row.firstName}`;
      try {
        const existing = await findExistingPersonnel(row);
        if (existing) {
          const next: PersonnelRecord = normalizePersonnelRecord({
            ...existing,
            firstName: row.firstName,
            lastName: row.lastName,
            displayName: `${row.firstName} ${row.lastName}`.trim(),
            category: row.category as PersonnelCategory,
            jobTitle: row.jobTitle ?? existing.jobTitle,
            emailPerso: row.emailPerso ?? existing.emailPerso,
            emailPro: row.emailPro ?? existing.emailPro,
            updatedAt: now,
          });
          await savePersonnelRecord(next);
          updated++;
          continue;
        }

        const externalUserId = await resolveExternalUserId(row);
        const record: PersonnelRecord = normalizePersonnelRecord({
          id: uid("p"),
          externalUserId,
          emailPro: row.emailPro,
          emailPerso: row.emailPerso,
          firstName: row.firstName,
          lastName: row.lastName,
          displayName: `${row.firstName} ${row.lastName}`.trim(),
          category: row.category,
          jobTitle: row.jobTitle,
          hireDate: null,
          active: true,
          createdAt: now,
          updatedAt: now,
          documents: [],
          formations: [],
          habilitations: [],
          medecineTravail: defaultMedecineTravail(),
          entretiens: [],
          onboarding: null,
        });
        await savePersonnelRecord(record);
        created++;
      } catch (e) {
        errors.push(`${label} : ${e instanceof Error ? e.message : "erreur"}`);
      }
    }

    const message = `${created} créé(s), ${updated} mis à jour — ${created + updated} dossier(s) personnel OGEC. Pensez à synchroniser les dossiers OneDrive.`;

    return NextResponse.json({
      success: true,
      created,
      updated,
      skipped: [...parsed.skipped, ...errors].slice(0, 25),
      message,
    });
  } catch (error: unknown) {
    console.error("[personnel/import]", error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Import impossible." },
      { status: 500 },
    );
  }
}
