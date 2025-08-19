use anyhow::{anyhow, Result};
use lopdf::{Document, Object};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

pub struct PdfMerger {
    documents: Vec<(String, Document)>,
}

impl PdfMerger {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
        }
    }

    pub async fn add_pdf(&mut self, path: &Path) -> Result<()> {
        let data = fs::read(path)
            .await
            .map_err(|e| anyhow!("Failed to read PDF file {}: {}", path.display(), e))?;

        let document = Document::load_mem(&data)
            .map_err(|e| anyhow!("Failed to parse PDF file {}: {}", path.display(), e))?;

        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.pdf")
            .to_string();

        debug!("Loaded PDF with {} pages from {}", document.get_pages().len(), path.display());
        self.documents.push((filename, document));

        Ok(())
    }

    pub async fn save(&self, output_path: &Path) -> Result<()> {
        if self.documents.is_empty() {
            return Err(anyhow!("No PDFs added to merge"));
        }

        if self.documents.len() == 1 {
            // If only one document, just copy it
            let data = fs::read(&output_path).await.unwrap_or_default();
            fs::write(output_path, data).await?;
            return Ok(());
        }

        info!("Starting PDF merge process with {} documents", self.documents.len());

        // Use the first document as the base
        let mut merged_doc = self.documents[0].1.clone();
        let mut all_page_ids = Vec::new();
        
        // Collect page IDs from the first document
        let first_pages = merged_doc.get_pages();
        debug!("First document has {} pages", first_pages.len());
        for (_, page_id) in first_pages {
            all_page_ids.push(page_id);
        }

        // Add pages from remaining documents
        let mut max_id = merged_doc.max_id;
        
        for (i, (filename, document)) in self.documents.iter().skip(1).enumerate() {
            debug!("Processing document {}: {} with {} pages", 
                   i + 2, filename, document.get_pages().len());
            
            let mut doc_copy = document.clone();
            
            // Renumber objects to avoid conflicts
            doc_copy.renumber_objects_with(max_id + 1);
            max_id = doc_copy.max_id;
            
            // Get pages from this document
            let pages = doc_copy.get_pages();
            
            // Copy all objects from this document
            for (obj_id, obj) in doc_copy.objects.iter() {
                merged_doc.objects.insert(*obj_id, obj.clone());
            }
            
            // Add page IDs to our list
            for (_, page_id) in pages {
                all_page_ids.push(page_id);
            }
        }

        info!("Total pages collected: {}", all_page_ids.len());

        // Update the Pages object to reference all pages
        if let Ok(catalog) = merged_doc.catalog() {
            if let Ok(pages_ref) = catalog.get(b"Pages") {
                if let Object::Reference(pages_id) = pages_ref {
                    if let Ok(pages_obj) = merged_doc.get_object_mut(*pages_id) {
                        if let Object::Dictionary(ref mut pages_dict) = pages_obj {
                            // Update the Kids array with all page references
                            pages_dict.set("Kids", Object::Array(
                                all_page_ids.into_iter().map(Object::Reference).collect()
                            ));
                            
                            // Update the Count
                            if let Ok(Object::Array(ref kids)) = pages_dict.get(b"Kids") {
                                let kids_len = kids.len();
                                pages_dict.set("Count", Object::Integer(kids_len as i64));
                                debug!("Updated Pages object with {} kids", kids_len);
                            }
                        }
                    }
                }
            }
        }

        // Update max_id and renumber if needed
        merged_doc.max_id = max_id;

        let final_page_count = if let Ok(catalog) = merged_doc.catalog() {
            if let Ok(pages_ref) = catalog.get(b"Pages") {
                if let Object::Reference(pages_id) = pages_ref {
                    if let Ok(pages_obj) = merged_doc.get_object(*pages_id) {
                        if let Object::Dictionary(ref pages_dict) = pages_obj {
                            if let Ok(Object::Integer(count)) = pages_dict.get(b"Count") {
                                *count
                            } else { 0 }
                        } else { 0 }
                    } else { 0 }
                } else { 0 }
            } else { 0 }
        } else { 0 };

        info!("Finalizing merged PDF with {} total pages", final_page_count);

        // Save the merged document
        let mut data = Vec::new();
        merged_doc
            .save_to(&mut data)
            .map_err(|e| anyhow!("Failed to serialize merged PDF: {}", e))?;

        fs::write(output_path, data)
            .await
            .map_err(|e| anyhow!("Failed to write merged PDF to {}: {}", output_path.display(), e))?;

        info!("Successfully merged {} PDFs into {}", 
              self.documents.len(), output_path.display());
        Ok(())
    }
}

impl Default for PdfMerger {
    fn default() -> Self {
        Self::new()
    }
}