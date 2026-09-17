import { NextResponse } from "next/server";
import { and, ilike, isNull, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { refEtablissement } from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

/**
 * Recherche publique (lien fiche de dialogue / RDV) dans le référentiel RNE.
 * Query : ?q=…&dept=076&cp=76500&limit=30
 */
export async function GET(req: Request) {
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim() || "";
  const dept = url.searchParams.get("dept")?.trim() || "";
  const cp = url.searchParams.get("cp")?.trim().replace(/\s+/g, "") || "";
  const limit = Math.min(50, Math.max(1, Number(url.searchParams.get("limit") || 30)));

  if (!q && !dept && !cp) {
    return NextResponse.json({
      etablissements: [],
      hint: "Indiquez un code postal, un département (ex. 076) et/ou un nom d’établissement.",
    });
  }

  const db = getDb();
  const conditions = [isNull(refEtablissement.dateFermeture)];

  if (dept) {
    const d = dept.replace(/[^0-9A-Za-z]/g, "").toUpperCase().slice(0, 3);
    if (d) {
      conditions.push(sql`${refEtablissement.codeRne} ILIKE ${`${d}%`}`);
    }
  }
  if (cp) {
    const cpDigits = cp.replace(/\D+/g, "").slice(0, 5);
    if (cpDigits.length >= 2) {
      conditions.push(ilike(refEtablissement.adresse, `%${cpDigits}%`));
    }
  }
  if (q.length >= 2) {
    const pattern = `%${q}%`;
    conditions.push(
      or(
        ilike(refEtablissement.denomPrinc, pattern),
        ilike(refEtablissement.denomCompl, pattern),
        ilike(refEtablissement.sigle, pattern),
        ilike(refEtablissement.codeRne, pattern),
        ilike(refEtablissement.adresse, pattern),
      )!,
    );
  }

  const rows = await db
    .select({
      codeRne: refEtablissement.codeRne,
      denomPrinc: refEtablissement.denomPrinc,
      denomCompl: refEtablissement.denomCompl,
      sigle: refEtablissement.sigle,
      adresse: refEtablissement.adresse,
      codeNature: refEtablissement.codeNature,
    })
    .from(refEtablissement)
    .where(and(...conditions))
    .limit(limit);

  return NextResponse.json({
    etablissements: rows.map((r) => ({
      codeRne: r.codeRne,
      label: [r.denomPrinc, r.denomCompl, r.sigle].filter(Boolean).join(" — ") || r.codeRne,
      adresse: r.adresse,
      codeNature: r.codeNature,
    })),
  });
}
