use pdfium_render::prelude::*;
use std::path::Path;
use tauri::{Emitter, Manager, Window};

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

fn parse_page_range(range_str: &str, total_pages: u16) -> Vec<usize> {
    if range_str.trim().is_empty() {
        return (0..total_pages as usize).collect();
    }

    let mut pages = Vec::new();
    for part in range_str.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let bounds: Vec<&str> = part.split('-').collect();
            if bounds.len() == 2 {
                if let (Ok(start), Ok(end)) = (
                    bounds[0].trim().parse::<usize>(),
                    bounds[1].trim().parse::<usize>(),
                ) {
                    let s = start.saturating_sub(1);
                    let e = (end as usize).min(total_pages as usize);
                    for i in s..e {
                        pages.push(i);
                    }
                }
            }
        } else if let Ok(p) = part.parse::<usize>() {
            if p > 0 && p <= total_pages as usize {
                pages.push(p - 1);
            }
        }
    }

    // Remote duplicates and sort
    pages.sort_unstable();
    pages.dedup();
    pages
}

#[tauri::command]
fn convert_pdf(
    window: Window,
    input_paths: Vec<String>,
    output_dir: String,
    format: String,
    scale: f32,
    page_range: String,
    merge: bool,
    quality: u8,
) -> Result<String, String> {
    let resource_dir = window
        .app_handle()
        .path()
        .resource_dir()
        .unwrap_or_else(|_| std::env::current_dir().unwrap());
    let binaries_dir = resource_dir.join("binaries");
    let binaries_dir_str = binaries_dir.to_string_lossy();

    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&binaries_dir))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./src-tauri/")))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./target/release/")))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./target/debug/")))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| format!("Failed to load PDFium library: {}. \n\nTips: \n1. Install libpdfium (e.g., 'sudo apt install libpdfium-dev' on Linux). \n2. Or download the shared library from GitHub and place it next to the app executable.", e))?
    );

    std::env::set_var("FONTCONFIG_PATH", "/etc/fonts");

    for path_str in input_paths {
        let path = Path::new(&path_str);
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

        let document_res = pdfium.load_pdf_from_file(&path_str, None);

        match document_res {
            Ok(document) => {
                let total_pages_in_doc = document.pages().len();
                let target_pages = parse_page_range(&page_range, total_pages_in_doc);
                let total_work = target_pages.len();

                if total_work == 0 {
                    let _ = window.emit(
                        "file_status",
                        FileStatusPayload {
                            filename: filename.to_string(),
                            status: "error".into(),
                            error: Some("No valid pages selected in range".into()),
                            output_path: None,
                        },
                    );
                    continue;
                }

                let mut rendered_images = Vec::new();
                let mut last_output = String::new();

                for (idx, &page_index) in target_pages.iter().enumerate() {
                    let _ = window.emit(
                        "progress",
                        ProgressPayload {
                            filename: filename.to_string(),
                            current: idx + 1,
                            total: total_work,
                        },
                    );

                    if let Ok(page) = document.pages().get(page_index as u16) {
                        let render_width = (page.width().value * scale) as i32;
                        let render_height = (page.height().value * scale) as i32;

                        if let Ok(bitmap) = page.render(render_width, render_height, None) {
                            let image = bitmap.as_image();

                            if merge {
                                rendered_images.push(image);
                            } else {
                                let ext = if format.to_lowercase() == "png" {
                                    "png"
                                } else {
                                    "jpg"
                                };
                                let suffix = if total_work > 1 {
                                    format!("_page_{}", page_index + 1)
                                } else {
                                    "".to_string()
                                };
                                let out_path = Path::new(&output_dir)
                                    .join(format!("{}{}.{}", filename, suffix, ext));

                                let save_res = if ext == "jpg" {
                                    let mut file = std::fs::File::create(&out_path)
                                        .map_err(|e| e.to_string())?;
                                    let mut encoder =
                                        image::codecs::jpeg::JpegEncoder::new_with_quality(
                                            &mut file, quality,
                                        );
                                    encoder.encode_image(&image).map_err(|e| e.to_string())
                                } else {
                                    image.save(&out_path).map_err(|e| e.to_string())
                                };

                                if let Err(e) = save_res {
                                    let _ = window.emit(
                                        "file_status",
                                        FileStatusPayload {
                                            filename: filename.to_string(),
                                            status: "error".into(),
                                            error: Some(format!("Save error: {}", e)),
                                            output_path: None,
                                        },
                                    );
                                } else {
                                    last_output = out_path.to_string_lossy().to_string();
                                }
                            }
                        }
                    }
                }

                if merge && !rendered_images.is_empty() {
                    let total_width = rendered_images
                        .iter()
                        .map(|img| img.width())
                        .max()
                        .unwrap_or(0);
                    let total_height: u32 = rendered_images.iter().map(|img| img.height()).sum();

                    if total_width > 0 && total_height > 0 {
                        let mut combined =
                            image::DynamicImage::new_rgba8(total_width, total_height);
                        let mut current_y = 0;
                        for img in rendered_images {
                            image::imageops::replace(&mut combined, &img, 0, i64::from(current_y));
                            current_y += img.height();
                        }

                        let ext = if format.to_lowercase() == "png" {
                            "png"
                        } else {
                            "jpg"
                        };
                        let out_path =
                            Path::new(&output_dir).join(format!("{}_merged.{}", filename, ext));

                        let save_res = if ext == "jpg" {
                            let mut file =
                                std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
                            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                                &mut file, quality,
                            );
                            encoder.encode_image(&combined).map_err(|e| e.to_string())
                        } else {
                            combined.save(&out_path).map_err(|e| e.to_string())
                        };

                        if let Err(e) = save_res {
                            let _ = window.emit(
                                "file_status",
                                FileStatusPayload {
                                    filename: filename.to_string(),
                                    status: "error".into(),
                                    error: Some(format!("Merge save error: {}", e)),
                                    output_path: None,
                                },
                            );
                        } else {
                            last_output = out_path.to_string_lossy().to_string();
                        }
                    }
                }

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
