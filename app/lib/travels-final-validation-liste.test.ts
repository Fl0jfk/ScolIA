import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { isListeElevesConfirmed } from "@/app/lib/travels-eleves-list";
import {
  isTravelsDirectionFinalizedStatus,
  TRAVELS_STATUS_LABELS,
} from "@/app/lib/travels-types";

/** Règle métier : statut après validation finale direction. */
function resolveStatusAfterDirectionFinal(listeConfirmed: boolean): string {
  return listeConfirmed ? "VALIDE" : "FINALISE_DIR_ATTENTE_ELEVES";
}

describe("validation finale → liste élèves", () => {
  it("passe en VALIDE si la liste est déjà confirmée", () => {
    assert.equal(
      resolveStatusAfterDirectionFinal(
        isListeElevesConfirmed({
          listeElevesStatus: "confirmed",
          participantEleves: [{ ine: "1", nom: "A", prenom: "B", droitImageOk: true }],
        }),
      ),
      "VALIDE",
    );
  });

  it("passe en FINALISE_DIR_ATTENTE_ELEVES si liste absente ou brouillon", () => {
    assert.equal(resolveStatusAfterDirectionFinal(isListeElevesConfirmed({})), "FINALISE_DIR_ATTENTE_ELEVES");
    assert.equal(
      resolveStatusAfterDirectionFinal(
        isListeElevesConfirmed({
          listeElevesStatus: "draft",
          participantEleves: [{ ine: "1", nom: "A", prenom: "B", droitImageOk: true }],
        }),
      ),
      "FINALISE_DIR_ATTENTE_ELEVES",
    );
    assert.equal(
      resolveStatusAfterDirectionFinal(
        isListeElevesConfirmed({
          listeElevesStatus: "confirmed",
          participantEleves: [],
        }),
      ),
      "FINALISE_DIR_ATTENTE_ELEVES",
    );
  });

  it("expose le label et le helper direction finalisée", () => {
    assert.equal(
      TRAVELS_STATUS_LABELS.FINALISE_DIR_ATTENTE_ELEVES,
      "Finalisé direction — liste élèves",
    );
    assert.equal(isTravelsDirectionFinalizedStatus("FINALISE_DIR_ATTENTE_ELEVES"), true);
    assert.equal(isTravelsDirectionFinalizedStatus("VALIDE"), true);
    assert.equal(isTravelsDirectionFinalizedStatus("EN_ATTENTE_DIR_FINAL"), false);
  });
});
