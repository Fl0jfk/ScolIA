import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import * as XLSX from "xlsx";
import { getTableName } from "drizzle-orm";
import { applyPlan, ApplyPartialError, type FoyersDb, type FoyersWriter } from "./apply";
import {
  autoriteFromLien,
  isGenericEmail,
  excelSerialToIso,
  normEmail,
  normName,
  normPhone,
  parseBirthDate,
} from "./normalize";
import { assertSafeXlsx, FoyersFileError, parseFoyersWorkbook, rowsFromRecords } from "./parse";
import { validateResponsableInput } from "./responsable-validate";
import { pickFoyerByEleve } from "../foyer-pick";
import { filterStageSiblings } from "../stage-siblings-filter";
import { elevesMatchingParentContact } from "../rdv-inscription-match";
import { pickIdentityRecipients } from "../stage-identity-pick";
import { filterFoyersForProfClasses, searchPaginateFoyers } from "../famille-messaging-matrix";
import {
  analyzeFile,
  buildPlan,
  fileFingerprint,
  planFingerprint,
  planIsNoop,
  type DbEleve,
  type DbResp,
  type ExistingData,
} from "./plan";
import { buildReportCsv } from "./report";
import { canManageFoyersRoles } from "./roles";

/* ───────── Données synthétiques ───────── */

const TODAY = "2026-11-25";
const DOB = 39000; // numéro de série Excel

function rec(over: Record<string, unknown>): Record<string, unknown> {
  return {
    Nom: "Martin",
    Prénom: "Léa",
    "Code classe": "6A",
    "Date de naissance": DOB,
    "Lien de parenté": "Mère",
    "Lien de parenté avec le conjoint": "Père",
    "Nom Resp": "Martin",
    "Prénom Resp": "Claire",
    "Email Personnel Resp": "claire@example.org",
    "Tél. portable Resp": "06 11 22 33 44",
    "Nom Conjoint": "Martin",
    "Prénom Conjoint": "Paul",
    "Email Personnel Conjoint": "paul@example.org",
    "Tél. portable Conjoint": "0655667788",
    ...over,
  };
}

const dobIso = excelSerialToIso(DOB)!;

function eleve(id: string, nom: string, prenom: string, over: Partial<DbEleve> = {}): DbEleve {
  return { id, nom, prenom, dateNaissance: dobIso, classe: "6A", status: "inscrit", dateSortie: null, ...over };
}

function analyze(records: Array<Record<string, unknown>>, eleves: DbEleve[]) {
  return analyzeFile(rowsFromRecords(records), eleves, TODAY);
}

const EMPTY: ExistingData = { foyers: [], responsables: [], links: [] };

/* ───────── Normalisation ───────── */

test("normalisation : accents, casse, séparateurs", () => {
  assert.equal(normName("  Éloïse-Marie  "), "eloise marie");
  assert.equal(normName("D'ARC"), normName("d’arc"));
  assert.equal(normEmail(" Jean@Example.ORG "), "jean@example.org");
  assert.equal(normEmail("pas-un-mail"), null);
});

test("normalisation : téléphones FR", () => {
  assert.equal(normPhone("06 11 22 33 44"), "0611223344");
  assert.equal(normPhone("+33 6 11 22 33 44"), "0611223344");
  assert.equal(normPhone("611223344"), "0611223344");
  assert.equal(normPhone("12"), null);
  assert.equal(normPhone(""), null);
});

test("normalisation : date de naissance (série Excel, Date, JJ/MM/AAAA)", () => {
  assert.equal(excelSerialToIso(25569), "1970-01-01");
  assert.equal(parseBirthDate(39000), excelSerialToIso(39000));
  assert.equal(parseBirthDate("39000"), excelSerialToIso(39000));
  assert.equal(parseBirthDate("05/03/2012"), "2012-03-05");
  assert.equal(parseBirthDate("2012-03-05"), "2012-03-05");
  assert.equal(parseBirthDate(new Date(2012, 2, 5)), "2012-03-05");
  assert.equal(parseBirthDate("31/02/2012"), null);
  assert.equal(parseBirthDate("n'importe quoi"), null);
});

test("autorité parentale selon le lien", () => {
  for (const l of ["Père", "Mère", "Famille d'accueil", "Organisme (ASE)", "Oncle"]) {
    assert.equal(autoriteFromLien(l).value, true, l);
  }
  for (const l of ["Beau-père", "Belle-mère", "Autre"]) {
    assert.deepEqual(autoriteFromLien(l), { value: false, known: true }, l);
  }
  assert.equal(autoriteFromLien("Cousin").known, false);
});

test("lecture d'un classeur .xlsx synthétique", () => {
  const ws = XLSX.utils.json_to_sheet([rec({}), rec({ Nom: "Durand", "Prénom": "Tom" })]);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "Feuil1");
  const buf = XLSX.write(wb, { type: "buffer", bookType: "xlsx" }) as Buffer;
  const rows = parseFoyersWorkbook(buf);
  assert.equal(rows.length, 2);
  assert.equal(rows[0]!.dateNaissance, dobIso);
  assert.equal(rows[0]!.adults.length, 2);
  assert.equal(rows[0]!.adults[0]!.telephone, "0611223344");
  // colonnes manquantes
  const bad = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(bad, XLSX.utils.json_to_sheet([{ A: 1 }]), "x");
  assert.throws(() => parseFoyersWorkbook(XLSX.write(bad, { type: "buffer", bookType: "xlsx" }) as Buffer));
});

/* ───────── Matching élèves ───────── */

test("matching : reconnu / non trouvé / ambigu (actif prioritaire)", () => {
  const eleves = [
    eleve("e1", "MARTIN", "Léa"),
    eleve("e2", "Dupont", "Hugo", { status: "ancien" }),
    eleve("e3", "Dupont", "Hugo", { status: "inscrit" }),
    eleve("e4", "Bernard", "Zoé"),
    eleve("e5", "Bernard", "Zoé"),
  ];
  const a = analyze(
    [
      rec({}),
      rec({ Nom: "Dupont", Prénom: "Hugo" }),
      rec({ Nom: "Bernard", Prénom: "Zoé" }),
      rec({ Nom: "Inconnu", Prénom: "Zed" }),
    ],
    eleves,
  );
  const by = (n: string) => a.students.find((s) => s.nom === n)!;
  assert.deepEqual(by("Martin").match, { status: "reconnu", eleveId: "e1", classe: "6A" });
  assert.equal((by("Dupont").match as { eleveId: string }).eleveId, "e3");
  assert.equal(by("Bernard").match.status, "ambigu");
  assert.equal(by("Inconnu").match.status, "non_trouve");
  assert.ok(a.anomalies.some((x) => x.kind === "ambigu"));
  assert.ok(a.anomalies.some((x) => x.kind === "non_trouve"));
  // le code classe n'est pas bloquant
  assert.equal(by("Martin").codeClasse, "6A");
});

test("matching : le code classe du fichier ne bloque pas", () => {
  const a = analyze([rec({ "Code classe": "C2_CE1D" })], [eleve("e1", "Martin", "Léa", { classe: "CE1 D" })]);
  assert.equal(a.students[0]!.match.status, "reconnu");
});

/* ───────── Reconstitution des foyers ───────── */

test("un couple = 1 foyer à 2 responsables", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  assert.equal(a.foyers.size, 1);
  assert.equal([...a.foyers.values()][0]!.adults.length, 2);
});

test("parents séparés = 2 foyers pour un même élève", () => {
  const a = analyze(
    [
      rec({ "Nom Conjoint": "", "Prénom Conjoint": "", "Email Personnel Conjoint": "", "Tél. portable Conjoint": "" }),
      rec({
        "Nom Resp": "Martin",
        "Prénom Resp": "Paul",
        "Email Personnel Resp": "paul@example.org",
        "Tél. portable Resp": "0655667788",
        "Lien de parenté": "Père",
        "Nom Conjoint": "Roux",
        "Prénom Conjoint": "Anne",
        "Email Personnel Conjoint": "",
        "Tél. portable Conjoint": "",
        "Lien de parenté avec le conjoint": "Belle-mère",
      }),
    ],
    [eleve("e1", "Martin", "Léa")],
  );
  assert.equal(a.foyers.size, 2);
  assert.equal(a.students.length, 1);
  assert.equal(a.students[0]!.foyerKeys.length, 2);
});

