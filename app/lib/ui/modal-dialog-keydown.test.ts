import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { handleModalDialogKeyDown } from "./modal-dialog-keydown";

describe("handleModalDialogKeyDown", () => {
  it("ferme sur Échap", () => {
    let closed = false;
    let prevented = false;
    handleModalDialogKeyDown(
      {
        key: "Escape",
        preventDefault: () => {
          prevented = true;
        },
      },
      () => {
        closed = true;
      },
    );
    assert.equal(closed, true);
    assert.equal(prevented, true);
  });

  it("ignore les autres touches", () => {
    let closed = false;
    handleModalDialogKeyDown(
      { key: "Enter", preventDefault: () => {} },
      () => {
        closed = true;
      },
    );
    assert.equal(closed, false);
  });
});
