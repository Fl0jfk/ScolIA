import assert from "node:assert/strict";
import { test } from "node:test";
import { classifyScoliaMode, instructionPart, quickRoute } from "@/app/lib/brain-ai/intent-router";

test("répondre / reformuler un message collé → hors métier", () => {
  const cases = [
    "Tu peux répondre à ce message de façon cordiale : Bonjour, je souhaite faire une demande de rendez-vous pour mon fils",
    "Réponds-lui sèchement que ce n'est pas possible",
    "Voici le message d'un parent : « Mon fils sera absent demain, pouvez-vous m'envoyer les devoirs ? ». Réponds en disant qu'il faut un justificatif",
    "Réécris ce message de façon claire et professionnelle : Bonjour, la réunion est décalée",
    "Corrige l'orthographe : je vous informe que les bulletin sont disponible",
    "Écris un mot pour remercier les parents d'élèves bénévoles",
    "Bonjour madame,\nJe n'ai pas reçu la convention de stage de ma fille.\nCordialement\n\nréponds de manière polie",
    "Réécris ta dernière réponse en plus court, sans perdre l’essentiel.",
  ];
  for (const m of cases) assert.equal(quickRoute(m), "general", m);
});

test("actions et données de l'ENT → métier", () => {
  const cases = [
    "Envoie ce message à Mme Dupont",
    "Réécris ce message et envoie-le à M. Martin",
    "Je veux réserver une salle demain",
    "Quel est mon planning cette semaine ?",
    "Qu'y a-t-il dans le cahier de textes de la 4B ?",
    "Ouvre le dossier de Léa Martin",
    "Déclare une absence pour moi vendredi",
    "Qu'est-ce que j'ai à traiter aujourd'hui ?",
    "Bonjour",
  ];
  for (const m of cases) assert.equal(quickRoute(m), "metier", m);
});

test("fichiers joints → toujours métier", () => {
  assert.equal(quickRoute("Réponds à ce mail de façon cordiale", { hasFiles: true }), "metier");
});

test("cas ambigus → classifieur", () => {
  assert.equal(quickRoute("Explique-moi la pédagogie Montessori en quelques lignes"), null);
  // Notion propre à l'école (PAP…) : métier, la base de connaissances peut répondre.
  assert.equal(quickRoute("C'est quoi la différence entre un PAP et un PPS ?"), "metier");
  assert.equal(quickRoute("Donne-moi des idées d'activités pour la journée du sport"), null);
  // Rédiger à partir de données de l'ENT : le classifieur tranche.
  assert.equal(quickRoute("Résume le dossier de Léa Martin"), null);
});

test("la consigne est séparée du texte collé", () => {
  const m =
    "Réponds poliment : « Pouvez-vous m'envoyer le formulaire et réserver une place à la cantine pour mon fils ? »";
  const instr = instructionPart(m);
  assert.ok(!/envoyer|réserver/.test(instr), instr);
  assert.equal(quickRoute(m), "general");
});

function fakeFetch(content: string, ok = true): typeof fetch {
  return (async () =>
    ({
      ok,
      json: async () => ({ choices: [{ message: { content } }] }),
    }) as Response) as unknown as typeof fetch;
}

test("classifieur : réponse lue, erreur → métier", async () => {
  assert.equal(await classifyScoliaMode("idées", [], "k", fakeFetch('{"mode":"general"}')), "general");
  assert.equal(await classifyScoliaMode("x", [], "k", fakeFetch('{"mode":"metier"}')), "metier");
  assert.equal(await classifyScoliaMode("x", [], "k", fakeFetch("pas du json")), "metier");
  assert.equal(await classifyScoliaMode("x", [], "k", fakeFetch("{}", false)), "metier");
  const boom = (async () => {
    throw new Error("réseau");
  }) as unknown as typeof fetch;
  assert.equal(await classifyScoliaMode("x", [], "k", boom), "metier");
});