test("fratrie de 10 enfants = 1 foyer", () => {
  const recs = Array.from({ length: 10 }, (_, i) => rec({ Prénom: `Enfant${i}` }));
  const eleves = recs.map((r, i) => eleve(`e${i}`, "Martin", `Enfant${i}`));
  const a = analyze(recs, eleves);
  assert.equal(a.foyers.size, 1);
  assert.equal([...a.foyers.values()][0]!.studentKeys.length, 10);
  assert.equal(a.matchedEleveIds.length, 10);
  const plan = buildPlan(a, EMPTY, { keepOldLinks: false });
  assert.equal(plan.counts.foyersCrees, 1);
  assert.equal(plan.counts.liensAjoutes, 10);
});

test("e-mail partagé par un couple : les deux responsables sont gardés", () => {
  const a = analyze(
    [rec({ "Email Personnel Conjoint": "claire@example.org" })],
    [eleve("e1", "Martin", "Léa")],
  );
  const adults = [...a.foyers.values()][0]!.adults;
  assert.equal(adults.length, 2);
  assert.equal(adults[0]!.email, adults[1]!.email);
  // même foyer retrouvé pour un frère avec le même e-mail commun
  const a2 = analyze(
    [
      rec({ "Email Personnel Conjoint": "claire@example.org" }),
      rec({ Prénom: "Tom", "Email Personnel Conjoint": "claire@example.org" }),
    ],
    [eleve("e1", "Martin", "Léa"), eleve("e2", "Martin", "Tom")],
  );
  assert.equal(a2.foyers.size, 1);
});

test("doublon de ligne ignoré, responsable sans contact signalé", () => {
  const a = analyze(
    [
      rec({}),
      rec({}),
      rec({
        Nom: "Seul",
        Prénom: "Ana",
        "Nom Resp": "Seul",
        "Prénom Resp": "Bob",
        "Email Personnel Resp": "",
        "Tél. portable Resp": "",
        "Nom Conjoint": "",
        "Prénom Conjoint": "",
        "Email Personnel Conjoint": "",
        "Tél. portable Conjoint": "",
      }),
    ],
    [eleve("e1", "Martin", "Léa"), eleve("e2", "Seul", "Ana")],
  );
  assert.equal(a.anomalies.filter((x) => x.kind === "doublon").length, 1);
  assert.equal(a.anomalies.filter((x) => x.kind === "sans_contact").length, 1);
  assert.equal(a.foyers.size, 2);
});

/* ───────── Plan face à l'existant ───────── */

function resp(over: Partial<DbResp> & { id: string; foyerId: string }): DbResp {
  return {
    nom: "X",
    prenom: "Y",
    email: null,
    telephone: null,
    autoriteParentale: false,
    payeur: false,
    userId: null,
    rang: 1,
    ...over,
  };
}

test("mise à jour en place d'un foyer existant (par e-mail), ajout du conjoint, complétion du téléphone", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const existing: ExistingData = {
    foyers: [{ id: "f1", label: "Foyer ancien" }],
    responsables: [
      resp({
        id: "r1",
        foyerId: "f1",
        nom: "MARTIN-DUPUIS",
        prenom: "Claire",
        email: "CLAIRE@example.org",
        telephone: null,
        payeur: true,
        autoriteParentale: true,
      }),
    ],
    links: [{ eleveId: "e1", foyerId: "f1" }],
  };
  const plan = buildPlan(a, existing, { keepOldLinks: false });
  assert.equal(plan.foyers.length, 1);
  const f = plan.foyers[0]!;
  assert.equal(f.existingId, "f1");
  assert.equal(f.action, "update");
  assert.equal(f.label, "Foyer ancien", "le libellé existant n'est pas renommé");
  const r1 = f.responsables.find((r) => r.respId === "r1")!;
  assert.deepEqual(r1.changes.sort(), ["nom", "téléphone"].sort());
  assert.equal(r1.hasUser, false);
  assert.equal(r1.autoriteParentale, true);
  const created = f.responsables.find((r) => r.action === "create")!;
  assert.equal(created.prenom, "Paul");
  assert.equal(created.rang, 2);
  assert.equal(plan.counts.foyersCrees, 0);
  assert.equal(plan.counts.responsablesAjoutes, 1);
  assert.equal(plan.counts.responsablesModifies, 1);
  assert.equal(plan.counts.liensAjoutes, 0);
  assert.equal(plan.counts.liensRetires, 0);
});

test("rattachement par nom + prénom sans e-mail ni téléphone", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const plan = buildPlan(
    a,
    {
      foyers: [{ id: "f1", label: "F" }],
      responsables: [resp({ id: "r1", foyerId: "f1", nom: "martin", prenom: "CLAIRE" })],
      links: [{ eleveId: "e1", foyerId: "f1" }],
    },
    { keepOldLinks: false },
  );
  assert.equal(plan.foyers[0]!.existingId, "f1");
  assert.equal(plan.foyers[0]!.responsables[0]!.respId, "r1");
});

test("retrait de lien vers un foyer absent du fichier : foyer orphelin conservé", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const existing: ExistingData = {
    foyers: [
      { id: "f1", label: "OK" },
      { id: "fOld", label: "Ancien foyer" },
    ],
    responsables: [
      resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", email: "claire@example.org" }),
      resp({ id: "rOld", foyerId: "fOld", nom: "Autre", prenom: "Gens", payeur: true }),
    ],
    links: [
      { eleveId: "e1", foyerId: "f1" },
      { eleveId: "e1", foyerId: "fOld" },
    ],
  };
  const plan = buildPlan(a, existing, { keepOldLinks: false });
  assert.deepEqual(plan.linksRemove, [{ eleveId: "e1", foyerId: "fOld" }]);
  assert.equal(plan.orphelins.length, 1);
  assert.equal(plan.orphelins[0]!.foyerId, "fOld");
  // aucun plan de suppression de foyer ni de responsable
  assert.ok(plan.foyers.every((f) => f.ref !== "fOld"));

  const keep = buildPlan(a, existing, { keepOldLinks: true });
  assert.equal(keep.linksRemove.length, 0);
  assert.equal(keep.orphelins.length, 0);
});

test("responsable existant absent du fichier (compte / payeur) : conservé", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const plan = buildPlan(
    a,
    {
      foyers: [{ id: "f1", label: "F" }],
      responsables: [
        resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", email: "claire@example.org", rang: 1 }),
        resp({ id: "r2", foyerId: "f1", nom: "Martin", prenom: "Paul", email: "paul@example.org", rang: 2 }),
        resp({ id: "r3", foyerId: "f1", nom: "Tiers", prenom: "Payeur", payeur: true, userId: "u3", rang: 3 }),
      ],
      links: [{ eleveId: "e1", foyerId: "f1" }],
    },
    { keepOldLinks: false },
  );
  const f = plan.foyers[0]!;
  assert.deepEqual(f.conserves.map((c) => c.id), ["r3"]);
  assert.equal(f.conserves[0]!.payeur, true);
  assert.equal(f.conserves[0]!.hasUser, true);
});

test("foyer plein (4 responsables) : pas de cinquième", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const resps = [1, 2, 3, 4].map((i) =>
    resp({ id: `r${i}`, foyerId: "f1", nom: i === 1 ? "Martin" : `Z${i}`, prenom: i === 1 ? "Claire" : `Z${i}`, rang: i, email: i === 1 ? "claire@example.org" : null }),
  );
  const plan = buildPlan(
    a,
    { foyers: [{ id: "f1", label: "F" }], responsables: resps, links: [{ eleveId: "e1", foyerId: "f1" }] },
    { keepOldLinks: false },
  );
  assert.equal(plan.counts.responsablesAjoutes, 0);
  assert.ok(plan.anomalies.some((x) => x.kind === "foyer_plein"));
});

