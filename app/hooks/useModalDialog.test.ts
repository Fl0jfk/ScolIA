import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, it } from "node:test";
import { modalDialogLayoutEffectDeps } from "./useModalDialog";

const here = dirname(fileURLToPath(import.meta.url));
const hookSource = readFileSync(join(here, "useModalDialog.ts"), "utf8");

function effectWouldRebind(prevDeps: readonly unknown[], nextDeps: readonly unknown[]): boolean {
  if (prevDeps.length !== nextDeps.length) return true;
  return prevDeps.some((d, i) => !Object.is(d, nextDeps[i]));
}

describe("modalDialogLayoutEffectDeps", () => {
  it("ne dépend que de open", () => {
    assert.deepEqual(modalDialogLayoutEffectDeps(true), [true]);
    assert.deepEqual(modalDialogLayoutEffectDeps(false), [false]);
  });

  it("reste stable quand seul onClose change (régression focus textarea)", () => {
    const open = true;
    const stable = modalDialogLayoutEffectDeps(open);
    const again = modalDialogLayoutEffectDeps(open);
    assert.equal(effectWouldRebind(stable, again), false);
    const closeA = () => {};
    const closeB = () => {};
    const buggyA = [open, closeA] as const;
    const buggyB = [open, closeB] as const;
    assert.equal(
      effectWouldRebind(buggyA, buggyB),
      true,
      "inclure onClose dans les deps relance l’effet et vole le focus",
    );
  });
});

describe("useModalDialog hook source", () => {
  it("stocke onClose dans une ref et n’inclut pas onClose dans les deps de l’effet", () => {
    assert.match(hookSource, /onCloseRef/);
    assert.doesNotMatch(hookSource, /\[open,\s*onClose\]/);
    assert.match(hookSource, /modalDialogLayoutEffectDeps\(open\)/);
  });
});
