import { test } from "node:test";
import assert from "node:assert/strict";
import { IDENTITY_OTP_MAX_ATTEMPTS } from "@/app/lib/stage-identity-otp";

// confirmIdentityOtpCode dépend de S3 (getJson/putJson) : pas testable sans stockage réel ici.
test("le plafond d'essais OTP est fixé à 5", () => {
  assert.equal(IDENTITY_OTP_MAX_ATTEMPTS, 5);
});
