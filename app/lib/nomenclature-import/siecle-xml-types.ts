export type NomenclatureUpsertRow = {
  type: string;
  code: string;
  /** Cycle Siècle college|lycee — stocké en colonne + metadata.sourceCycle. */
  cycle?: string;
  libelleCourt?: string;
  libelleLong?: string;
  metadataJson?: Record<string, unknown>;
  validFrom?: string;
  validTo?: string;
};
