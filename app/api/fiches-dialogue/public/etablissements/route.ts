import { NextResponse } from "next/server";
import { and, ilike, isNull, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { refEtablissement } from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { searchAnnuaireEducation } from "@/app/lib/annuaire-education-search";

type EtabHit = {
  codeRne: string;
  label: string;
  adresse: string | null;
  codeNature: string | null;
};

function hasUsableLabel(hit: EtabHit): boolean {
  const label = hit.label.trim();
  return Boolean(label) && label.toUpperCase() !== hit.codeRne.toUpperCase();
}

/**
 * Recherche publique (fiche de dialogue / RDV) :
 * 1. `ref_etablissement` (import Siècle Etablissements.xml)
 * 2. Annuaire national MEN si le référentiel local est vide / incomplet
 *    (ex. seuls les UAJ Communs sans libellé).
 *
 * Query : ?q=…&dept=076&cp=76500&limit=80
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
  const limit = Math.min(100, Math.max(1, Number(url.searchParams.get("limit") || 80)));

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

  const localHits: EtabHit[] = rows.map((r) => ({
    codeRne: r.codeRne,
    label: [r.denomPrinc, r.denomCompl, r.sigle].filter(Boolean).join(" — ") || r.codeRne,
    adresse: r.adresse,
    codeNature: r.codeNature,
  }));

  const usableLocal = localHits.filter(hasUsableLabel);
  // Recherche par CP : toujours croiser l’annuaire national (Siècle local est souvent incomplet).
  // Sinon, compléter si moins de 8 libellés utiles.
  const needAnnuaire = Boolean(cp) || usableLocal.length < 8;

  let etablissements: EtabHit[] = usableLocal;
  let source: "siecle" | "annuaire" | "mixte" = "siecle";

  if (needAnnuaire) {
    const national = await searchAnnuaireEducation({ q, dept, cp, limit });
    const byRne = new Map<string, EtabHit>();
    for (const hit of national) byRne.set(hit.codeRne.toUpperCase(), hit);
    for (const hit of usableLocal) byRne.set(hit.codeRne.toUpperCase(), hit);
    etablissements = [...byRne.values()].slice(0, limit);
    source = usableLocal.length ? "mixte" : "annuaire";
  }

  return NextResponse.json({
    etablissements,
    source,
    hint:
      etablissements.length === 0
        ? "Aucun établissement trouvé — affinez le code postal, le département (076) ou le nom."
        : undefined,
  });
}
