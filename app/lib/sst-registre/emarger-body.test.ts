import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  SST_ATTESTATION_ONLY_DATA_URL,
  SST_MARKER_AS_DRAWN_SIGNATURE_ERROR,
  canSubmitSstEmargement,
  isSstAttestationOnlySignature,
  parseEmargerBody,
} from "./emarger-body";
import { SST_DRAWN_SIGNATURE_TEST_DATA_URL } from "./emarger-test-fixtures";

describe("parseEmargerBody", () => {
  it("accepte attestation sans signature dessinée", () => {
    const r = parseEmargerBody({ attestationAcknowledged: true, remarques: "ok" });
    assert.equal(r.ok, true);
    if (!r.ok) return;
    assert.equal(r.data.attestationOnly, true);
    assert.equal(r.data.signatureDataUrl, SST_ATTESTATION_ONLY_DATA_URL);
    assert.equal(r.data.remarques, "ok");
  });

  it("accepte signature PNG valide distincte du marqueur", () => {
    const r = parseEmargerBody({ signatureDataUrl: SST_DRAWN_SIGNATURE_TEST_DATA_URL });
    assert.equal(r.ok, true);
    if (!r.ok) return;
    assert.equal(r.data.attestationOnly, false);
    assert.equal(r.data.signatureDataUrl, SST_DRAWN_SIGNATURE_TEST_DATA_URL);
  });

  it("refuse le marqueur d’attestation en mode signature dessinée", () => {
    const r = parseEmargerBody({ signatureDataUrl: SST_ATTESTATION_ONLY_DATA_URL });
    assert.equal(r.ok, false);
    if (r.ok) return;
    assert.equal(r.error, SST_MARKER_AS_DRAWN_SIGNATURE_ERROR);
  });

  it("refuse ni signature ni attestation", () => {
    const r = parseEmargerBody({});
    assert.equal(r.ok, false);
  });

  it("refuse attestation et signature simultanées", () => {
    const r = parseEmargerBody({
      attestationAcknowledged: true,
      signatureDataUrl: SST_DRAWN_SIGNATURE_TEST_DATA_URL,
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
        signatureDataUrl: SST_DRAWN_SIGNATURE_TEST_DATA_URL,
        attestationAcknowledged: true,
      }),
      false,
    );
  });
});

describe("isSstAttestationOnlySignature", () => {
  it("reconnaît le marqueur d’attestation", () => {
    assert.equal(isSstAttestationOnlySignature(SST_ATTESTATION_ONLY_DATA_URL), true);
    assert.equal(isSstAttestationOnlySignature(SST_DRAWN_SIGNATURE_TEST_DATA_URL), false);
  });
});
