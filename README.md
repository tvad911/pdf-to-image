# PDF to Image Converter

A high-performance desktop application built with **Rust**, **Tauri**, and **TypeScript** to batch convert PDF files into images (JPG/PNG). This application processes files locally on your machine using `pdfium-render`, ensuring privacy and speed.

## 🚀 Features

- **Batch Processing**: Select and convert multiple PDF files at once.
- **High Performance**: Powered by Rust and `pdfium` for fast rendering.
- **Customizable Output**:
  - **Formats**: Support for `JPG` and `PNG`.
  - **Scaling**: Adjust quality with 1x, 2x, or 4x scaling options.
- **Real-time Progress**: Track the status (Queued, Processing, Success, Error) of each file.
- **Privacy Focused**: All processing happens locally; no files are uploaded to the cloud.

## 🛠️ Tech Stack

- **Frontend**: TypeScript, HTML, CSS (Vanilla, no heavy frameworks).
- **Backend**: Rust (Tauri framework).
- **PDF Engine**: `pdfium-render` (bindings to Google's PDFium).
- **Window Management**: Tauri.

## 📋 Prerequisites

Before running the project, ensure you have the following installed:

1.  **Node.js**: v14 or later.
2.  **Rust**: The latest stable version. [Install Rust](https://www.rust-lang.org/tools/install).
3.  **Build Tools**:
    - Linux: `sudo apt install libwebkit2gtk-4.0-dev build-essential curl wget libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`

> **Note on PDFium**: The app uses `pdfium-render`. On some systems, you might need to ensure the dynamic library for PDFium is available, although the crate attempts to bundle or locate it.

## 💻 Installation & Development

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/yourusername/pdf-to-image-converter.git
    cd pdf-to-image-converter
    ```

2.  **Install frontend dependencies**:
    ```bash
    npm install
    ```

3.  **Run in Development Mode**:
    This will start the Tauri window with hot-reload enabled.
    ```bash
    npm run tauri dev
    ```

## 📦 Building for Production

To create an optimized executable for your OS:

```bash
npm run tauri build
```
The output binary will be located in `src-tauri/target/release/`.

## 📖 Usage Guide

1.  **Select Files**: Click "Select PDF Files" to choose one or more documents.
2.  **Choose Output Folder**: Click "Select Output Folder" to choose where images will be saved.
3.  **Configure**: Use the dropdowns to select your desired format (JPG/PNG) and scale (e.g., 2.0x for higher quality).
4.  **Convert**: Hit "Start Conversion" and wait for the process to finish.
5.  **View Results**: Click "Open Output Folder" when done to see your images.

## 📄 License

[MIT License](LICENSE)
