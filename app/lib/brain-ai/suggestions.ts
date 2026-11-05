/**
 * Suggestions cliquables à l'ouverture de ScolIA. Une suggestion n'apparaît que si
 * l'outil qu'elle déclenche est autorisé pour l'utilisateur. Un prompt qui finit
 * par « : » pré-remplit la saisie au lieu d'être envoyé.
 */
export type ScoliaSuggestion = { label: string; prompt: string };

const TOOL_SUGGESTIONS: Array<ScoliaSuggestion & { tool: string }> = [
  { tool: "get_my_pending_actions", label: "Qu’ai-je à faire ?", prompt: "Qu’est-ce que j’ai à traiter aujourd’hui ?" },
  { tool: "send_internal_message", label: "Envoyer un message", prompt: "Envoie un message à :" },
  { tool: "get_mon_planning", label: "Mon planning", prompt: "Quel est mon planning cette semaine ?" },
  { tool: "get_cahier_texte", label: "Cahier de textes", prompt: "Qu’y a-t-il dans le cahier de textes cette semaine ?" },
  { tool: "get_week_sheet_today", label: "Feuille du jour", prompt: "Qu’est-ce qui se passe aujourd’hui dans l’établissement ?" },
  { tool: "open_appel", label: "Faire l’appel", prompt: "Je veux faire l’appel." },
  { tool: "create_reservation", label: "Réserver une salle", prompt: "Je veux réserver une salle." },
  { tool: "create_absence", label: "Déclarer mon absence", prompt: "Je veux déclarer une absence." },
  { tool: "create_photocopie_demand", label: "Demande de photocopies", prompt: "Je veux faire une demande de photocopies." },
  { tool: "decide_rh_absence", label: "Absences à valider", prompt: "Quelles absences du personnel dois-je valider ?" },
  { tool: "list_eleves_filtered", label: "Élèves avec PAP", prompt: "Liste les élèves qui ont un PAP." },
  { tool: "get_stages_overview", label: "Point sur les stages", prompt: "Où en sont les stages ?" },
];

const WRITING_SUGGESTIONS: ScoliaSuggestion[] = [
  { label: "Réécrire un message", prompt: "Réécris ce message de façon claire et professionnelle :" },
  { label: "Corriger l’orthographe", prompt: "Corrige l’orthographe de ce texte :" },
];

const MAX_TOOL_SUGGESTIONS = 4;

export function suggestionsForTools(allowedTools: Set<string>): ScoliaSuggestion[] {
  const fromTools = TOOL_SUGGESTIONS.filter((s) => allowedTools.has(s.tool))
    .slice(0, MAX_TOOL_SUGGESTIONS)
    .map(({ label, prompt }) => ({ label, prompt }));
  return [...fromTools, ...WRITING_SUGGESTIONS];
}
