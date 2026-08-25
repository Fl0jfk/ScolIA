import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getFoyerFacturation, listFactures } from "@/app/lib/facturation-db";
import { buildPain008Xml, resolveSepaCreditorConfig, type SepaDebitRow } from "@/app/lib/facturation-sepa";
import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { foyer, foyerResponsable } from "@/db/schema";

export async function POST(req: Request) {
  const gate = await requireModule("facturation-familles");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => ({}));
  const factureIds = Array.isArray(body.factureIds)
    ? body.factureIds.map(String).filter(Boolean)
    : [];
  const collectionDate =
    String(body.collectionDate || "").trim() ||
    new Date(Date.now() + 7 * 86400000).toISOString().slice(0, 10);

  const factures = await listFactures(etabId);
  const selected = factureIds.length
    ? factures.filter((f) => factureIds.includes(f.id))
    : factures.filter((f) => f.statut === "emise");

  if (!selected.length) {
    return NextResponse.json({ error: "Aucune facture émise à prélever." }, { status: 400 });
  }

  const creditor = resolveSepaCreditorConfig();
  const db = getDb();
  const debits: SepaDebitRow[] = [];

  for (const fac of selected) {
    const ff = await getFoyerFacturation(etabId, fac.foyerId);
    if (!ff?.acceptePrelevement || !ff.iban || !ff.rum) continue;

    const [f] = await db
      .select({ label: foyer.label })
      .from(foyer)
      .where(and(eq(foyer.etablissementId, etabId), eq(foyer.id, fac.foyerId)))
      .limit(1);
    const [payeur] = await db
      .select({ nom: foyerResponsable.nom, prenom: foyerResponsable.prenom })
      .from(foyerResponsable)
      .where(
        and(
          eq(foyerResponsable.etablissementId, etabId),
          eq(foyerResponsable.foyerId, fac.foyerId),
          eq(foyerResponsable.payeur, true),
        ),
      )
      .limit(1);

    debits.push({
      endToEndId: fac.numero.slice(0, 35),
      amount: Number(fac.totalTtc),
      debtorName: payeur ? `${payeur.prenom} ${payeur.nom}`.trim() : f?.label || "Payeur",
      debtorIban: ff.iban,
      debtorBic: ff.bic || "NOTPROVIDED",
      mandateId: ff.rum,
      mandateDate: ff.mandatDate ? String(ff.mandatDate) : collectionDate,
      remittanceInfo: `Facture ${fac.numero}`,
    });
  }

  if (!debits.length) {
    return NextResponse.json(
      { error: "Aucun foyer éligible SEPA (IBAN, RUM, mandat actif requis)." },
      { status: 400 },
    );
  }

  const now = new Date();
  const messageId = `SEPA-${now.toISOString().replace(/[-:TZ.]/g, "").slice(0, 14)}`;
  const xml = buildPain008Xml({
    messageId,
    creationDateTime: now.toISOString(),
    initiatingPartyName: creditor.name,
    creditorName: creditor.name,
    creditorIban: creditor.iban,
    creditorBic: creditor.bic,
    creditorId: creditor.creditorId,
    collectionDate,
    debits,
  });

  return new NextResponse(xml, {
    headers: {
      "Content-Type": "application/xml; charset=utf-8",
      "Content-Disposition": `attachment; filename="${messageId}.xml"`,
    },
  });
}
