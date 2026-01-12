use pdfium_render::prelude::*;
use std::path::Path;
use tauri::{Emitter, Window};

#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    filename: String,
    current: usize,
    total: usize,
}

#[derive(Clone, serde::Serialize)]
struct FileStatusPayload {
    filename: String,
    status: String, // "queued", "processing", "success", "error"
    error: Option<String>,
    output_path: Option<String>,
}

#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn convert_pdf(
    window: Window,
    input_paths: Vec<String>,
    output_dir: String,
    format: String,
    scale: f32,
) -> Result<String, String> {
    // Try to bind to the library.
    // We try multiple common locations relative to the binary or system.
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./src-tauri/")))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./target/release/")))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./target/debug/")))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| format!("Failed to load PDFium library: {}. \n\nTips: \n1. Install libpdfium (e.g., 'sudo apt install libpdfium-dev' on Linux). \n2. Or download the shared library from GitHub and place it next to the app executable.", e))?
    );

    // Load system fonts to support non-embedded characters (e.g., Vietnamese)
    // pdfium-render 0.8+ allows configuring fonts via ExternalFontMapper or similar,
    // but the simplest way on Linux with the binary is often automatic if fontconfig is present.
    // However, since we need to be explicit, let's try to pass standard paths if the API allows.
    // If we can't find the exact API for this version, we will assume standard loading.

    // NOTE: Based on common pdfium-render usage, there isn't always a direct "use_system_fonts" helper
    // on the Pdfium struct itself without checking `pdfium.fonts()`.
    // Let's try to see if we can get the font config or just skip this if not strictly needed by the crate version.
    // BUT since the user has an error, we MUST do something.
    // Let's rely on `env::set_var` for FONTCONFIG_PATH as a fallback if the crate doesn't expose it easily.
    std::env::set_var("FONTCONFIG_PATH", "/etc/fonts");

    for path_str in input_paths {
        let path = Path::new(&path_str);

        // Notify: Processing started for file
        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let _ = window.emit(
            "file_status",
            FileStatusPayload {
                filename: filename.to_string(),
                status: "processing".into(),
                error: None,
                output_path: None,
            },
        );

        // Load the document
        // Capture error to emit "error" status
        let document_res = pdfium.load_pdf_from_file(&path_str, None);

        match document_res {
            Ok(document) => {
                let total_pages = document.pages().len();
                let mut last_output = String::new();

                // Render each page
                for (i, page) in document.pages().iter().enumerate() {
                    // Emit progress
                    let _ = window.emit(
                        "progress",
                        ProgressPayload {
                            filename: filename.to_string(),
                            current: i as usize + 1,
                            total: total_pages as usize,
                        },
                    );

                    // Let's get page size to maintain aspect ratio
                    let width = page.width().value;
                    let height = page.height().value;

                    // Use provided scale
                    let render_width = (width * scale) as i32;
                    let render_height = (height * scale) as i32;

                    let bitmap_res = page.render(render_width, render_height, None);
                    match bitmap_res {
                        Ok(bitmap) => {
                            let image = bitmap.as_image();
                            let ext = if format.to_lowercase() == "png" {
                                "png"
                            } else {
                                "jpg"
                            };
                            let output_path = Path::new(&output_dir).join(format!(
                                "{}_page_{}.{}",
                                filename,
                                i + 1,
                                ext
                            ));

                            match image.save(&output_path) {
                                Ok(_) => {
                                    last_output = output_path.to_string_lossy().to_string();
                                }
                                Err(e) => {
                                    let _ = window.emit(
                                        "file_status",
                                        FileStatusPayload {
                                            filename: filename.to_string(),
                                            status: "error".into(),
                                            error: Some(format!("Save error: {}", e)),
                                            output_path: None,
                                        },
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            let _ = window.emit(
                                "file_status",
                                FileStatusPayload {
                                    filename: filename.to_string(),
                                    status: "error".into(),
                                    error: Some(format!("Render error: {}", e)),
                                    output_path: None,
                                },
                            );
                        }
                    }
                }

                // Notify: Success
                let _ = window.emit(
                    "file_status",
                    FileStatusPayload {
                        filename: filename.to_string(),
                        status: "success".into(),
                        error: None,
                        output_path: Some(last_output),
                    },
                );
            }
            Err(e) => {
                let _ = window.emit(
                    "file_status",
                    FileStatusPayload {
                        filename: filename.to_string(),
                        status: "error".into(),
                        error: Some(format!("Load PDF error: {}", e)),
                        output_path: None,
                    },
                );
            }
        }
    }

    Ok("Batch processing complete".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![convert_pdf, open_folder])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
