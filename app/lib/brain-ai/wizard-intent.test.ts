import assert from "node:assert/strict";
import { detectWizardStartTool, isWritingHelpRequest } from "@/app/lib/brain-ai/wizard-intent";

assert.equal(detectWizardStartTool("ouvre une sortie scolaire"), "open_trip");
assert.equal(detectWizardStartTool("Ouvre la sortie scolaire"), "open_trip");
assert.equal(detectWizardStartTool("montre les sorties"), "open_trip");
assert.equal(detectWizardStartTool("créer une sortie scolaire"), "create_trip");
assert.equal(detectWizardStartTool("nouvelle sortie"), "create_trip");
assert.equal(detectWizardStartTool("faire une sortie scolaire"), "create_trip");
assert.equal(detectWizardStartTool("comment faire une sortie scolaire"), null);

// Aide à la rédaction : jamais de wizard, même si le texte parle d'absence ou de demande.
assert.equal(
  detectWizardStartTool("réécris ce message : il faut faire une demande d'absence"),
  null,
);
assert.equal(detectWizardStartTool("corrige ce mail pour déclarer mon absence"), null);
assert.equal(isWritingHelpRequest("Reformule ce message aux parents de Léa"), true);
assert.equal(isWritingHelpRequest("corrige l'orthographe : Bonjour madame"), true);
assert.equal(isWritingHelpRequest("Peux-tu résumer :\nLe conseil de classe a décidé…"), true);
// Les vraies actions restent des actions.
assert.equal(isWritingHelpRequest("déclarer une absence demain"), false);
assert.equal(detectWizardStartTool("déclarer une absence demain"), "create_absence");
assert.equal(detectWizardStartTool("je veux réserver une salle"), "create_reservation");

console.log("wizard-intent trip open/create + writing help: ok");