test("autorité parentale : calculée selon le lien, jamais rétrogradée", () => {
  const a = analyze(
    [rec({ "Lien de parenté": "Beau-père", "Lien de parenté avec le conjoint": "Père" })],
    [eleve("e1", "Martin", "Léa")],
  );
  // nouveau foyer
  const plan = buildPlan(a, EMPTY, { keepOldLinks: false });
  const [r1, r2] = plan.foyers[0]!.responsables;
  assert.equal(r1!.autoriteParentale, false);
  assert.equal(r2!.autoriteParentale, true);
  // existant déjà true avec lien Beau-père : reste true
  const plan2 = buildPlan(
    a,
    {
      foyers: [{ id: "f1", label: "F" }],
      responsables: [
        resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", email: "claire@example.org", autoriteParentale: true }),
      ],
      links: [{ eleveId: "e1", foyerId: "f1" }],
    },
    { keepOldLinks: false },
  );
  const upd = plan2.foyers[0]!.responsables.find((r) => r.respId === "r1")!;
  assert.equal(upd.autoriteParentale, true);
  assert.ok(!upd.changes.includes("autorité parentale"));
});

test("élèves non trouvés / ambigus : aucun foyer ni lien", () => {
  const a = analyze([rec({ Nom: "Inconnu" })], [eleve("e1", "Martin", "Léa")]);
  const plan = buildPlan(a, EMPTY, { keepOldLinks: false });
  assert.equal(plan.foyers.length, 0);
  assert.equal(plan.linksAdd.length, 0);
  assert.equal(plan.counts.elevesNonTrouves, 1);
});

/* ───────── Application, idempotence, aucune écriture sur eleve ───────── */

type Row = Record<string, any>;

/** Base mémoire minimale qui comprend les écritures de apply.ts. */
function fakeDb() {
  const tables: Record<string, Row[]> = {
    foyer: [],
    foyer_responsable: [],
    eleve_foyer_link: [],
    eleve_access_audit: [],
  };
  const touched: Array<{ op: string; table: string; set?: Row }> = [];
  let seq = 0;

  function params(node: any, out: unknown[] = []): unknown[] {
    if (node === null || typeof node !== "object") return out;
    if (Array.isArray(node)) {
      node.forEach((n) => params(n, out));
      return out;
    }
    if (node.constructor?.name === "Param") {
      out.push(node.value);
      return out;
    }
    if (Array.isArray(node.queryChunks)) node.queryChunks.forEach((n: unknown) => params(n, out));
    return out;
  }

  const tx: FoyersWriter = {
    insert(table: unknown) {
      const name = getTableName(table as never);
      return {
        values(v: Row | Row[]) {
          touched.push({ op: "insert", table: name });
          const rows = (Array.isArray(v) ? v : [v]).map((r) => ({ id: `${name}-${++seq}`, ...r }));
          const run = (skipDup: boolean) => {
            for (const r of rows) {
              if (
                skipDup &&
                name === "eleve_foyer_link" &&
                tables[name]!.some((x) => x.eleveId === r.eleveId && x.foyerId === r.foyerId)
              )
                continue;
              tables[name]!.push(r);
            }
          };
          const chain: any = {
            returning: async () => {
              run(false);
              return rows.map((r) => ({ id: r.id }));
            },
            onConflictDoNothing: async () => run(true),
            then: (res: (v: unknown) => void) => {
              run(false);
              res(undefined);
            },
          };
          return chain;
        },
      };
    },
    update(table: unknown) {
      const name = getTableName(table as never);
      return {
        set(set: Row) {
          touched.push({ op: "update", table: name, set });
          return {
            where: async (cond: unknown) => {
              const ps = params(cond);
              const rows = tables[name]!.filter((r) => ps.includes(r.id));
              assert.ok(rows.length, `update ${name} introuvable`);
              for (const row of rows) Object.assign(row, set);
            },
          };
        },
      };
    },
    delete(table: unknown) {
      const name = getTableName(table as never);
      touched.push({ op: "delete", table: name });
      return {
        where: async (cond: unknown) => {
          const [, eleveId, ...foyerIds] = params(cond);
          tables[name] = tables[name]!.filter((r) => !(r.eleveId === eleveId && foyerIds.includes(r.foyerId)));
        },
      };
    },
  };
  const db: FoyersDb = { transaction: async (fn) => fn(tx) };
  return { db, tables, touched };
}

function existingFrom(tables: Record<string, Row[]>): ExistingData {
  return {
    foyers: tables.foyer!.map((f) => ({ id: f.id, label: f.label })),
    responsables: tables.foyer_responsable!.map((r) => ({
      id: r.id,
      foyerId: r.foyerId,
      nom: r.nom,
      prenom: r.prenom,
      email: r.email ?? null,
      telephone: r.telephone ?? null,
      autoriteParentale: !!r.autoriteParentale,
      payeur: !!r.payeur,
      userId: r.userId ?? null,
      rang: r.rang,
    })),
    links: tables.eleve_foyer_link!.map((l) => ({ eleveId: l.eleveId, foyerId: l.foyerId })),
  };
}

const CTX = { etablissementId: "etab", actorUserId: "admin", importId: "imp" };

test("application : aucune écriture sur eleve, payeur/facturation intacts, idempotence", async () => {
  const recs = [
    rec({}),
    rec({ Prénom: "Tom" }),
    rec({ Nom: "Roux", Prénom: "Ana", "Nom Resp": "Roux", "Prénom Resp": "Bob", "Email Personnel Resp": "bob@example.org", "Nom Conjoint": "", "Prénom Conjoint": "", "Email Personnel Conjoint": "", "Tél. portable Conjoint": "" }),
  ];
  const eleves = [eleve("e1", "Martin", "Léa"), eleve("e2", "Martin", "Tom"), eleve("e3", "Roux", "Ana")];
  const { db, tables, touched } = fakeDb();
  // état initial : un foyer existant avec payeur + compte, lié à e1 et un ancien foyer
  tables.foyer!.push({ id: "f1", label: "Foyer Martin" }, { id: "fOld", label: "Ancien" });
  tables.foyer_responsable!.push({
    id: "r1",
    foyerId: "f1",
    nom: "Martin",
    prenom: "Claire",
    email: "claire@example.org",
    telephone: null,
    autoriteParentale: false,
    payeur: true,
    userId: "u1",
    rang: 1,
  });
  tables.eleve_foyer_link!.push({ eleveId: "e1", foyerId: "f1" }, { eleveId: "e1", foyerId: "fOld" });

  const analysis = analyzeFile(rowsFromRecords(recs), eleves, TODAY);
  const plan = buildPlan(analysis, existingFrom(tables), { keepOldLinks: false });
  const res = await applyPlan(db, plan, CTX);

  // tables écrites : uniquement foyers / responsables / liens / journal
  const allowed = new Set(["foyer", "foyer_responsable", "eleve_foyer_link", "eleve_access_audit"]);
  assert.ok(touched.length > 0);
  for (const t of touched) assert.ok(allowed.has(t.table), `table inattendue ${t.table}`);
  assert.ok(!touched.some((t) => t.table === "eleve"));
  // jamais de modification de payeur / userId / contactUrgence sur l'existant
  for (const t of touched.filter((x) => x.op === "update" && x.table === "foyer_responsable")) {
    for (const k of ["payeur", "userId", "contactUrgence", "peutRecuperer"]) assert.ok(!(k in t.set!), k);
  }
  const r1 = tables.foyer_responsable!.find((r) => r.id === "r1")!;
  assert.equal(r1.payeur, true);
  assert.equal(r1.userId, "u1");
  assert.equal(r1.telephone, null, "responsable avec compte : téléphone non modifié");
  assert.equal(r1.autoriteParentale, true);
  // foyer + responsables non supprimés ; lien retiré ; foyer ancien conservé
  assert.ok(tables.foyer!.some((f) => f.id === "fOld"));
  assert.ok(!tables.eleve_foyer_link!.some((l) => l.eleveId === "e1" && l.foyerId === "fOld"));
  assert.equal(res.applied.liensRetires, 1);
  assert.equal(tables.foyer!.length, 3); // f1, fOld, + foyer Roux
  assert.equal(tables.eleve_foyer_link!.filter((l) => l.foyerId === "f1").length, 2); // fratrie
  assert.ok(tables.eleve_access_audit!.length >= 3);

  // idempotence : même fichier, même base → plus rien à faire
  const plan2 = buildPlan(
    analyzeFile(rowsFromRecords(recs), eleves, TODAY),
    existingFrom(tables),
    { keepOldLinks: false },
  );
  assert.equal(plan2.counts.foyersCrees, 0);
  assert.equal(plan2.counts.foyersMisAJour, 0);
  assert.equal(plan2.counts.responsablesAjoutes, 0);
  assert.equal(plan2.counts.responsablesModifies, 0);
  assert.equal(plan2.counts.liensAjoutes, 0);
  assert.equal(plan2.counts.liensRetires, 0);
  assert.ok(planIsNoop(plan2));
  const before = JSON.stringify(tables);
  await applyPlan(db, plan2, CTX);
  assert.equal(JSON.stringify(tables), before, "relancer le même fichier ne change rien");
});

