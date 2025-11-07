use anyhow::{anyhow, Result};
use lopdf::{Document, Object};
use std::path::{Path, PathBuf};
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

    /// Merge all PDF files from a directory into a single PDF
    pub async fn merge_directory(input_dir: &str, output_file: &str) -> Result<()> {
        let input_path = PathBuf::from(input_dir);
        
        if !input_path.exists() {
            return Err(anyhow!("Input directory '{input_dir}' does not exist"));
        }

        info!("Scanning directory: {}", input_dir);
        
        let mut entries = fs::read_dir(&input_path).await?;
        let mut pdf_files = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension == "pdf" {
                    pdf_files.push(path);
                }
            }
        }
        
        if pdf_files.is_empty() {
            return Err(anyhow!("No PDF files found in '{input_dir}'"));
        }
        
        // Sort by filename to maintain order (especially numbered files)
        pdf_files.sort();
        
        info!("Found {} PDF files to merge:", pdf_files.len());
        for (i, path) in pdf_files.iter().enumerate() {
            info!("  {}: {}", i + 1, &path.file_name().unwrap().to_string_lossy());
        }
        
        let mut merger = PdfMerger::new();
        
        for pdf_path in &pdf_files {
            info!("Adding: {}", pdf_path.display());
            if let Err(e) = merger.add_pdf(pdf_path).await {
                tracing::error!("Failed to add PDF {}: {}", pdf_path.display(), e);
            }
        }
        
        let output_path = PathBuf::from(output_file);
        merger.save(&output_path).await?;
        
        info!("Successfully merged {} PDFs into: {}", 
              pdf_files.len(), 
              &output_path.display().to_string());
        
        Ok(())
    }

    pub async fn save(mut self, output_path: &Path) -> Result<()> {
        if self.documents.is_empty() {
            return Err(anyhow!("No PDFs added to merge"));
        }

        if self.documents.len() == 1 {
            // If only one document, save it directly
            let mut data = Vec::new();
            self.documents[0].1.save_to(&mut data)
                .map_err(|e| anyhow!("Failed to save single PDF: {e}"))?;
            fs::write(output_path, data).await?;
            info!("Saved single PDF to {}", output_path.display());
            return Ok(());
        }

        info!("Starting PDF merge process with {} documents", self.documents.len());

        // Take ownership of the first document to avoid cloning
        let mut merged_doc = self.documents.remove(0).1;
        let mut all_page_ids = Vec::new();
        
        // Collect page IDs from the first document
        let first_pages = merged_doc.get_pages();
        debug!("First document has {} pages", first_pages.len());
        for (_, page_id) in first_pages {
            all_page_ids.push(page_id);
        }

        // Add pages from remaining documents
        let mut max_id = merged_doc.max_id;
        
        // Process documents one by one and immediately drop them
        while !self.documents.is_empty() {
            let (filename, mut document) = self.documents.remove(0);
            debug!("Processing document: {} with {} pages", 
                   filename, document.get_pages().len());
            
            // Renumber objects to avoid conflicts
            document.renumber_objects_with(max_id + 1);
            max_id = document.max_id;
            
            // Get pages from this document
            let pages = document.get_pages();
            
            // Move all objects from this document (avoiding clone)
            for (obj_id, obj) in document.objects.into_iter() {
                merged_doc.objects.insert(obj_id, obj);
            }
            
            // Add page IDs to our list
            for (_, page_id) in pages {
                all_page_ids.push(page_id);
            }
            
            // Document is automatically dropped here, freeing memory
        }

        info!("Total pages collected: {}", all_page_ids.len());

        // Update the Pages object to reference all pages
        if let Ok(catalog) = merged_doc.catalog() {
            if let Ok(Object::Reference(pages_id)) = catalog.get(b"Pages") {
                if let Ok(Object::Dictionary(ref mut pages_dict)) = merged_doc.get_object_mut(*pages_id) {
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

        // Update max_id and renumber if needed
        merged_doc.max_id = max_id;

        let final_page_count = if let Ok(catalog) = merged_doc.catalog() {
            if let Ok(Object::Reference(pages_id)) = catalog.get(b"Pages") {
                if let Ok(Object::Dictionary(ref pages_dict)) = merged_doc.get_object(*pages_id) {
                    if let Ok(Object::Integer(count)) = pages_dict.get(b"Count") {
                        *count
                    } else { 0 }
                } else { 0 }
            } else { 0 }
        } else { 0 };

        info!("Finalizing merged PDF with {} total pages", final_page_count);

        // Save the merged document
        let mut data = Vec::new();
        merged_doc
            .save_to(&mut data)
            .map_err(|e| anyhow!("Failed to serialize merged PDF: {e}"))?;

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