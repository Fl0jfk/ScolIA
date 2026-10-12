import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  buildTransportReplyRecap,
  composeTransportReplyBody,
  guessQuoteReferenceFromText,
  normalizeQuoteReference,
} from "./travels-transport-reply-context";

describe("travels-transport-reply-context", () => {
  it("normalise et détecte un n° de devis dans un texte", () => {
    assert.equal(normalizeQuoteReference("  #DEV-2026-12 "), "DEV-2026-12");
    assert.equal(
      guessQuoteReferenceFromText("Bonjour, ci-joint devis numéro 1815-TTC pour la sortie"),
      "1815-TTC",
    );
    assert.equal(
      guessQuoteReferenceFromText("Objet: Devis n° HGR-1601 — Caen"),
      "HGR-1601",
    );
  });

  it("construit un récap avec date, adresse, montant et réf devis", () => {
    const recap = buildTransportReplyRecap(
      {
        id: "trip-abc",
        data: {
          title: "Musée Caen",
          destination: "Caen",
          startDate: "2026-05-12",
          startTime: "08:30",
          endTime: "17:00",
          classes: "4B",
          transportRequest: { pickupPoint: "Parking gymnase" },
        },
        receivedDevis: [
          {
            id: "q1",
            providerName: "Hangar",
            extractedPrice: "1 601 €",
            extractedReference: "DEV-7788",
          },
        ],
      },
      "q1",
    );

    assert.equal(recap.tripId, "trip-abc");
    assert.equal(recap.destination, "Caen");
    assert.equal(recap.pickupAddress, "Parking gymnase");
    assert.equal(recap.amountTtc, "1 601 €");
    assert.equal(recap.quoteReference, "DEV-7788");
    assert.ok(recap.textBlock.includes("Réf. dossier ScolIA : trip-abc"));
    assert.ok(recap.textBlock.includes("Montant proposé (TTC) : 1 601 €"));
    assert.ok(recap.subjectSuffix.includes("devis DEV-7788"));

    const body = composeTransportReplyBody(
      "Merci d'affiner : être sur place à 10h45, repartir 15h30.",
      recap,
    );
    assert.ok(body.startsWith("Merci d'affiner"));
    assert.ok(body.includes("Contexte sortie"));
  });

  it("devine la réf depuis un message e-mail du même transporteur", () => {
    const recap = buildTransportReplyRecap({
      id: "t2",
      data: {
        destination: "Rouen",
        transportEmailMessages: [
          {
            fromEmail: "commercial@carperier.fr",
            subject: "Devis n° CAR-1815",
            summary: "Proposition bus",
          },
        ],
      },
      receivedDevis: [
        {
          id: "q2",
          providerName: "Carperier",
          providerEmail: "commercial@carperier.fr",
          extractedPrice: "1 815 €",
        },
      ],
    }, "q2");

    assert.equal(recap.quoteReference, "CAR-1815");
    assert.equal(recap.amountTtc, "1 815 €");
  });
});
