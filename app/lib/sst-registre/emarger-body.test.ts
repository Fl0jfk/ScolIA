import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  SST_ATTESTATION_ONLY_DATA_URL,
  canSubmitSstEmargement,
  isSstAttestationOnlySignature,
  parseEmargerBody,
} from "./emarger-body";

const DRAWN_PNG =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==AAAAAAAA";

describe("parseEmargerBody", () => {
  it("accepte attestation sans signature dessinée", () => {
    const r = parseEmargerBody({ attestationAcknowledged: true, remarques: "ok" });
    assert.equal(r.ok, true);
    if (!r.ok) return;
    assert.equal(r.data.attestationOnly, true);
    assert.equal(r.data.signatureDataUrl, SST_ATTESTATION_ONLY_DATA_URL);
    assert.equal(r.data.remarques, "ok");
  });

  it("accepte signature PNG valide", () => {
    const r = parseEmargerBody({ signatureDataUrl: DRAWN_PNG });
    assert.equal(r.ok, true);
    if (!r.ok) return;
    assert.equal(r.data.attestationOnly, false);
    assert.equal(r.data.signatureDataUrl, DRAWN_PNG);
  });

  it("refuse ni signature ni attestation", () => {
    const r = parseEmargerBody({});
    assert.equal(r.ok, false);
  });

  it("refuse attestation et signature simultanées", () => {
    const r = parseEmargerBody({
      attestationAcknowledged: true,
      signatureDataUrl: DRAWN_PNG,
    });
    assert.equal(r.ok, false);
  });
});

describe("canSubmitSstEmargement", () => {
  it("active le bouton avec attestation cochée", () => {
    assert.equal(
      canSubmitSstEmargement({
        signing: false,
        signatureDataUrl: null,
        attestationAcknowledged: true,
      }),
      true,
    );
  });

  it("désactive pendant l’envoi", () => {
    assert.equal(
      canSubmitSstEmargement({
        signing: true,
        signatureDataUrl: DRAWN_PNG,
        attestationAcknowledged: true,
      }),
      false,
    );
  });
});

describe("isSstAttestationOnlySignature", () => {
  it("reconnaît le marqueur d’attestation", () => {
    assert.equal(isSstAttestationOnlySignature(SST_ATTESTATION_ONLY_DATA_URL), true);
    assert.equal(isSstAttestationOnlySignature(DRAWN_PNG), false);
  });
});
