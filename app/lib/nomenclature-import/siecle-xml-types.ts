export type NomenclatureUpsertRow = {
  type: string;
  code: string;
  libelleCourt?: string;
  libelleLong?: string;
  metadataJson?: Record<string, unknown>;
  validFrom?: string;
  validTo?: string;
};
