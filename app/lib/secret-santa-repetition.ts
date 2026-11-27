/**
 * Répétition : un faux tirage pour vérifier les incompatibilités. Mêmes règles et même fonction que
 * le vrai, mais un hasard indépendant ; rien n'est lu d'un tirage existant ni enregistré.
 */

import {
  SANTA_MAX_PARTICIPANTS,
  SANTA_MIN_PARTICIPANTS,
  SantaError,
  drawBlockerWithRules,
  drawSecretSanta,
  exclusionsAmong,
  santaMaxExclusionsPerPerson,
  type RandomInt,
  type SantaExclusion,
  type SantaPair,
} from "@/app/lib/secret-santa";

export const REHEARSAL_CHECKS = 1000;
const TIME_BUDGET_MS = 1500;

export type SantaRehearsal =
  | {
      ok: true;
      /** Le faux tirage affiché. */
      pairs: SantaPair[];
      /** Règles qui concernent les personnes choisies. */
      rules: SantaExclusion[];
      /** Faux tirages vérifiés en plus, sans les afficher. */
      checked: number;
      violations: number;
    }
  | {
      ok: false;
      reason: "not_enough" | "blocked";
      maxPerPerson: number;
      /** Personnes qui ont trop de personnes à éviter pour un groupe de cette taille. */
      overLimit: { userId: string; count: number }[];
    };

export function breaksRules(pairs: SantaPair[], rules: SantaExclusion[]): boolean {
  return pairs.some(({ giver, receiver }) =>
    rules.some(([a, b]) => (a === giver && b === receiver) || (b === giver && a === receiver)),
  );
}

export function rehearseDraw(
  participantIds: string[],
  confidential: SantaExclusion[],
  randomInt: RandomInt,
  clock: () => number = Date.now,
): SantaRehearsal {
  const ids = [...new Set(participantIds)];
  if (ids.length > SANTA_MAX_PARTICIPANTS) throw new SantaError(`${SANTA_MAX_PARTICIPANTS} personnes au maximum.`);
  const rules = exclusionsAmong(ids, confidential);
  const maxPerPerson = Math.max(0, santaMaxExclusionsPerPerson(ids.length));
  if (drawBlockerWithRules(ids, [], confidential)) {
    return {
      ok: false,
      reason: ids.length < SANTA_MIN_PARTICIPANTS ? "not_enough" : "blocked",
      maxPerPerson,
      overLimit: ids
        .map((userId) => ({ userId, count: rules.filter((r) => r.includes(userId)).length }))
        .filter((p) => p.count > maxPerPerson),
    };
  }
  const pairs = drawSecretSanta(ids, rules, randomInt);
  const start = clock();
  let checked = 0;
  let violations = breaksRules(pairs, rules) ? 1 : 0;
  while (checked < REHEARSAL_CHECKS && clock() - start < TIME_BUDGET_MS) {
    if (breaksRules(drawSecretSanta(ids, rules, randomInt), rules)) violations++;
    checked++;
  }
  return { ok: true, pairs, rules, checked, violations };
}
