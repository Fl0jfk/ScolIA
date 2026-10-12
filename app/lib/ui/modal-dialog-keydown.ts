/** Gestion clavier commune des modales (fermeture Échap). */
export function handleModalDialogKeyDown(
  event: { key: string; preventDefault: () => void },
  onClose: () => void,
): void {
  if (event.key === "Escape") {
    event.preventDefault();
    onClose();
  }
}
