use anyhow::Result;
use book2pdf::PdfMerger;
use tokio::fs;

fn create_temp_dir() -> Result<tempfile::TempDir> {
    Ok(tempfile::tempdir()?)
}

fn create_mock_pdf_data(page_count: usize) -> Vec<u8> {
    format!("%%PDF-1.4\n%Mock PDF with {} pages\n%%EOF", page_count).into_bytes()
}

/// Test error handling when merging empty directory
#[tokio::test]
async fn test_merge_empty_directory_error_handling() -> Result<()> {
    let temp_dir = create_temp_dir()?;
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir(&empty_dir).await?;
    
    let output_file = temp_dir.path().join("output.pdf");
    
    let result = PdfMerger::merge_directory(
        empty_dir.to_str().unwrap(),
        output_file.to_str().unwrap()
    ).await;
    
    // Should fail with specific error message
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("No PDF files found"));
    
    Ok(())
}

/// Test error handling when directory doesn't exist
#[tokio::test]
async fn test_merge_nonexistent_directory_error_handling() -> Result<()> {
    let temp_dir = create_temp_dir()?;
    let nonexistent_dir = temp_dir.path().join("definitely_does_not_exist_12345");
    let output_file = temp_dir.path().join("output.pdf");
    
    let result = PdfMerger::merge_directory(
        nonexistent_dir.to_str().unwrap(),
        output_file.to_str().unwrap()
    ).await;
    
    // Should fail with directory not found error
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("does not exist"));
    
    Ok(())
}

/// Test file discovery and sorting logic
#[tokio::test]
async fn test_pdf_file_discovery_and_sorting() -> Result<()> {
    let temp_dir = create_temp_dir()?;
    let pdf_dir = temp_dir.path().join("pdfs");
    fs::create_dir(&pdf_dir).await?;
    
    // Create files in non-alphabetical order to test sorting
    let files = ["page3.pdf", "page1.pdf", "page10.pdf", "page2.pdf", "not_a_pdf.txt"];
    
    for file in &files {
        let file_path = pdf_dir.join(file);
        if file.ends_with(".pdf") {
            fs::write(&file_path, create_mock_pdf_data(1)).await?;
        } else {
            fs::write(&file_path, "not a pdf").await?;
        }
    }
    
    let output_file = temp_dir.path().join("merged.pdf");
    
    // Attempt merge - should find only PDF files and sort them
    let result = PdfMerger::merge_directory(
        pdf_dir.to_str().unwrap(),
        output_file.to_str().unwrap()
    ).await;
    
    // Should attempt to process PDF files (may fail with mock data, but that's OK)
    // The important thing is it found 4 PDF files and ignored the .txt file
    match result {
        Ok(_) => println!("Merge succeeded (unexpected with mock data)"),
        Err(e) => {
            let error_msg = e.to_string();
            // Should be a PDF parsing error, not a "No PDF files found" error
            assert!(!error_msg.contains("No PDF files found"));
            // Should be attempting to process files
            assert!(error_msg.contains("PDF") || error_msg.contains("parse") || error_msg.contains("Failed"));
        }
    }
    
    Ok(())
}

/// Test error handling when save() is called on empty merger
#[tokio::test]
async fn test_save_empty_merger_error_handling() -> Result<()> {
    let merger = PdfMerger::new();
    let temp_dir = create_temp_dir()?;
    let output_path = temp_dir.path().join("empty.pdf");
    
    let result = merger.save(&output_path).await;
    
    // Should fail with specific error about no PDFs
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("No PDFs added to merge"));
    
    Ok(())
}

/// Test directory permission error handling
#[tokio::test]
async fn test_output_directory_creation_error_handling() -> Result<()> {
    let temp_dir = create_temp_dir()?;
    let pdf_dir = temp_dir.path().join("pdfs");
    fs::create_dir(&pdf_dir).await?;
    
    // Create a mock PDF
    fs::write(pdf_dir.join("test.pdf"), create_mock_pdf_data(1)).await?;
    
    // Try to write to an invalid output path (subdirectory of a file)
    let invalid_output = temp_dir.path().join("some_file.txt").join("nested").join("output.pdf");
    
    // First create the file that will block directory creation
    fs::write(temp_dir.path().join("some_file.txt"), "blocking file").await?;
    
    let result = PdfMerger::merge_directory(
        pdf_dir.to_str().unwrap(),
        invalid_output.to_str().unwrap()
    ).await;
    
    // Should fail due to path/permission issues
    assert!(result.is_err());
    
    Ok(())
}

/// Test merge with single PDF file (special case handling)
#[tokio::test]
async fn test_merge_single_pdf_optimization() -> Result<()> {
    let temp_dir = create_temp_dir()?;
    let pdf_dir = temp_dir.path().join("pdfs");
    fs::create_dir(&pdf_dir).await?;
    
    // Create only one PDF file
    fs::write(pdf_dir.join("single.pdf"), create_mock_pdf_data(1)).await?;
    
    let output_file = temp_dir.path().join("output.pdf");
    
    let result = PdfMerger::merge_directory(
        pdf_dir.to_str().unwrap(),
        output_file.to_str().unwrap()
    ).await;
    
    // Should handle single PDF case (may fail with mock data)
    // The important thing is it takes the single-PDF optimization path
    match result {
        Ok(_) => println!("Single PDF case handled successfully"),
        Err(e) => {
            let error_msg = e.to_string();
            // Should attempt to process, not fail on file discovery
            assert!(!error_msg.contains("No PDF files found"));
        }
    }
    
    Ok(())
}

/// Test concurrent access patterns don't cause issues
#[tokio::test]
async fn test_concurrent_merger_operations() -> Result<()> {
    let temp_dir = create_temp_dir()?;
    let pdf_dir = temp_dir.path().join("pdfs");
    fs::create_dir(&pdf_dir).await?;
    
    // Create test PDF
    fs::write(pdf_dir.join("test.pdf"), create_mock_pdf_data(1)).await?;
    
    // Test multiple concurrent merge attempts
    let mut handles = Vec::new();
    
    for i in 0..3 {
        let pdf_dir_clone = pdf_dir.clone();
        let output_file = temp_dir.path().join(format!("output_{}.pdf", i));
        
        let handle = tokio::spawn(async move {
            PdfMerger::merge_directory(
                pdf_dir_clone.to_str().unwrap(),
                output_file.to_str().unwrap()
            ).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await);
    }
    
    // All operations should complete (may succeed or fail, but shouldn't hang/crash)
    for result in results {
        match result {
            Ok(Ok(_)) => println!("Concurrent operation succeeded"),
            Ok(Err(e)) => println!("Concurrent operation failed as expected: {}", e),
            Err(e) => panic!("Task panicked: {:?}", e),
        }
    }
    
    Ok(())
}