test("application par lots de 100 : un lot en échec laisse les précédents enregistrés", async () => {
  const n = 250;
  const recs = Array.from({ length: n }, (_, i) =>
    rec({ Prénom: `P${i}`, "Nom Resp": `R${i}`, "Email Personnel Resp": `r${i}@example.org`, "Nom Conjoint": "", "Prénom Conjoint": "", "Email Personnel Conjoint": "", "Tél. portable Conjoint": "" }),
  );
  const eleves = recs.map((_, i) => eleve(`e${i}`, "Martin", `P${i}`));
  const plan = buildPlan(analyzeFile(rowsFromRecords(recs), eleves, TODAY), EMPTY, { keepOldLinks: false });
  const { db, tables } = fakeDb();
  const res = await applyPlan(db, plan, CTX);
  assert.equal(res.batchesTotal, 3);
  assert.equal(res.batchesDone, 3);
  assert.equal(tables.foyer!.length, n);
  assert.equal(tables.eleve_foyer_link!.length, n);

  const second = fakeDb();
  let calls = 0;
  const failing: FoyersDb = {
    transaction: async (fn) => {
      if (++calls === 2) throw new Error("boom");
      return second.db.transaction(fn);
    },
  };
  await assert.rejects(
    () => applyPlan(failing, plan, CTX),
    (err: unknown) =>
      err instanceof ApplyPartialError && err.partial.batchesDone === 1 && err.partial.applied.foyersCrees === 100,
  );
  assert.equal(second.tables.foyer!.length, 100);
});

test("empreintes : stables, sensibles au fichier et aux options", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const p1 = buildPlan(a, EMPTY, { keepOldLinks: false });
  const p2 = buildPlan(a, EMPTY, { keepOldLinks: false });
  const h = fileFingerprint(Buffer.from("abc"));
  assert.equal(planFingerprint(p1, h, { keepOldLinks: false }), planFingerprint(p2, h, { keepOldLinks: false }));
  assert.notEqual(
    planFingerprint(p1, h, { keepOldLinks: false }),
    planFingerprint(p1, fileFingerprint(Buffer.from("abd")), { keepOldLinks: false }),
  );
  assert.notEqual(
    planFingerprint(p1, h, { keepOldLinks: false }),
    planFingerprint(p1, h, { keepOldLinks: true }),
  );
});

test("rapport CSV : en-têtes et neutralisation des formules", () => {
  const a = analyze([rec({ "Prénom Resp": "=CMD()" })], [eleve("e1", "Martin", "Léa")]);
  const csv = buildReportCsv(buildPlan(a, EMPTY, { keepOldLinks: false }));
  assert.ok(csv.startsWith("﻿type;"));
  assert.ok(!csv.includes(";=CMD()"));
  assert.ok(csv.includes("'=CMD()"));
});

/* ───────── Droits et garde-fou « fiche élève » ───────── */

test("droits : admin / direction / administratif uniquement", () => {
  assert.equal(canManageFoyersRoles(["admin"], {}), true);
  assert.equal(canManageFoyersRoles(["direction_college"], {}), true);
  assert.equal(canManageFoyersRoles(["administratif"], {}), true);
  assert.equal(canManageFoyersRoles(["professeur"], {}), false);
  assert.equal(canManageFoyersRoles(["comptabilite"], {}), false);
  assert.equal(canManageFoyersRoles(["accueil"], {}), false);
  assert.equal(canManageFoyersRoles(["parent"], {}), false);
  assert.equal(canManageFoyersRoles([], { orgAdmin: true }), true);
});

test("garde-fou : aucun code de l'import n'écrit dans la table eleve", () => {
  const dir = join(process.cwd(), "app", "lib", "foyers-import");
  for (const f of [
    "apply.ts",
    "plan.ts",
    "service.ts",
    "parse.ts",
    "normalize.ts",
    "report.ts",
    "responsable-edit.ts",
    "responsable-validate.ts",
  ]) {
    const src = readFileSync(join(dir, f), "utf8");
    assert.ok(!/\.(insert|update|delete)\(\s*eleve\s*\)/.test(src), `${f} écrit dans eleve`);
    assert.ok(!/saveElevesRegistry|upsertElevesInDb|EleveConfig|ensureEleveFoyerFromParentContacts/.test(src), `${f} touche la fiche élève`);
  }
  // apply.ts n'importe même pas la table eleve.
  const apply = readFileSync(join(dir, "apply.ts"), "utf8");
  const imp = /import\s*\{([^}]*)\}\s*from "@\/db\/schema"/.exec(apply)![1]!;
  assert.ok(!/\beleve\b/.test(imp));
  // les routes non plus
  for (const f of ["import/route.ts", "route.ts"]) {
    const src = readFileSync(join(process.cwd(), "app", "api", "eleves", "foyers", f), "utf8");
    assert.ok(!/\.(insert|update|delete)\(\s*eleve\s*\)/.test(src));
  }
});

/* ───────── Revue : correctifs ───────── */

const NOADULT = {
  "Nom Resp": "",
  "Prénom Resp": "",
  "Email Personnel Resp": "",
  "Tél. portable Resp": "",
  "Nom Conjoint": "",
  "Prénom Conjoint": "",
  "Email Personnel Conjoint": "",
  "Tél. portable Conjoint": "",
};
const NOCONJ = {
  "Nom Conjoint": "",
  "Prénom Conjoint": "",
  "Email Personnel Conjoint": "",
  "Tél. portable Conjoint": "",
};

