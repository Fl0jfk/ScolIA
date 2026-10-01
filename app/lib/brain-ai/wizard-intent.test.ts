import assert from "node:assert/strict";
import { detectWizardStartTool } from "@/app/lib/brain-ai/wizard-intent";

assert.equal(detectWizardStartTool("ouvre une sortie scolaire"), "open_trip");
assert.equal(detectWizardStartTool("Ouvre la sortie scolaire"), "open_trip");
assert.equal(detectWizardStartTool("montre les sorties"), "open_trip");
assert.equal(detectWizardStartTool("créer une sortie scolaire"), "create_trip");
assert.equal(detectWizardStartTool("nouvelle sortie"), "create_trip");
assert.equal(detectWizardStartTool("faire une sortie scolaire"), "create_trip");
assert.equal(detectWizardStartTool("comment faire une sortie scolaire"), null);

console.log("wizard-intent trip open/create: ok");
