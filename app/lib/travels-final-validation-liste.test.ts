import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  isCuisineWhoEatsComplete,
  isListeElevesConfirmed,
  isTripListeReadyForFinalisation,
} from "@/app/lib/travels-eleves-list";
import { parentHorairesRequiredForTrip } from "@/app/lib/travels-parent-calendar";
import {
  isTravelsDirectionFinalizedStatus,
  TRAVELS_STATUS_LABELS,
} from "@/app/lib/travels-types";

/** Règle métier : statut après validation finale direction. */
function resolveStatusAfterDirectionFinal(listeReady: boolean): string {
  return listeReady ? "VALIDE" : "FINALISE_DIR_ATTENTE_ELEVES";
}

describe("validation finale → liste élèves", () => {
  it("passe en VALIDE si la liste est déjà confirmée", () => {
    assert.equal(
      resolveStatusAfterDirectionFinal(
        isTripListeReadyForFinalisation({
          listeElevesStatus: "confirmed",
          participantEleves: [{ ine: "1", nom: "A", prenom: "B", droitImageOk: true }],
        }),
      ),
      "VALIDE",
    );
  });

  it("passe en FINALISE_DIR_ATTENTE_ELEVES si liste absente ou brouillon", () => {
    assert.equal(
      resolveStatusAfterDirectionFinal(isTripListeReadyForFinalisation({})),
      "FINALISE_DIR_ATTENTE_ELEVES",
    );
    assert.equal(
      resolveStatusAfterDirectionFinal(
        isTripListeReadyForFinalisation({
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

describe("horaires parents SIMPLE vs COMPLEX", () => {
  it("obligatoires seulement pour COMPLEX", () => {
    assert.equal(parentHorairesRequiredForTrip({ type: "COMPLEX" }), true);
    assert.equal(parentHorairesRequiredForTrip({ type: "SIMPLE" }), false);
  });
});

describe("qui mange si cuisine active", () => {
  it("OK sans cuisine ou 0 repas", () => {
    assert.equal(isCuisineWhoEatsComplete({}), true);
    assert.equal(
      isCuisineWhoEatsComplete({
        piqueNiqueDetails: { active: false } as never,
      }),
      true,
    );
  });

  it("exige paniers = repas commandés", () => {
    const details = {
      active: true,
      daysSelection: { lundi: true },
      orders: { lundi: { picnicTotal: "2" } },
    };
    const data = {
      piqueNiqueDetails: details as never,
      listeElevesStatus: "confirmed" as const,
      participantEleves: [
        { ine: "1", nom: "A", prenom: "B", droitImageOk: true, panierRepas: true },
        { ine: "2", nom: "C", prenom: "D", droitImageOk: true, panierRepas: false },
      ],
    };
    assert.equal(isCuisineWhoEatsComplete(data), false);
    assert.equal(isTripListeReadyForFinalisation(data), false);

    data.participantEleves[1].panierRepas = true;
    assert.equal(isCuisineWhoEatsComplete(data), true);
    assert.equal(isTripListeReadyForFinalisation(data), true);
  });
});
