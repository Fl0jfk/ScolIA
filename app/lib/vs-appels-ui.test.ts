import assert from "node:assert/strict";
import test from "node:test";
import { vsAppelCloseConfirmMessageFr } from "@/app/lib/vs-appels-ui";

test("vsAppelCloseConfirmMessageFr — avec ou sans id", () => {
  assert.match(vsAppelCloseConfirmMessageFr("abc-123"), /abc-123/);
  assert.match(vsAppelCloseConfirmMessageFr("abc-123"), /suivi CPE/);
  assert.equal(
    vsAppelCloseConfirmMessageFr(),
    "Clôturer cet appel et transmettre les absences au suivi CPE ?",
  );
  assert.equal(vsAppelCloseConfirmMessageFr(""), vsAppelCloseConfirmMessageFr(null));
});
