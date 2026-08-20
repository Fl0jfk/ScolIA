export * from "@/app/lib/document-templates/types";
export * from "@/app/lib/document-templates/catalog";
export {
  INSCRIPTION_LEVELS,
  getInscriptionLevelMeta,
  isInscriptionLevelId,
  inscriptionSourcePath,
} from "@/app/lib/document-templates/inscription-levels";
export {
  loadGeneratedIndex,
  saveGeneratedDocument,
  loadGeneratedDocument,
  loadGeneratedFileBytes,
  generatedFileKey,
  contentTypeForFormat,
  extensionForFormat,
} from "@/app/lib/document-templates/storage";
export {
  defaultInscriptionTenantSettings,
  loadInscriptionTenantSettings,
  saveInscriptionTenantSettings,
  saveInscriptionLevelOverride,
  clearInscriptionLevelOverride,
  loadInscriptionOverrideBytes,
} from "@/app/lib/document-templates/inscription-storage";
export { renderDocumentTemplateDocx } from "@/app/lib/document-templates/render-docx";
export { renderDocumentTemplateFillablePdf } from "@/app/lib/document-templates/render-fillable-pdf";
export { renderInscriptionFillablePdf } from "@/app/lib/document-templates/render-inscription-pdf";
export { renderSixiemeInscriptionPdf } from "@/app/lib/document-templates/render-inscription-sixieme";
export {
  DEFAULT_SIXIEME_OPTIONS,
  defaultSixiemeCodeConfig,
  normalizeSixiemeCodeConfig,
} from "@/app/lib/document-templates/inscription-sixieme-config";
export { documentTitle, inscriptionDocumentTitle } from "@/app/lib/document-templates/fill";