test("B1 : élève reconnu sans responsable exploitable = non touché, aucun lien retiré", () => {
  const a = analyze([rec({ ...NOADULT })], [eleve("e1", "Martin", "Léa")]);
  const plan = buildPlan(
    a,
    {
      foyers: [{ id: "f1", label: "F" }],
      responsables: [resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire" })],
      links: [{ eleveId: "e1", foyerId: "f1" }],
    },
    { keepOldLinks: false },
  );
  assert.equal(plan.linksRemove.length, 0);
  assert.equal(plan.students.length, 0);
  assert.ok(plan.anomalies.some((x) => x.kind === "sans_responsable"));
});

test("B2 : responsable avec compte famille : ni nom, ni e-mail, ni téléphone modifiés (différences signalées)", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const plan = buildPlan(
    a,
    {
      foyers: [{ id: "f1", label: "F" }],
      responsables: [
        resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", email: "ancien@example.org", userId: "u1" }),
      ],
      links: [{ eleveId: "e1", foyerId: "f1" }],
    },
    { keepOldLinks: false },
  );
  const r1 = plan.foyers[0]!.responsables.find((r) => r.respId === "r1")!;
  assert.equal(r1.email, "ancien@example.org");
  assert.ok(r1.differences.includes("e-mail"));
  assert.ok(!r1.changes.includes("e-mail"));
  const svc = readFileSync(join(process.cwd(), "app/lib/foyers-import/service.ts"), "utf8");
  assert.ok(svc.includes("linkFoyerResponsableToUserAccountIfUnlinked"));
  assert.ok(!/\blinkFoyerResponsableToUserAccount\(/.test(svc));
  const lk = readFileSync(join(process.cwd(), "app/lib/nomenclature-import/link-responsable-user.ts"), "utf8");
  const variant = lk.slice(lk.indexOf("IfUnlinked"));
  assert.ok(variant.includes("isNull(foyerResponsable.userId)") && variant.includes("foyerResponsable.etablissementId"));
});

test("B3 : liens vers foyers avec facturation / messagerie / compte conservés par défaut", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const base: ExistingData = {
    foyers: [
      { id: "f1", label: "OK" },
      { id: "fFact", label: "Fact", facturation: true },
      { id: "fMsg", label: "Msg", messagerie: true },
      { id: "fUser", label: "User" },
      { id: "fLibre", label: "Libre" },
    ],
    responsables: [
      resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", email: "claire@example.org" }),
      resp({ id: "rU", foyerId: "fUser", nom: "A", prenom: "B", userId: "u" }),
    ],
    links: ["f1", "fFact", "fMsg", "fUser", "fLibre"].map((foyerId) => ({ eleveId: "e1", foyerId })),
  };
  const plan = buildPlan(a, base, { keepOldLinks: false });
  assert.deepEqual(plan.linksRemove, [{ eleveId: "e1", foyerId: "fLibre" }]);
  assert.deepEqual(plan.liensAVerifier.map((l) => l.foyerId).sort(), ["fFact", "fMsg", "fUser"]);
  const all = buildPlan(a, base, { keepOldLinks: false, removeProtectedLinks: true });
  assert.equal(all.linksRemove.length, 4);
  assert.equal(all.liensAVerifier.length, 0);
  assert.notEqual(
    planFingerprint(plan, "h", { keepOldLinks: false }),
    planFingerprint(all, "h", { keepOldLinks: false, removeProtectedLinks: true }),
  );
});

test("M1 : nouveau conjoint avec le portable de la mère = nouveau responsable", () => {
  const a = analyze(
    [
      rec({
        "Nom Conjoint": "Leroy",
        "Prénom Conjoint": "Marc",
        "Email Personnel Conjoint": "",
        "Tél. portable Conjoint": "06 11 22 33 44",
      }),
    ],
    [eleve("e1", "Martin", "Léa")],
  );
  const plan = buildPlan(
    a,
    {
      foyers: [{ id: "f1", label: "F" }],
      responsables: [
        resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", telephone: "0611223344", rang: 1 }),
      ],
      links: [{ eleveId: "e1", foyerId: "f1" }],
    },
    { keepOldLinks: false },
  );
  const rs = plan.foyers[0]!.responsables;
  assert.equal(rs.find((r) => r.prenom === "Marc")!.action, "create");
  assert.equal(rs.find((r) => r.respId === "r1")!.prenom, "Claire");
  const a2 = analyze(
    [
      rec({
        "Nom Resp": "Leroy",
        "Prénom Resp": "Marc",
        "Email Personnel Resp": "",
        "Tél. portable Resp": "0611223344",
        ...NOCONJ,
      }),
    ],
    [eleve("e1", "Martin", "Léa")],
  );
  const p2 = buildPlan(
    a2,
    {
      foyers: [{ id: "f1", label: "F" }],
      responsables: [resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", telephone: "0611223344" })],
      links: [{ eleveId: "e1", foyerId: "f1" }],
    },
    { keepOldLinks: false },
  );
  assert.equal(p2.foyers[0]!.existingId, null, "pas de rattachement sur le seul portable");
});

test("M2 : contact partagé par des adultes de noms différents : non fusionné + anomalie", () => {
  const a = analyze(
    [
      rec({ Prénom: "Lea" }),
      rec({
        Nom: "Autre",
        Prénom: "Zoe",
        "Nom Resp": "Autre",
        "Prénom Resp": "Marie",
        "Email Personnel Resp": "claire@example.org",
        ...NOCONJ,
      }),
    ],
    [eleve("e1", "Martin", "Lea"), eleve("e2", "Autre", "Zoe")],
  );
  assert.equal(a.foyers.size, 2);
  assert.ok(a.anomalies.some((x) => x.kind === "contact_partage"));
});

test("M2 : famille recomposée = un seul foyer ; sous-ensemble rattaché au foyer à 2 adultes", () => {
  const recs = [
    rec({ Prénom: "A" }),
    rec({ Nom: "Dupuis", Prénom: "B" }),
    rec({ Nom: "Roux", Prénom: "C" }),
    rec({ Prénom: "D", ...NOCONJ }),
  ];
  const names = [
    ["Martin", "A"],
    ["Dupuis", "B"],
    ["Roux", "C"],
    ["Martin", "D"],
  ];
  const a = analyze(
    recs,
    names.map(([n, p], i) => eleve(`e${i}`, n!, p!)),
  );
  assert.equal(a.foyers.size, 1);
  assert.equal([...a.foyers.values()][0]!.studentKeys.length, 4);
  assert.equal([...a.foyers.values()][0]!.adults.length, 2);
  const plan = buildPlan(a, EMPTY, { keepOldLinks: false });
  assert.equal(plan.counts.foyersCrees, 1);
  assert.equal(plan.counts.liensAjoutes, 4);
});

test("M4 : ~1800 lignes / 1300 foyers planifiés et appliqués rapidement", async () => {
  const recs: Array<Record<string, unknown>> = [];
  const eleves: DbEleve[] = [];
  for (let i = 0; i < 1800; i++) {
    const fam = i % 1300;
    recs.push(
      rec({
        Nom: `Nom${i}`,
        Prénom: `Pre${i}`,
        "Nom Resp": `Fam${fam}`,
        "Prénom Resp": "Mere",
        "Email Personnel Resp": `m${fam}@example.org`,
        "Nom Conjoint": `Fam${fam}`,
        "Prénom Conjoint": "Pere",
        "Email Personnel Conjoint": `p${fam}@example.org`,
      }),
    );
    eleves.push(eleve(`e${i}`, `Nom${i}`, `Pre${i}`));
  }
  const t0 = Date.now();
  const plan = buildPlan(analyzeFile(rowsFromRecords(recs), eleves, TODAY), EMPTY, { keepOldLinks: false });
  assert.equal(plan.counts.foyersCrees, 1300);
  const { db, tables } = fakeDb();
  await applyPlan(db, plan, CTX);
  assert.equal(tables.foyer!.length, 1300);
  assert.equal(tables.eleve_foyer_link!.length, 1800);
  assert.ok(Date.now() - t0 < 10000);
});

test("M5 : relance après lot partiel sans doublon de foyers", async () => {
  const n = 250;
  const recs = Array.from({ length: n }, (_, i) =>
    rec({ Prénom: `P${i}`, "Nom Resp": `R${i}`, "Email Personnel Resp": `r${i}@example.org`, ...NOCONJ }),
  );
  const eleves = recs.map((_, i) => eleve(`e${i}`, "Martin", `P${i}`));
  const analysis = () => analyzeFile(rowsFromRecords(recs), eleves, TODAY);
  const f = fakeDb();
  const plan1 = buildPlan(analysis(), EMPTY, { keepOldLinks: false });
  let calls = 0;
  const failing: FoyersDb = {
    transaction: async (fn) => {
      if (++calls === 2) throw new Error("boom");
      return f.db.transaction(fn);
    },
  };
  await assert.rejects(
    () => applyPlan(failing, plan1, CTX),
    (e: unknown) => e instanceof ApplyPartialError && /refaites l'aperçu/.test(e.message),
  );
  assert.equal(f.tables.foyer!.length, 100);
  const plan2 = buildPlan(analysis(), existingFrom(f.tables), { keepOldLinks: false });
  assert.equal(plan2.counts.foyersCrees, 150);
  await applyPlan(f.db, plan2, CTX);
  assert.equal(f.tables.foyer!.length, n);
  assert.equal(f.tables.eleve_foyer_link!.length, n);
});

test("M3 : verrou par établissement et audit début / échec dans l'application", () => {
  const svc = readFileSync(join(process.cwd(), "app/lib/foyers-import/service.ts"), "utf8");
  assert.ok(svc.includes('withNamedLock("foyers-import"'));
  assert.ok(svc.includes("ImportBusyError"));
  assert.ok(svc.indexOf('"import_start"') < svc.indexOf("applyPlan("));
  assert.ok(svc.includes('"import_failed"'));
});

test("M6 : seuls les .xlsx valides sont acceptés", () => {
  assert.throws(() => assertSafeXlsx(Buffer.from("pas un zip du tout, vraiment pas")), FoyersFileError);
  const ole = Buffer.concat([Buffer.from([0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]), Buffer.alloc(100)]);
  assert.throws(() => assertSafeXlsx(ole), FoyersFileError);
  const ws = XLSX.utils.json_to_sheet([rec({})]);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "F");
  assert.doesNotThrow(() => assertSafeXlsx(XLSX.write(wb, { type: "buffer", bookType: "xlsx" }) as Buffer));
  const route = readFileSync(join(process.cwd(), "app/api/eleves/foyers/import/route.ts"), "utf8");
  assert.ok(!route.includes('".xls"'));
});

