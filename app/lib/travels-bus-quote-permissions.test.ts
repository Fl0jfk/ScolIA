import assert from "node:assert/strict";
import {
  canChooseBusQuoteForLogisticsTransition,
  canSelectBusQuoteWithoutSignature,
} from "@/app/lib/travels-bus-quote-permissions";

const denied = {
  isOwner: false,
  canSign: false,
  isAdministratif: false,
  isCompta: false,
  isGlobalAdmin: false,
};

assert.equal(canChooseBusQuoteForLogisticsTransition(denied), false);
assert.equal(canSelectBusQuoteWithoutSignature(denied), false);

assert.equal(
  canChooseBusQuoteForLogisticsTransition({ ...denied, isOwner: true }),
  true,
  "créateur peut choisir",
);
assert.equal(
  canSelectBusQuoteWithoutSignature({ ...denied, isOwner: true }),
  true,
  "créateur sans signature",
);

assert.equal(
  canChooseBusQuoteForLogisticsTransition({ ...denied, isAdministratif: true }),
  true,
);
assert.equal(
  canChooseBusQuoteForLogisticsTransition({ ...denied, isCompta: true }),
  true,
);
assert.equal(
  canChooseBusQuoteForLogisticsTransition({ ...denied, isGlobalAdmin: true }),
  true,
);
assert.equal(
  canChooseBusQuoteForLogisticsTransition({ ...denied, canSign: true }),
  true,
  "direction peut choisir / signer",
);
assert.equal(
  canSelectBusQuoteWithoutSignature({ ...denied, canSign: true }),
  false,
  "direction utilise choisir-et-signer, pas choisir seul",
);

console.log("travels-bus-quote-permissions.test.ts: ok");
