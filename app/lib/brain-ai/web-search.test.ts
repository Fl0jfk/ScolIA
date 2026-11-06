import assert from "node:assert/strict";
import { test } from "node:test";
import {
  parseConversationOutput,
  runMistralWebSearch,
  webQueryProblem,
} from "@/app/lib/brain-ai/web-search";

test("requêtes avec données personnelles refusées", () => {
  assert.equal(webQueryProblem("réglementation sorties scolaires 2026"), null);
  assert.match(webQueryProblem("contacter jean.dupont@gmail.com") || "", /e-mail/);
  assert.match(webQueryProblem("appeler 06 12 34 56 78") || "", /téléphone/);
  assert.match(webQueryProblem("situation de [Élève 1] au collège") || "", /personne/);
  assert.match(webQueryProblem("ab") || "", /courte/);
});

test("réponse Conversations : texte et sources extraits", () => {
  const data = {
    outputs: [
      { type: "tool.execution", name: "web_search" },
      {
        type: "message.output",
        content: [
          { type: "text", text: "Les sorties scolaires sont encadrées par " },
          { type: "tool_reference", tool: "web_search", title: "Circulaire", url: "https://www.education.gouv.fr/x" },
          { type: "text", text: "la circulaire de 2023." },
          { type: "tool_reference", tool: "web_search", title: "Doublon", url: "https://www.education.gouv.fr/x" },
          { type: "tool_reference", tool: "web_search", title: "Non https", url: "http://exemple.fr" },
        ],
      },
    ],
  };
  const out = parseConversationOutput(data);
  assert.equal(out.text, "Les sorties scolaires sont encadrées par la circulaire de 2023.");
  assert.deepEqual(out.sources, [{ title: "Circulaire", url: "https://www.education.gouv.fr/x" }]);
  assert.deepEqual(parseConversationOutput(null), { text: "", sources: [] });
});

test("appel : outil web_search, rien conservé chez Mistral, erreurs gérées", async () => {
  let sentBody: Record<string, unknown> = {};
  const okFetch = (async (_url: unknown, init?: RequestInit) => {
    sentBody = JSON.parse(String(init?.body || "{}"));
    return {
      ok: true,
      json: async () => ({ outputs: [{ type: "message.output", content: "Réponse." }] }),
    } as Response;
  }) as unknown as typeof fetch;
  const r = await runMistralWebSearch("calendrier scolaire zone B", "k", okFetch);
  assert.equal(r.ok, true);
  assert.deepEqual(sentBody.tools, [{ type: "web_search" }]);
  assert.equal(sentBody.store, false);

  const ko = (async () => ({ ok: false, status: 500 }) as Response) as unknown as typeof fetch;
  assert.equal((await runMistralWebSearch("calendrier scolaire", "k", ko)).ok, false);
  const boom = (async () => {
    throw new Error("réseau");
  }) as unknown as typeof fetch;
  assert.equal((await runMistralWebSearch("calendrier scolaire", "k", boom)).ok, false);
  // Requête refusée : aucun appel réseau.
  let called = false;
  const spy = (async () => {
    called = true;
    return { ok: true, json: async () => ({}) } as Response;
  }) as unknown as typeof fetch;
  await runMistralWebSearch("écrire à paul@ecole.fr", "k", spy);
  assert.equal(called, false);
});
