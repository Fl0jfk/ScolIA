import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import * as XLSX from "xlsx";
import { getTableName } from "drizzle-orm";
import { applyPlan, ApplyPartialError, type FoyersDb, type FoyersWriter } from "./apply";
import {
  autoriteFromLien,
  excelSerialToIso,
  normEmail,
  normName,
  normPhone,
  parseBirthDate,
} from "./normalize";
import { parseFoyersWorkbook, rowsFromRecords } from "./parse";
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
        userId: "u1",
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
  assert.equal(r1.hasUser, true);
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
      resp({ id: "rOld", foyerId: "fOld", nom: "Autre", prenom: "Gens", userId: "u9", payeur: true }),
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
    resp({ id: `r${i}`, foyerId: "f1", nom: `Z${i}`, prenom: `Z${i}`, rang: i, email: i === 1 ? "claire@example.org" : null }),
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
              const id = params(cond)[0];
              const row = tables[name]!.find((r) => r.id === id);
              assert.ok(row, `update ${name} ${String(id)} introuvable`);
              Object.assign(row, set);
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
          const [, eleveId, foyerId] = params(cond);
          tables[name] = tables[name]!.filter((r) => !(r.eleveId === eleveId && r.foyerId === foyerId));
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
  assert.equal(r1.telephone, "0611223344");
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
  for (const f of ["apply.ts", "plan.ts", "service.ts", "parse.ts", "normalize.ts", "report.ts"]) {
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
