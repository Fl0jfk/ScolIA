export async function register() {
  // Job serveur des nouveautés de l'ENT (Web Push) : runtime Node uniquement, jamais au build ni en test.
  if (process.env.NEXT_RUNTIME !== "nodejs") return;
  if (process.env.NEXT_PHASE === "phase-production-build") return;
  if (process.env.SCOLIA_TEST_MODE === "1") return;
  try {
    const { startSignalsJob } = await import("./app/lib/web-push/signals-job");
    startSignalsJob();
  } catch (err) {
    console.error("[instrumentation] job signaux non démarré", err instanceof Error ? err.name : "error");
  }
  // Relances automatiques du circuit voyages v2 (J-3 / J-1, pièces avant départ).
  try {
    const { startRemindersJob } = await import("./app/lib/travels-v2-reminders-job");
    startRemindersJob();
  } catch (err) {
    console.error("[instrumentation] job relances voyages non démarré", err instanceof Error ? err.name : "error");
  }
  // Clôture automatique des inscriptions, alertes, purge du lendemain du retour (voyages v2).
  try {
    const { startLifecycleJob } = await import("./app/lib/travels-v2-lifecycle-job");
    startLifecycleJob();
  } catch (err) {
    console.error("[instrumentation] job cycle voyages non démarré", err instanceof Error ? err.name : "error");
  }
  // Autorisations de sortie des internes : relances parents (J-2) et point final de la veille.
  try {
    const { startSortiesClasseJob } = await import("./app/lib/sorties-classe-job");
    startSortiesClasseJob();
  } catch (err) {
    console.error("[instrumentation] job sorties de classe non démarré", err instanceof Error ? err.name : "error");
  }
}
