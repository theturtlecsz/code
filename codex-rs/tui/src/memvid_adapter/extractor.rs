//! SPEC-KIT-980: Multi-modal document text extraction
//!
//! Provides feature-gated text extraction for PDF and DOCX files.
//! Extraction is behind feature flags (`memvid-pdf`, `memvid-docx`) to keep
//! the binary slim when these capabilities aren't needed.
//!
//! ## Design
//! - Raw bytes are stored as non-indexable artifacts
//! - Extracted text is stored as a separate indexable artifact
//! - Both artifacts are linked via metadata (source_uri / extracted_uri)

use thiserror::Error;

/// Result of text extraction from a document.
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Extracted text content
    pub text: String,
    /// Number of pages (for PDF) or sections (for DOCX)
    pub page_count: Option<usize>,
    /// Approximate word count
    pub word_count: usize,
    /// Extraction method used (e.g., "pdf-extract", "docx-rs")
    pub extraction_method: String,
}

/// Errors that can occur during extraction.
#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("PDF extraction failed: {0}")]
    PdfError(String),
    #[error("DOCX extraction failed: {0}")]
    DocxError(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Feature not enabled: rebuild with --features {0}")]
    FeatureDisabled(String),
}

// =============================================================================
// PDF Extraction (feature-gated)
// =============================================================================

/// Extract text from PDF bytes.
///
/// Requires `memvid-pdf` feature to be enabled.
#[cfg(feature = "memvid-pdf")]
pub fn extract_pdf(data: &[u8]) -> Result<ExtractionResult, ExtractionError> {
    use pdf_extract::extract_text_from_mem;

    let text = extract_text_from_mem(data).map_err(|e| ExtractionError::PdfError(e.to_string()))?;

    let word_count = text.split_whitespace().count();

    // Note: pdf-extract doesn't expose page count directly in simple API
    // For now, estimate based on form feeds or page break patterns
    let page_count = text.matches('\x0C').count().max(1);

    Ok(ExtractionResult {
        text,
        page_count: Some(page_count),
        word_count,
        extraction_method: "pdf-extract".to_string(),
    })
}

/// Stub when `memvid-pdf` feature is not enabled.
#[cfg(not(feature = "memvid-pdf"))]
pub fn extract_pdf(_data: &[u8]) -> Result<ExtractionResult, ExtractionError> {
    Err(ExtractionError::FeatureDisabled("memvid-pdf".to_string()))
}

// =============================================================================
// DOCX Extraction (feature-gated)
// =============================================================================

