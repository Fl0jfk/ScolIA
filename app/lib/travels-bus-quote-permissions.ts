export type TravelsBusQuoteSelectionCtx = {
  isOwner: boolean;
  canSign: boolean;
  isAdministratif: boolean;
  isCompta: boolean;
  isGlobalAdmin: boolean;
};

/** Créateur, administratif, compta ou admin général — choix sans signature direction. */
export function canSelectBusQuoteWithoutSignature(ctx: TravelsBusQuoteSelectionCtx): boolean {
  return (
    !ctx.canSign &&
    (ctx.isOwner || ctx.isAdministratif || ctx.isCompta || ctx.isGlobalAdmin)
  );
}

/** Transition PROF_LOGISTICS → EN_ATTENTE_BUS_SIGNATURE (sélection devis bus). */
export function canChooseBusQuoteForLogisticsTransition(ctx: TravelsBusQuoteSelectionCtx): boolean {
  return ctx.canSign || canSelectBusQuoteWithoutSignature(ctx);
}
