import assert from "node:assert/strict";
import {
  extractExplicitTripIdHints,
  extractIsoDatesFromText,
  resolveTripMatchDeterministic,
  tripAlreadyHasSignedBusQuote,
} from "@/app/lib/travel-email-match";

const candidates = [
  {
    id: "trip-1783356505943",
    title: "Sortie Somme 1916",
    destination: "Musée Somme 1916 à Albert",
    startDate: "2026-09-25",
    endDate: "2026-09-25",
  },
  {
    id: "trip-1788276259103",
    title: "Plages du Débarquement et Mémorial de Caen",
    destination: "Pointe du Hoc, Mémorial de Caen",
    startDate: "2027-05-18",
    endDate: "2027-05-18",
  },
];

assert.deepEqual(extractIsoDatesFromText("sortie le 18/05/2027"), ["2027-05-18"]);
assert.ok(extractExplicitTripIdHints("ref trip 17882762509103").includes("trip-17882762509103"));

const byRef = resolveTripMatchDeterministic({
  subject: "ref trip 17882762509103",
  ocrText: "18 mai 2027",
  candidates,
});
assert.equal(byRef.tripId, "trip-1788276259103");
assert.equal(byRef.confidence, "high");

const byDate = resolveTripMatchDeterministic({
  subject: "Devis transport",
  ocrText: "Date : 18/05/2027 — Pointe du Hoc — Mémorial de Caen",
  candidates,
});
assert.equal(byDate.tripId, "trip-1788276259103");

assert.equal(
  tripAlreadyHasSignedBusQuote({
    status: "EN_ATTENTE_COMPTA",
    data: { signedQuoteUrl: "https://example.com/x.pdf" },
  }),
  true,
);
assert.equal(
  tripAlreadyHasSignedBusQuote({
    status: "PROF_LOGISTICS",
    data: {},
  }),
  false,
);

console.log("travel-email-match.test.ts OK");