/* ───────── Modification d'un responsable ───────── */

test("modification responsable : validation", () => {
  const ok = validateResponsableInput({
    nom: " Dupont ",
    prenom: "Anne",
    email: " ANNE@Example.ORG ",
    telephone: "+33 6 11 22 33 44",
    autoriteParentale: true,
  });
  assert.ok(ok.ok);
  if (ok.ok) {
    assert.equal(ok.value.nom, "Dupont");
    assert.equal(ok.value.email, "anne@example.org");
    assert.equal(ok.value.telephone, "0611223344");
    assert.equal(ok.value.autoriteParentale, true);
  }
  const vide = validateResponsableInput({ nom: "A", prenom: "B", email: "", telephone: "" });
  assert.ok(vide.ok && vide.value.email === null && vide.value.telephone === null);
  for (const bad of [
    { nom: "", prenom: "B" },
    { nom: "A", prenom: " " },
    { nom: "A", prenom: "B", email: "pas-un-mail" },
    { nom: "A", prenom: "B", telephone: "12" },
    { nom: "A", prenom: "B", autoriteParentale: "oui" },
    null,
  ]) {
    assert.equal(validateResponsableInput(bad).ok, false, JSON.stringify(bad));
  }
});

test("modification responsable : garde, établissement, userId et eleve intacts", () => {
  const route = readFileSync(join(process.cwd(), "app/api/eleves/foyers/responsables/[id]/route.ts"), "utf8");
  assert.ok(route.includes("requireFoyersGate()") && route.includes("gate.etablissementId"));
  const edit = readFileSync(join(process.cwd(), "app/lib/foyers-import/responsable-edit.ts"), "utf8");
  const setBlock = edit.slice(edit.indexOf(".set("), edit.indexOf(".set(") + 120);
  assert.ok(!/userId/.test(setBlock));
  assert.ok(!/import\s*\{[^}]*\beleve\b[^}]*\}\s*from "@\/db\/schema"/.test(edit));
  assert.ok(edit.includes("eq(foyerResponsable.etablissementId"));
  assert.ok(edit.includes("avant") && edit.includes("apres"));
  assert.equal(canManageFoyersRoles(["professeur"], {}), false);
  assert.equal(canManageFoyersRoles(["administratif"], {}), true);
});

/* ───────── Analyse d'impact (partie 3) ───────── */

test("adresses à vérifier : générique, personnel, autre foyer — non écrites par défaut", () => {
  const a = analyze(
    [
      rec({
        "Email Personnel Resp": "secretariat@example.org",
        "Email Personnel Conjoint": "prof@example.org",
      }),
    ],
    [eleve("e1", "Martin", "Léa")],
  );
  const existing: ExistingData = { ...EMPTY, staffEmails: ["prof@example.org"] };
  const plan = buildPlan(a, existing, { keepOldLinks: false });
  const rs = plan.foyers[0]!.responsables;
  assert.equal(rs[0]!.email, null);
  assert.equal(rs[1]!.email, null);
  assert.equal(plan.anomalies.filter((x) => x.kind === "adresse_a_verifier").length, 2);
  // option explicite : écrites
  const forced = buildPlan(a, existing, { keepOldLinks: false, writeSuspectEmails: true });
  assert.equal(forced.foyers[0]!.responsables[0]!.email, "secretariat@example.org");
  assert.equal(forced.anomalies.filter((x) => x.kind === "adresse_a_verifier").length, 0);
  assert.notEqual(
    planFingerprint(plan, "h", { keepOldLinks: false }),
    planFingerprint(forced, "h", { keepOldLinks: false, writeSuspectEmails: true }),
  );
  // adresse déjà sur un AUTRE foyer de la base
  const other = buildPlan(
    analyze([rec({})], [eleve("e1", "Martin", "Léa")]),
    { ...EMPTY, emailFoyers: { "claire@example.org": ["fAutre"] } },
    { keepOldLinks: false },
  );
  assert.equal(other.foyers[0]!.responsables[0]!.email, null);
  assert.ok(other.anomalies.some((x) => x.kind === "adresse_a_verifier"));
  // …mais pas si c'est le propre foyer rattaché
  const own = buildPlan(
    analyze([rec({})], [eleve("e1", "Martin", "Léa")]),
    {
      foyers: [{ id: "f1", label: "F" }],
      responsables: [resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", email: "claire@example.org" })],
      links: [{ eleveId: "e1", foyerId: "f1" }],
      emailFoyers: { "claire@example.org": ["f1"] },
    },
    { keepOldLinks: false },
  );
  assert.ok(!own.anomalies.some((x) => x.kind === "adresse_a_verifier" && x.eleve.includes("Claire")));
});

test("adresses génériques reconnues", () => {
  for (const e of ["contact@x.fr", "Secretariat@x.fr", "info@x.fr", "ecole@x.fr", "no-reply@x.fr", "noreply@x.fr", "contact2@x.fr"]) {
    assert.equal(isGenericEmail(e), true, e);
  }
  assert.equal(isGenericEmail("claire.martin@x.fr"), false);
});

test("liens protégés : un e-mail de compte existant (sans userId) compte comme compte famille", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const plan = buildPlan(
    a,
    {
      foyers: [
        { id: "f1", label: "OK" },
        { id: "fOld", label: "Ancien" },
      ],
      responsables: [
        resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", email: "claire@example.org" }),
        resp({ id: "rOld", foyerId: "fOld", nom: "X", prenom: "Y", email: "Parent@Example.org" }),
      ],
      links: [
        { eleveId: "e1", foyerId: "f1" },
        { eleveId: "e1", foyerId: "fOld" },
      ],
      accountEmails: ["parent@example.org"],
    },
    { keepOldLinks: false },
  );
  assert.equal(plan.linksRemove.length, 0);
  assert.deepEqual(plan.liensAVerifier[0]!.raisons, ["compte famille"]);
});

test("autorité parentale : passage de « aucun coché » à « certains cochés » signalé", () => {
  const a = analyze([rec({})], [eleve("e1", "Martin", "Léa")]);
  const existing = (autorite: boolean): ExistingData => ({
    foyers: [{ id: "f1", label: "F" }],
    responsables: [
      resp({ id: "r1", foyerId: "f1", nom: "Martin", prenom: "Claire", email: "claire@example.org", autoriteParentale: autorite }),
    ],
    links: [{ eleveId: "e1", foyerId: "f1" }],
  });
  const p1 = buildPlan(a, existing(false), { keepOldLinks: false });
  assert.equal(p1.foyers[0]!.autoriteChange, true);
  assert.ok(p1.anomalies.some((x) => x.kind === "autorite_change"));
  const p2 = buildPlan(a, existing(true), { keepOldLinks: false });
  assert.equal(p2.foyers[0]!.autoriteChange, false);
});

