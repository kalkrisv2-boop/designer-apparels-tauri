import { invoke } from "@tauri-apps/api/core";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";

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

// ---- Products (Phase 1, vm_products via platform_db) ----
export const listProducts = () => invoke("list_products");
export const addProduct = (input) => invoke("add_product", { input });
export const updateProduct = (productId, input) => invoke("update_product", { productId, input });
export const deleteProduct = (productId) => invoke("delete_product", { productId });
export const searchProducts = (q) => invoke("search_products", { q });
export const styleVariants = (model) => invoke("style_variants", { model });
export const quickAddProduct = (input) => invoke("quick_add_product", { input });

// ---- Customers (Phase 1, vm_customer via platform_db) ----
export const listCustomers = () => invoke("list_customers");
export const addCustomer = (input) => invoke("add_customer", { input });
export const updateCustomer = (customerId, input) => invoke("update_customer", { customerId, input });
export const deleteCustomer = (customerId) => invoke("delete_customer", { customerId });

// ---- Suppliers (Phase 1, vm_supplier via platform_db) ----
export const listSuppliers = () => invoke("list_suppliers");
export const addSupplier = (input) => invoke("add_supplier", { input });
export const updateSupplier = (supplierId, input) => invoke("update_supplier", { supplierId, input });
export const deleteSupplier = (supplierId) => invoke("delete_supplier", { supplierId });

// ---- Invoices (Phase 1, read-only against vm_billentry/vm_billitems) ----
export const findBillByNumber = (billNumber) => invoke("find_bill_by_number", { billNumber });
export const getInvoiceData = (billId) => invoke("get_invoice_data", { billId });

// ---- Billing / POS checkout (Phase 1, writes vm_billentry/vm_billitems) ----
export const billingInit = () => invoke("billing_init");
export const checkout = (input) => invoke("checkout", { input });

// ---- Accounts: Vouchers / Ledger / Daybook (Phase 2) ----
export const accountsInit = () => invoke("accounts_init");
export const addVoucher = (input) => invoke("add_voucher", { input });
export const listLedger = (dateFrom, dateTo) => invoke("list_ledger", { dateFrom, dateTo });
export const listDaybook = (dateFrom, dateTo) => invoke("list_daybook", { dateFrom, dateTo });

// ---- Purchases: stock IN (Phase 3, writes vm_purentry/vm_puritems) ----
export const purchasesInit = () => invoke("purchases_init");
export const purchasesSearchProducts = (q) => invoke("purchases_search_products", { q });
export const purchasesCheckout = (input) => invoke("purchases_checkout", { input });

// ---- Purchase Returns: stock OUT (Phase 3, writes vm_purreturnentry/vm_purreturnitem) ----
export const purchaseReturnsInit = () => invoke("purchase_returns_init");
export const purchasesSearchProductsForReturn = (q) => invoke("purchases_search_products_for_return", { q });
export const purchaseReturnsCheckout = (input) => invoke("purchase_returns_checkout", { input });

// ---- Sales Returns: customer credit notes/refunds (Phase 4, writes vm_salreturnentry/vm_salreturnitem) ----
export const salesReturnsInit = () => invoke("sales_returns_init");
export const salesReturnsSearchProducts = (q) => invoke("sales_returns_search_products", { q });
export const salesReturnsLookupBill = (billNumber) => invoke("sales_returns_lookup_bill", { billNumber });
export const salesReturnsCheckout = (input) => invoke("sales_returns_checkout", { input });

// ---- Reports (Phase 5, read-only against vm_billitems/vm_billentry/vm_customer/vm_salreturnentry/vm_transaction) ----
export const gstReport = (dateFrom, dateTo) =>
  invoke("gst_report", { dateFrom: dateFrom || undefined, dateTo: dateTo || undefined });
export const profitReport = (dateFrom, dateTo) =>
  invoke("profit_report", { dateFrom: dateFrom || undefined, dateTo: dateTo || undefined });
export const customerReport = (dateFrom, dateTo, customerId) =>
  invoke("customer_report", {
    dateFrom: dateFrom || undefined,
    dateTo: dateTo || undefined,
    customerId: customerId || undefined,
  });

// GSTR-1 export needs a real file path up front (Rust writes the zip
// directly to disk, there's no "download" concept in a desktop app) --
// so this wraps the native save dialog + the invoke into one call.
// Returns null if the user cancelled the dialog.
export const exportGstr1 = async (dateFrom, dateTo) => {
  const suggestedName = `GSTR1_${dateFrom || "all"}_to_${dateTo || "all"}.zip`;
  const savePath = await saveDialog({
    defaultPath: suggestedName,
    filters: [{ name: "Zip Archive", extensions: ["zip"] }],
  });
  if (!savePath) return null;
  const summary = await invoke("gstr1_export", {
    dateFrom: dateFrom || undefined,
    dateTo: dateTo || undefined,
    savePath,
  });
  return { ...summary, savePath };
};

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
