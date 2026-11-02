export type SstRegistreView = "accueil" | "fiche" | "suivi";

export function formatSstFicheCreatedSuccess(numero: number | undefined | null): string {
  return `Fiche n°${numero ?? "—"} enregistrée.`;
}

/** Où afficher le message après création : le formulaire bascule vers l’accueil → bandeau page. */
export function sstFicheCreatedSuccessChannel(viewAfterSubmit: SstRegistreView): "page" | "form" {
  return viewAfterSubmit === "fiche" ? "form" : "page";
}
