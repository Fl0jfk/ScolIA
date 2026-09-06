/**
 * Façade compat : les portes ouvertes sont stockées en Postgres.
 * @see portes-ouvertes-db.ts
 */
export {
  addPortesOuvertesRegistration,
  countRegistrationsBySlot,
  countRegistrationsBySlotAndCycle,
  countRegistrationsForSlot,
  listPortesOuvertesRegistrations,
  remainingPlacesForSlotCycle,
  updatePortesOuvertesRegistration,
  type SlotCycleCounts,
} from "@/app/lib/portes-ouvertes-db";

export type { PortesOuvertesRegistration } from "@/app/lib/portes-ouvertes-types";
