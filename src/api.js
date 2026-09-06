import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { open as openPath } from "@tauri-apps/plugin-shell";

// ---- Settings ----
export const getSettings = () => invoke("get_settings");
export const saveSettings = (settings) => invoke("save_settings", { settings });

// ---- Catalog ----
export const listCatalogItems = () => invoke("list_catalog_items");
export const saveCatalogItem = (item) => invoke("save_catalog_item", { item });
export const deleteCatalogItem = (id) => invoke("delete_catalog_item", { id });

// ---- Invoices ----
export const listInvoices = () => invoke("list_invoices");
export const getInvoice = (id) => invoke("get_invoice", { id });
export const createInvoice = (input) => invoke("create_invoice", { input });
export const regenerateInvoicePdf = (invoiceId) =>
  invoke("regenerate_invoice_pdf", { invoiceId });

// ---- Native folder picker (for Settings > PDF output folder) ----
export const pickFolder = async () => {
  const selected = await openDialog({ directory: true, multiple: false });
  return selected; // string path, or null if cancelled
};

// ---- Open a generated PDF with the system default viewer ----
export const openFile = (path) => openPath(path);

export const INDIAN_STATES = [
  { name: "Jammu and Kashmir", code: "01" },
  { name: "Himachal Pradesh", code: "02" },
  { name: "Punjab", code: "03" },
  { name: "Chandigarh", code: "04" },
  { name: "Uttarakhand", code: "05" },
  { name: "Haryana", code: "06" },
  { name: "Delhi", code: "07" },
  { name: "Rajasthan", code: "08" },
  { name: "Uttar Pradesh", code: "09" },
  { name: "Bihar", code: "10" },
  { name: "Sikkim", code: "11" },
  { name: "Arunachal Pradesh", code: "12" },
  { name: "Nagaland", code: "13" },
  { name: "Manipur", code: "14" },
  { name: "Mizoram", code: "15" },
  { name: "Tripura", code: "16" },
  { name: "Meghalaya", code: "17" },
  { name: "Assam", code: "18" },
  { name: "West Bengal", code: "19" },
  { name: "Jharkhand", code: "20" },
  { name: "Odisha", code: "21" },
  { name: "Chhattisgarh", code: "22" },
  { name: "Madhya Pradesh", code: "23" },
  { name: "Gujarat", code: "24" },
  { name: "Daman and Diu", code: "25" },
  { name: "Dadra and Nagar Haveli", code: "26" },
  { name: "Maharashtra", code: "27" },
  { name: "Andhra Pradesh (Old)", code: "28" },
  { name: "Karnataka", code: "29" },
  { name: "Goa", code: "30" },
  { name: "Lakshadweep", code: "31" },
  { name: "Kerala", code: "32" },
  { name: "Tamil Nadu", code: "33" },
  { name: "Puducherry", code: "34" },
  { name: "Andaman and Nicobar Islands", code: "35" },
  { name: "Telangana", code: "36" },
  { name: "Andhra Pradesh (New)", code: "37" },
  { name: "Ladakh", code: "38" },
];

export const GST_RATES = [0, 5, 12, 18, 28];
