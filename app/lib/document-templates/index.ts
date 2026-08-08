export * from "@/app/lib/document-templates/types";
export * from "@/app/lib/document-templates/catalog";
export {
  loadGeneratedIndex,
  saveGeneratedDocument,
  loadGeneratedDocument,
  loadGeneratedPdfBytes,
  loadGeneratedFileBytes,
  generatedPdfKey,
  generatedFileKey,
  generatedMetaKey,
  contentTypeForFormat,
  extensionForFormat,
} from "@/app/lib/document-templates/storage";
export { renderDocumentTemplatePdf } from "@/app/lib/document-templates/render-pdf";
export { renderDocumentTemplateDocx } from "@/app/lib/document-templates/render-docx";
export { renderDocumentTemplateFillablePdf } from "@/app/lib/document-templates/render-fillable-pdf";
export {
  valuesFromEleve,
  mergeTemplateValues,
  documentTitle,
} from "@/app/lib/document-templates/fill";
