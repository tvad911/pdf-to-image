import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";

interface ProgressPayload {
  filename: string;
  current: number;
  total: number;
}

interface FileStatusPayload {
  filename: string;
  status: "queued" | "processing" | "success" | "error";
  error?: string;
  output_path?: string;
}

interface FileState {
  filename: string;
  status: "queued" | "processing" | "success" | "error";
  progressCurrent: number;
  progressTotal: number;
  error?: string;
}

let selectedFiles: string[] = [];
let outputDirectory: string | null = null;
let unlistenProgress: (() => void) | null = null;
let unlistenStatus: (() => void) | null = null;
let fileStates = new Map<string, FileState>();

// Elements
const selectFilesBtn = document.getElementById("select-files-btn") as HTMLButtonElement;
const selectDirBtn = document.getElementById("select-dir-btn") as HTMLButtonElement;
const convertBtn = document.getElementById("convert-btn") as HTMLButtonElement;
const openOutputBtn = document.getElementById("open-output-btn") as HTMLButtonElement;
const fileTableBody = document.getElementById("file-table-body") as HTMLTableSectionElement;
const outputDirInp = document.getElementById("output-dir") as HTMLInputElement;
const formatSelect = document.getElementById("format-select") as HTMLSelectElement;
const scaleSelect = document.getElementById("scale-select") as HTMLSelectElement;
const statusContainer = document.getElementById("status-container") as HTMLDivElement;
const statusMsg = document.getElementById("status-msg") as HTMLParagraphElement;
const spinner = document.querySelector(".spinner") as HTMLDivElement;
const pageRangeInp = document.getElementById("page-range") as HTMLInputElement;
const qualitySlider = document.getElementById("quality-slider") as HTMLInputElement;
const qualityVal = document.getElementById("quality-val") as HTMLSpanElement;
const qualitySection = document.getElementById("quality-section") as HTMLDivElement;
const mergeCheckbox = document.getElementById("merge-checkbox") as HTMLInputElement;

// Handle Quality Visibility and Label
formatSelect.addEventListener("change", () => {
  if (formatSelect.value === "jpg") {
    qualitySection.style.display = "block";
  } else {
    qualitySection.style.display = "none";
  }
});

// Initial show/hide quality
if (formatSelect.value === "jpg") {
  qualitySection.style.display = "block";
}

qualitySlider.addEventListener("input", () => {
  qualityVal.textContent = qualitySlider.value;
});

// Helper to get basenames
const getBasename = (path: string) => path.split(/[\\/]/).pop() || "unknown";
// ... (renderTable and updateUI remain the same, adding them inside replace_file_content if needed)

// Helper to render table
function renderTable() {
  fileTableBody.innerHTML = "";
  if (selectedFiles.length === 0) {
    fileTableBody.innerHTML = '<tr class="empty-state"><td colspan="3">No files selected</td></tr>';
    return;
  }

  selectedFiles.forEach((path) => {
    const filename = getBasename(path);
    const stem = filename.replace(/\.pdf$/i, "");

    // Initial state if not exists
    if (!fileStates.has(stem)) {
      fileStates.set(stem, {
        filename: stem,
        status: "queued",
        progressCurrent: 0,
        progressTotal: 0
      });
    }

    const state = fileStates.get(stem)!;

    const tr = document.createElement("tr");
    tr.className = "file-row";

    // Column 1: File
    const tdName = document.createElement("td");
    tdName.innerHTML = `<span class="icon">📄</span> ${filename}`;
    tr.appendChild(tdName);

    // Column 2: Status
    const tdStatus = document.createElement("td");
    const badge = document.createElement("span");
    badge.className = `status-badge ${state.status}`;
    badge.textContent = state.status.toUpperCase();
    tdStatus.appendChild(badge);
    tr.appendChild(tdStatus);

    // Column 3: Progress
    const tdProgress = document.createElement("td");
    if (state.status === "processing") {
      tdProgress.textContent = `${state.progressCurrent} / ${state.progressTotal}`;
    } else if (state.status === "success") {
      tdProgress.textContent = "Done";
    } else if (state.status === "error") {
      tdProgress.textContent = state.error || "Failed";
      tdProgress.style.color = "#f87171";
    } else {
      tdProgress.textContent = "-";
    }
    tr.appendChild(tdProgress);

    fileTableBody.appendChild(tr);
  });
}

function updateUI() {
  renderTable();

  // Update Output Dir
  if (outputDirectory) {
    outputDirInp.value = outputDirectory;
  }

  // Enable/Disable Convert Button
  convertBtn.disabled = selectedFiles.length === 0 || !outputDirectory;
}

// SETUP LISTENERS
async function setupListeners() {
  if (unlistenProgress) unlistenProgress();
  if (unlistenStatus) unlistenStatus();

  unlistenProgress = await listen<ProgressPayload>("progress", (event) => {
    const { filename, current, total } = event.payload;
    const state = fileStates.get(filename);
    if (state) {
      state.progressCurrent = current;
      state.progressTotal = total;
      if (state.status !== "processing") state.status = "processing";
      renderTable();
    }
  });

  unlistenStatus = await listen<FileStatusPayload>("file_status", (event) => {
    const { filename, status, error } = event.payload;
    const state = fileStates.get(filename);
    if (state) {
      state.status = status;
      if (error) state.error = error;
      renderTable();
      if (status === "success") {
        openOutputBtn.classList.remove("hidden");
      }
    }
  });
}
setupListeners();

selectFilesBtn.addEventListener("click", async () => {
  const result = await open({
    multiple: true,
    filters: [{ name: "PDF Files", extensions: ["pdf"] }],
  });

  if (result) {
    selectedFiles = result as string[];
    fileStates.clear();
    openOutputBtn.classList.add("hidden");
    statusContainer.classList.add("hidden");
    updateUI();
  }
});

selectDirBtn.addEventListener("click", async () => {
  const result = await open({
    directory: true,
  });

  if (result) {
    outputDirectory = result as string;
    updateUI();
  }
});

openOutputBtn.addEventListener("click", () => {
  if (outputDirectory) {
    invoke("open_folder", { path: outputDirectory });
  }
});

convertBtn.addEventListener("click", async () => {
  if (selectedFiles.length === 0 || !outputDirectory) return;

  statusContainer.classList.remove("hidden");
  spinner.style.display = "block";
  convertBtn.disabled = true;
  statusMsg.textContent = "Processing...";
  statusMsg.style.color = "var(--text-muted)";

  fileStates.forEach(s => { s.status = "queued"; s.error = undefined; });
  renderTable();

  try {
    await invoke("convert_pdf", {
      inputPaths: selectedFiles,
      outputDir: outputDirectory,
      format: formatSelect.value,
      scale: parseFloat(scaleSelect.value),
      pageRange: pageRangeInp.value,
      merge: mergeCheckbox.checked,
      quality: parseInt(qualitySlider.value)
    });
    statusMsg.textContent = "Batch Completed! ✅";
    statusMsg.style.color = "#4ade80";
  } catch (error) {
    console.error(error);
    statusMsg.textContent = `Error: ${error} ❌`;
    statusMsg.style.color = "#f87171";
  } finally {
    convertBtn.disabled = false;
    spinner.style.display = "none";
  }
});