test("cantine : foyer avec facturation, sinon lien le plus ancien, ordre stable", () => {
  const links = [
    { eleveId: "e1", foyerId: "fB", createdAt: new Date("2025-02-01") },
    { eleveId: "e1", foyerId: "fA", createdAt: new Date("2025-03-01") },
    { eleveId: "e2", foyerId: "fZ", createdAt: new Date("2025-01-01") },
    { eleveId: "e2", foyerId: "fY", createdAt: new Date("2025-01-01") },
  ];
  const none = pickFoyerByEleve(links, new Set());
  assert.equal(none.get("e1"), "fB");
  assert.equal(none.get("e2"), "fY");
  assert.equal(pickFoyerByEleve(links, new Set(["fA"])).get("e1"), "fA");
  assert.equal(pickFoyerByEleve([...links].reverse(), new Set()).get("e2"), "fY");
  const src = readFileSync(join(process.cwd(), "app/lib/passages-facturation-db.ts"), "utf8");
  assert.ok(src.includes("pickFoyerByEleve") && src.includes("foyerFacturation"));
});

test("stages : une adresse par foyer avant de tronquer, e-mail élève conservé", () => {
  const out = pickIdentityRecipients(
    ["fiche1@example.org", "a1@example.org", "a2@example.org", "a3@example.org", "b1@example.org", "c1@example.org"],
    ["a1@example.org", "b1@example.org", "c1@example.org"],
    ["eleve@example.org"],
    4,
  );
  assert.equal(out.length, 4);
  for (const e of ["a1@example.org", "b1@example.org", "c1@example.org", "eleve@example.org"]) {
    assert.ok(out.includes(e), e);
  }
  assert.deepEqual(
    pickIdentityRecipients(["p@example.org"], ["p@example.org"], [], 4),
    ["p@example.org"],
  );
  const src = readFileSync(join(process.cwd(), "app/lib/stage-identity-recipients.ts"), "utf8");
  assert.ok(src.includes("pickIdentityRecipients") && src.includes("createdAt"));
});

test("messagerie familles : plus de plafond de 300 lignes, liste paginée côté serveur", () => {
  const db = readFileSync(join(process.cwd(), "app/lib/famille-messaging-db.ts"), "utf8");
  assert.ok(!/\.limit\(300\)/.test(db.slice(db.indexOf("listFoyersLight"), db.indexOf("getFamilleMessagingSettings"))));
  const foyers = Array.from({ length: 1300 }, (_, i) => ({
    id: `f${i}`,
    label: `Foyer ${String(i).padStart(4, "0")}`,
    eleves: [{ nom: i === 1250 ? "Éloïse" : `N${i}`, prenom: "P", classe: "6A" }],
  }));
  const p1 = searchPaginateFoyers(foyers, { limit: 100, offset: 0 });
  assert.equal(p1.items.length, 100);
  assert.equal(p1.total, 1300);
  assert.equal(searchPaginateFoyers(foyers, { q: "eloise" }).items[0]!.id, "f1250");
  assert.equal(searchPaginateFoyers(foyers, { limit: 500, offset: 1250 }).items.length, 50);
  // les droits d'un prof et la diffusion portent sur la liste complète
  assert.equal(filterFoyersForProfClasses(foyers, ["6A"]).length, 1300);
  const route = readFileSync(join(process.cwd(), "app/api/vie-scolaire/messages-foyer/route.ts"), "utf8");
  assert.ok(route.includes("searchPaginateFoyers"));
});

/* ───────── Cloisonnement des familles recomposées (partie 4) ───────── */
/*
 * Foyer C = Père (X, Y) ; foyer B = Mère + Beau-père (X, Y, Z).
 * Le Père ne voit jamais Z ; la Mère voit X, Y, Z ; aucun accès de proche en proche.
 */

const P = { Nom: "Durand", Prénom: "Paul", mail: "paul@example.org" };
const REC_NOCONJ = {
  "Nom Conjoint": "",
  "Prénom Conjoint": "",
  "Email Personnel Conjoint": "",
  "Tél. portable Conjoint": "",
};
function recompose() {
  const rowC = (prenom: string) =>
    rec({
      Nom: "Durand",
      Prénom: prenom,
      "Lien de parenté": "Père",
      "Nom Resp": P.Nom,
      "Prénom Resp": P.Prénom,
      "Email Personnel Resp": P.mail,
      "Tél. portable Resp": "0600000001",
      ...REC_NOCONJ,
    });
  const rowB = (nom: string, prenom: string) =>
    rec({
      Nom: nom,
      Prénom: prenom,
      "Lien de parenté": "Mère",
      "Lien de parenté avec le conjoint": "Beau-père",
      "Nom Resp": "Martin",
      "Prénom Resp": "Claire",
      "Email Personnel Resp": "claire@example.org",
      "Tél. portable Resp": "0600000002",
      "Nom Conjoint": "Leroy",
      "Prénom Conjoint": "Marc",
      "Email Personnel Conjoint": "marc@example.org",
      "Tél. portable Conjoint": "0600000003",
    });
  const recs = [rowC("X"), rowB("Durand", "X"), rowC("Y"), rowB("Durand", "Y"), rowB("Leroy", "Z")];
  const eleves = [eleve("eX", "Durand", "X"), eleve("eY", "Durand", "Y"), eleve("eZ", "Leroy", "Z")];
  return { recs, eleves };
}

function linksOf(plan: ReturnType<typeof buildPlan>, eleveId: string): string[] {
  return plan.linksAdd
    .filter((l) => l.eleveId === eleveId)
    .map((l) => l.foyerRef)
    .sort();
}

test("recomposée : plan d'import — X, Y liés à B et C, Z seulement à B, Père absent de B", () => {
  const { recs, eleves } = recompose();
  const plan = buildPlan(analyze(recs, eleves), EMPTY, { keepOldLinks: false });
  assert.equal(plan.counts.foyersCrees, 2);
  const C = plan.foyers.find((f) => f.responsables.some((r) => r.prenom === "Paul"))!;
  const B = plan.foyers.find((f) => f.responsables.some((r) => r.prenom === "Claire"))!;
  assert.notEqual(B.ref, C.ref);
  assert.deepEqual(C.responsables.map((r) => r.prenom), ["Paul"]);
  assert.deepEqual(B.responsables.map((r) => r.prenom).sort(), ["Claire", "Marc"]);
  assert.deepEqual(linksOf(plan, "eX"), [B.ref, C.ref].sort());
  assert.deepEqual(linksOf(plan, "eY"), [B.ref, C.ref].sort());
  assert.deepEqual(linksOf(plan, "eZ"), [B.ref]);
  // le Père n'est dans aucun foyer de Z
  assert.ok(!B.responsables.some((r) => r.email === P.mail));
});

test("recomposée : ancien foyer commun Père + Mère en base — jamais fusionné, jamais lié à Z, signalé", () => {
  const { recs, eleves } = recompose();
  const old = (protege: boolean): ExistingData => ({
    foyers: [{ id: "fOld", label: "Foyer Durand", facturation: protege }],
    responsables: [
      resp({ id: "rP", foyerId: "fOld", nom: "Durand", prenom: "Paul", email: P.mail, rang: 1 }),
      resp({ id: "rM", foyerId: "fOld", nom: "Martin", prenom: "Claire", email: "claire@example.org", rang: 2 }),
    ],
    links: [
      { eleveId: "eX", foyerId: "fOld" },
      { eleveId: "eY", foyerId: "fOld" },
    ],
  });
  // ancien foyer protégé (facturation) : conservé, signalé
  const plan = buildPlan(analyze(recs, eleves), old(true), { keepOldLinks: false });
  assert.ok(plan.foyers.every((f) => f.existingId !== "fOld"), "ancien foyer non réutilisé");
  assert.equal(plan.counts.foyersCrees, 2);
  assert.ok(plan.linksAdd.every((l) => l.foyerRef !== "fOld"));
  assert.ok(!plan.linksAdd.some((l) => l.eleveId === "eZ" && l.foyerRef === "fOld"));
  assert.ok(plan.anomalies.some((x) => x.kind === "foyer_commun_ancien"));
  const aVerifier = plan.liensAVerifier.filter((l) => l.foyerId === "fOld");
  assert.equal(aVerifier.length, 2);
  assert.ok(aVerifier.every((l) => l.raisons.includes("ancien foyer commun conservé")));
  assert.equal(plan.linksRemove.length, 0);
  // non protégé : liens retirés, jamais de lien vers Z
  const p2 = buildPlan(analyze(recs, eleves), old(false), { keepOldLinks: false });
  assert.equal(p2.linksRemove.filter((l) => l.foyerId === "fOld").length, 2);
  assert.ok(p2.linksAdd.every((l) => l.foyerRef !== "fOld"));
});