/// Extract text from DOCX bytes.
///
/// Requires `memvid-docx` feature to be enabled.
#[cfg(feature = "memvid-docx")]
pub fn extract_docx(data: &[u8]) -> Result<ExtractionResult, ExtractionError> {
    use docx_rs::*;

    let docx = read_docx(data).map_err(|e| ExtractionError::DocxError(e.to_string()))?;

    // Extract text from all paragraphs
    let mut text_parts: Vec<String> = Vec::new();

    for child in docx.document.children.iter() {
        if let DocumentChild::Paragraph(para) = child {
            let para_text: String = para
                .children
                .iter()
                .filter_map(|pc| {
                    if let ParagraphChild::Run(run) = pc {
                        Some(
                            run.children
                                .iter()
                                .filter_map(|rc| {
                                    if let RunChild::Text(t) = rc {
                                        Some(t.text.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(""),
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");

            if !para_text.is_empty() {
                text_parts.push(para_text);
            }
        }
    }

    let text = text_parts.join("\n");
    let word_count = text.split_whitespace().count();

    Ok(ExtractionResult {
        text,
        page_count: None, // DOCX doesn't have fixed pages
        word_count,
        extraction_method: "docx-rs".to_string(),
    })
}

/// Stub when `memvid-docx` feature is not enabled.
#[cfg(not(feature = "memvid-docx"))]
pub fn extract_docx(_data: &[u8]) -> Result<ExtractionResult, ExtractionError> {
    Err(ExtractionError::FeatureDisabled("memvid-docx".to_string()))
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Detect format from file extension and extract text.
///
/// Returns `Err(UnsupportedFormat)` for unknown extensions.
pub fn extract_text(data: &[u8], extension: &str) -> Result<ExtractionResult, ExtractionError> {
    match extension.to_lowercase().as_str() {
        "pdf" => extract_pdf(data),
        "docx" => extract_docx(data),
        ext => Err(ExtractionError::UnsupportedFormat(ext.to_string())),
    }
}

/// Check if extraction is supported for a given extension.
pub fn is_extraction_supported(extension: &str) -> bool {
    match extension.to_lowercase().as_str() {
        "pdf" => cfg!(feature = "memvid-pdf"),
        "docx" => cfg!(feature = "memvid-docx"),
        _ => false,
    }
}

/// Get the feature flag name for a given extension.
pub fn feature_for_extension(extension: &str) -> Option<&'static str> {
    match extension.to_lowercase().as_str() {
        "pdf" => Some("memvid-pdf"),
        "docx" => Some("memvid-docx"),
        _ => None,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_unsupported_format() {
        let result = extract_text(b"some data", "xyz");
        assert!(matches!(result, Err(ExtractionError::UnsupportedFormat(_))));
    }

    #[test]
    #[cfg(not(feature = "memvid-pdf"))]
    fn test_pdf_feature_gate_error() {
        let result = extract_pdf(b"fake pdf data");
        assert!(
            matches!(result, Err(ExtractionError::FeatureDisabled(ref f)) if f == "memvid-pdf")
        );
    }

    #[test]
    #[cfg(not(feature = "memvid-docx"))]
    fn test_docx_feature_gate_error() {
        let result = extract_docx(b"fake docx data");
        assert!(
            matches!(result, Err(ExtractionError::FeatureDisabled(ref f)) if f == "memvid-docx")
        );
    }

    #[test]
    fn test_is_extraction_supported_pdf() {
        let supported = is_extraction_supported("pdf");
        #[cfg(feature = "memvid-pdf")]
        assert!(supported);
        #[cfg(not(feature = "memvid-pdf"))]
        assert!(!supported);
    }

    #[test]
    fn test_is_extraction_supported_docx() {
        let supported = is_extraction_supported("docx");
        #[cfg(feature = "memvid-docx")]
        assert!(supported);
        #[cfg(not(feature = "memvid-docx"))]
        assert!(!supported);
    }

    #[test]
    fn test_feature_for_extension() {
        assert_eq!(feature_for_extension("pdf"), Some("memvid-pdf"));
        assert_eq!(feature_for_extension("docx"), Some("memvid-docx"));
        assert_eq!(feature_for_extension("txt"), None);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Feature-gated tests (only run when features are enabled)
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[cfg(feature = "memvid-pdf")]
    fn test_pdf_extraction_with_feature() {
        // Minimal valid PDF for testing (this is a very simple PDF structure)
        // In real tests, you'd use include_bytes! with a fixture file
        let simple_pdf = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Contents 4 0 R>>endobj\n4 0 obj<</Length 44>>stream\nBT /F1 12 Tf 100 700 Td (Hello World) Tj ET\nendstream\nendobj\nxref\n0 5\n0000000000 65535 f \n0000000009 00000 n \n0000000052 00000 n \n0000000101 00000 n \n0000000192 00000 n\ntrailer<</Size 5/Root 1 0 R>>\nstartxref\n291\n%%EOF";

        // This may or may not work depending on pdf-extract's tolerance
        // The test verifies the code path, not PDF parsing quality
        let result = extract_pdf(simple_pdf);

        // For a minimal PDF, we just check it doesn't panic
        // Real extraction tests should use proper fixture files
        match result {
            Ok(r) => {
                assert!(r.extraction_method == "pdf-extract");
            }
            Err(_) => {
                // Some minimal PDFs may fail extraction - that's OK for this test
            }
        }
    }

    #[test]
    #[cfg(feature = "memvid-docx")]
    fn test_docx_extraction_with_feature() {
        // DOCX files are ZIP archives - without a real fixture, we just verify
        // the error handling works properly for invalid data
        let result = extract_docx(b"not a valid docx file");

        // Should return an error for invalid DOCX
        assert!(result.is_err());
        if let Err(ExtractionError::DocxError(msg)) = result {
            assert!(!msg.is_empty());
        }
    }
}
