import "server-only";

/** Masque IBAN pour affichage (conserve 4 derniers caractères). */
export function maskIban(iban: string | null | undefined): string | null {
  const raw = String(iban ?? "").replace(/\s/g, "").toUpperCase();
  if (!raw) return null;
  if (raw.length <= 4) return "****";
  return `${"*".repeat(Math.max(0, raw.length - 4))}${raw.slice(-4)}`;
}

export type SepaDebitRow = {
  endToEndId: string;
  amount: number;
  debtorName: string;
  debtorIban: string;
  debtorBic: string;
  mandateId: string;
  mandateDate: string;
  remittanceInfo: string;
};

export type SepaPain008Input = {
  messageId: string;
  creationDateTime: string;
  initiatingPartyName: string;
  creditorName: string;
  creditorIban: string;
  creditorBic: string;
  creditorId: string;
  collectionDate: string;
  debits: SepaDebitRow[];
};

function xmlEscape(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function normalizeIban(iban: string): string {
  return iban.replace(/\s/g, "").toUpperCase();
}

/** Génère un fichier pain.008.001.02 (prélèvement SEPA) minimal. */
export function buildPain008Xml(input: SepaPain008Input): string {
  if (!input.debits.length) throw new Error("Aucun prélèvement SEPA à exporter.");
  const ns = "urn:iso:std:iso:20022:tech:xsd:pain.008.001.02";
  const ctrlSum = input.debits.reduce((acc, d) => acc + d.amount, 0).toFixed(2);
  const nbTx = String(input.debits.length);

  const txBlocks = input.debits
    .map((d) => {
      const amt = d.amount.toFixed(2);
      return `<DrctDbtTxInf>
  <PmtId><EndToEndId>${xmlEscape(d.endToEndId)}</EndToEndId></PmtId>
  <InstdAmt Ccy="EUR">${amt}</InstdAmt>
  <DrctDbtTx>
    <MndtRltdInf>
      <MndtId>${xmlEscape(d.mandateId)}</MndtId>
      <DtOfSgntr>${xmlEscape(d.mandateDate)}</DtOfSgntr>
    </MndtRltdInf>
  </DrctDbtTx>
  <DbtrAgt><FinInstnId><BIC>${xmlEscape(d.debtorBic || "NOTPROVIDED")}</BIC></FinInstnId></DbtrAgt>
  <Dbtr><Nm>${xmlEscape(d.debtorName)}</Nm></Dbtr>
  <DbtrAcct><Id><IBAN>${normalizeIban(d.debtorIban)}</IBAN></Id></DbtrAcct>
  <RmtInf><Ustrd>${xmlEscape(d.remittanceInfo)}</Ustrd></RmtInf>
</DrctDbtTxInf>`;
    })
    .join("\n");

  return `<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="${ns}">
  <CstmrDrctDbtInitn>
    <GrpHdr>
      <MsgId>${xmlEscape(input.messageId)}</MsgId>
      <CreDtTm>${xmlEscape(input.creationDateTime)}</CreDtTm>
      <NbOfTxs>${nbTx}</NbOfTxs>
      <CtrlSum>${ctrlSum}</CtrlSum>
      <InitgPty><Nm>${xmlEscape(input.initiatingPartyName)}</Nm></InitgPty>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>${xmlEscape(input.messageId)}-PMT</PmtInfId>
      <PmtMtd>DD</PmtMtd>
      <NbOfTxs>${nbTx}</NbOfTxs>
      <CtrlSum>${ctrlSum}</CtrlSum>
      <PmtTpInf><SvcLvl><Cd>SEPA</Cd></SvcLvl><LclInstrm><Cd>CORE</Cd></LclInstrm><SeqTp>RCUR</SeqTp></PmtTpInf>
      <ReqdColltnDt>${xmlEscape(input.collectionDate)}</ReqdColltnDt>
      <Cdtr><Nm>${xmlEscape(input.creditorName)}</Nm></Cdtr>
      <CdtrAcct><Id><IBAN>${normalizeIban(input.creditorIban)}</IBAN></Id></CdtrAcct>
      <CdtrAgt><FinInstnId><BIC>${xmlEscape(input.creditorBic)}</BIC></FinInstnId></CdtrAgt>
      <CdtrSchmeId><Id><PrvtId><Othr><Id>${xmlEscape(input.creditorId)}</Id><SchmeNm><Prtry>SEPA</Prtry></SchmeNm></Othr></PrvtId></Id></CdtrSchmeId>
      ${txBlocks}
    </PmtInf>
  </CstmrDrctDbtInitn>
</Document>`;
}

export function resolveSepaCreditorConfig(): {
  name: string;
  iban: string;
  bic: string;
  creditorId: string;
} {
  const name = process.env.SEPA_CREDITOR_NAME?.trim() || "Établissement scolaire";
  const iban = process.env.SEPA_CREDITOR_IBAN?.trim() || "";
  const bic = process.env.SEPA_CREDITOR_BIC?.trim() || "";
  const creditorId = process.env.SEPA_CREDITOR_ID?.trim() || "";
  if (!iban || !bic || !creditorId) {
    throw new Error(
      "Configuration SEPA incomplète — renseignez SEPA_CREDITOR_IBAN, SEPA_CREDITOR_BIC et SEPA_CREDITOR_ID.",
    );
  }
  return { name, iban, bic, creditorId };
}