test("recomposée : une seconde application ne fusionne rien (idempotence du cloisonnement)", async () => {
  const { recs, eleves } = recompose();
  const f = fakeDb();
  const plan = buildPlan(analyze(recs, eleves), EMPTY, { keepOldLinks: false });
  await applyPlan(f.db, plan, CTX);
  const plan2 = buildPlan(analyze(recs, eleves), existingFrom(f.tables), { keepOldLinks: false });
  assert.equal(plan2.counts.foyersCrees, 0);
  assert.ok(planIsNoop(plan2));
  // Qui voit quoi, d'après les liens écrits : modèle « foyer dont l'adulte est responsable ».
  const seen = (email: string) =>
    new Set(
      f.tables
        .eleve_foyer_link!.filter((l) =>
          f.tables.foyer_responsable!.some((r) => r.foyerId === l.foyerId && r.email === email),
        )
        .map((l) => l.eleveId),
    );
  assert.deepEqual([...seen(P.mail)].sort(), ["eX", "eY"]);
  assert.deepEqual([...seen("claire@example.org")].sort(), ["eX", "eY", "eZ"]);
  assert.deepEqual([...seen("marc@example.org")].sort(), ["eX", "eY", "eZ"]);
});

test("famille-auth : accès par foyer du responsable lui-même, par fiche élève de sa propre adresse, aucun second saut", () => {
  const src = readFileSync(join(process.cwd(), "app/lib/famille-auth.ts"), "utf8");
  const viaFoyer = src.slice(src.indexOf("const viaFoyer"), src.indexOf("for (const row of viaFoyer)"));
  // le responsable désigné est celui de l'utilisateur (userId ou son e-mail), sur le même foyer que le lien
  assert.ok(viaFoyer.includes("eq(foyerResponsable.foyerId, eleveFoyerLink.foyerId)") || viaFoyer.includes("eq(foyerResponsable.foyerId, eleveFoyerLink.foyerId"));
  assert.ok(viaFoyer.includes("eq(foyerResponsable.userId, authUserId)"));
  assert.ok(viaFoyer.includes("= ${emailNorm}"));
  const viaEmail = src.slice(src.indexOf("const viaEmail"), src.indexOf("for (const row of viaEmail)"));
  // seule l'adresse de l'utilisateur est comparée aux e-mails parents de la fiche : pas d'expansion par frère
  assert.equal((viaEmail.match(/= \$\{emailNorm\}/g) ?? []).length, 3);
  assert.ok(!/eleveFoyerLink|foyerResponsable/.test(viaEmail));
});

function cfg(id: string, nom: string, prenom: string, parentMails: string[]) {
  return {
    id,
    nom,
    prenom,
    classe: "3A",
    status: "inscrit",
    dateNaissance: "2012-01-01",
    parent1Email: parentMails[0] ?? "",
    parentEmail: parentMails[1] ?? "",
    parent2Email: parentMails[2] ?? "",
  } as never;
}

test("fratrie de stage : le Père (adresse vérifiée) ne reçoit que X et Y, jamais Z", () => {
  const eleves = [
    // la fiche de X porte l'adresse de la Mère : ne doit rien ouvrir au Père
    cfg("eX", "Durand", "X", ["claire@example.org", P.mail]),
    cfg("eY", "Durand", "Y", ["claire@example.org", P.mail]),
    cfg("eZ", "Leroy", "Z", ["claire@example.org", "marc@example.org"]),
  ];
  const father = filterStageSiblings({
    eleves,
    verifiedEmails: [P.mail],
    foyerEleveIds: ["eX", "eY"], // enfants des foyers où le Père est responsable
    isClassEligible: () => true,
  });
  assert.deepEqual(father.map((s) => s.id).sort(), ["eX", "eY"]);
  const mother = filterStageSiblings({
    eleves,
    verifiedEmails: ["claire@example.org"],
    foyerEleveIds: ["eX", "eY", "eZ"],
    isClassEligible: () => true,
  });
  assert.deepEqual(mother.map((s) => s.id).sort(), ["eX", "eY", "eZ"]);
  // sans lien de foyer : la fiche seule suffit pour l'adresse vérifiée, jamais pour une autre adresse
  const onlyFiche = filterStageSiblings({ eleves, verifiedEmails: [P.mail], isClassEligible: () => true });
  assert.deepEqual(onlyFiche.map((s) => s.id).sort(), ["eX", "eY"]);
  // la requête de foyers part des responsables portant l'adresse vérifiée, puis de leurs liens (un seul saut)
  const src = readFileSync(join(process.cwd(), "app/lib/stage-siblings.ts"), "utf8");
  assert.ok(src.includes("lower(trim(${foyerResponsable.email}))") && src.includes("inArray(eleveFoyerLink.foyerId, foyerIds)"));
  // plafond conservé
  assert.ok(src.includes("STAGE_SIBLINGS_MAX = 6"));
});

test("stage : destinataires d'identité de Z = foyer de Z seulement (jamais le Père)", () => {
  const forZ = pickIdentityRecipients(
    ["claire@example.org", "marc@example.org"],
    ["claire@example.org"],
    ["z@example.org"],
    4,
  );
  assert.ok(!forZ.includes(P.mail));
  const forX = pickIdentityRecipients(
    ["claire@example.org", P.mail, "marc@example.org"],
    [P.mail, "claire@example.org"],
    [],
    4,
  );
  assert.ok(forX.includes(P.mail) && forX.includes("claire@example.org"));
  const src = readFileSync(join(process.cwd(), "app/lib/stage-identity-recipients.ts"), "utf8");
  // les foyers lus sont ceux de l'élève concerné uniquement
  assert.ok(src.includes("eq(eleveFoyerLink.eleveId, id)") && src.includes("inArray(foyerResponsable.foyerId, foyerIds)"));
});

test("RDV inscription : l'adresse du Père ne rattache que les enfants de ses foyers", () => {
  const eleves = [
    cfg("eX", "Durand", "X", ["claire@example.org", P.mail]),
    cfg("eZ", "Leroy", "Z", ["claire@example.org", "marc@example.org"]),
  ];
  assert.deepEqual(elevesMatchingParentContact(eleves, P.mail).map((e) => e.id), ["eX"]);
  assert.deepEqual(
    elevesMatchingParentContact(eleves, "claire@example.org").map((e) => e.id).sort(),
    ["eX", "eZ"],
  );
  const src = readFileSync(join(process.cwd(), "app/lib/rdv-inscription-eleve.ts"), "utf8");
  const fn = src.slice(src.indexOf("function listEleveIdsLinkedToFoyerEmail"), src.indexOf("function listEleveIdsLinkedToFoyerEmail") + 1200);
  assert.ok(fn.includes("= ${email}") && fn.includes("eleveFoyerLink.foyerId, foyerResponsable.foyerId"));
});

test("dossier : les enfants affichés pour un foyer sont ceux liés à CE foyer", () => {
  const src = readFileSync(join(process.cwd(), "app/api/eleves/[id]/dossier/route.ts"), "utf8");
  const blk = src.slice(src.indexOf("enfantsByFoyer = new Map"), src.indexOf("const respByFoyer"));
  assert.ok(blk.includes("inArray(eleveFoyerLink.foyerId, batch)"));
  assert.ok(blk.includes("enfantsByFoyer.get(k.foyerId)") && blk.includes("set(k.foyerId"));
  const svc = readFileSync(join(process.cwd(), "app/lib/foyers-import/service.ts"), "utf8");
  assert.ok(svc.includes("kidBy.get(f.id)") && svc.includes("kidBy.set(k.foyerId"));
});
