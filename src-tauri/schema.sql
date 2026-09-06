-- Auto-generated clean SQLite schema, converted from designer_db (1).sql
-- Empty schema only -- no legacy data migrated, per user request.
-- NOT NULL columns without an explicit default (which MySQL's relaxed
-- mode silently zero-filled) are given an explicit '' / 0 default here,
-- since SQLite enforces NOT NULL strictly.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS `administrator_account_name` (
  `refid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `acc_name` TEXT NOT NULL DEFAULT '',
  `acc_head` TEXT NOT NULL DEFAULT 'bs',
  `group_head` TEXT NOT NULL DEFAULT 'asset',
  `other_details` TEXT,
  `act_group_head` TEXT NOT NULL DEFAULT '',
  `opening_balance` INTEGER DEFAULT 0,
  `opening_balance_type` TEXT,
  `closing_balance` INTEGER DEFAULT 0,
  `closing_balance_type` TEXT,
  `status` TEXT,
  `backup` TEXT,
  `acc_updatedtime` TEXT,
  `acnt_branch` TEXT
);

CREATE TABLE IF NOT EXISTS `administrator_daybook` (
  `refid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `ad_branchid` TEXT,
  `dayBookDate` TEXT NOT NULL DEFAULT '0000-00-00',
  `debit` TEXT NOT NULL DEFAULT '',
  `credit` TEXT NOT NULL DEFAULT '',
  `dayBookContra` TEXT NOT NULL DEFAULT 'Y',
  `dayBookAmount` REAL NOT NULL DEFAULT 0,
  `description` TEXT NOT NULL DEFAULT '',
  `status` TEXT NOT NULL DEFAULT '',
  `backup` TEXT NOT NULL DEFAULT '',
  `billid` TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS `vm_attendanceentry` (
  `at_id` INTEGER PRIMARY KEY AUTOINCREMENT,
  `at_course` INTEGER NOT NULL DEFAULT 0,
  `at_date` TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS `vm_attendanceitems` (
  `ati_id` INTEGER PRIMARY KEY AUTOINCREMENT,
  `ati_studid` INTEGER NOT NULL DEFAULT 0,
  `ati_present` INTEGER NOT NULL DEFAULT 0,
  `ati_eid` INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `vm_billentry` (
  `be_billid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `be_billnumber` INTEGER,
  `be_customername` TEXT,
  `be_customermobile` TEXT,
  `be_customer_tin_num` TEXT NOT NULL DEFAULT '',
  `be_vehicle_number` TEXT NOT NULL DEFAULT '',
  `be_billdate` TEXT,
  `be_total` REAL,
  `be_gtotal` TEXT NOT NULL DEFAULT '',
  `be_paidamount` REAL,
  `be_paymethod` TEXT,
  `be_note` TEXT,
  `be_updateddate` TEXT,
  `be_updatedby` TEXT,
  `be_isactive` INTEGER DEFAULT 0,
  `be_discount` REAL,
  `be_mode` TEXT NOT NULL DEFAULT '',
  `be_paydate` TEXT NOT NULL DEFAULT '',
  `be_balance` INTEGER NOT NULL DEFAULT 0,
  `be_customerid` INTEGER NOT NULL DEFAULT 0,
  `be_coolie` TEXT NOT NULL DEFAULT '',
  `be_oldbal` TEXT NOT NULL DEFAULT '',
  `be_totvat` REAL NOT NULL DEFAULT 0,
  `be_debitid` TEXT NOT NULL DEFAULT '',
  `be_creditid` TEXT NOT NULL DEFAULT '',
  `be_statecode` TEXT,
  `be_mod` INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `vm_billitems` (
  `bi_billitemid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `bi_billid` INTEGER,
  `bi_productid` INTEGER,
  `bi_price` REAL,
  `bi_quantity` REAL,
  `bi_taxamount` REAL,
  `bi_discount` TEXT NOT NULL DEFAULT '',
  `bi_total` REAL,
  `bi_updatedon` TEXT,
  `bi_isactive` INTEGER DEFAULT 0,
  `bi_vatamount` REAL,
  `bi_vatper` REAL,
  `be_coolie` TEXT NOT NULL DEFAULT '',
  `bi_sgst` REAL,
  `bi_sgst_amt` REAL,
  `bi_cgst` REAL,
  `bi_cgst_amt` REAL,
  `bi_igst` REAL,
  `bi_igst_amt` REAL,
  `bi_billdate` TEXT,
  `bi_purprice` REAL,
  `bi_disc` REAL NOT NULL DEFAULT 0,
  `bi_prenet` REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `vm_catogory` (
  `ca_categoryid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `ca_categoryname` TEXT,
  `ca_vat` TEXT,
  `ca_isactive` INTEGER DEFAULT 0,
  `ca_updatedtime` TEXT,
  `ca_note` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_course` (
  `co_courseid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `co_coursename` TEXT,
  `co_fee` TEXT,
  `co_isactive` INTEGER DEFAULT 0,
  `co_updatedtime` TEXT,
  `co_details` TEXT,
  `co_duration` TEXT,
  `co_schedule` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_customer` (
  `cs_customerid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER,
  `cs_billnumber` INTEGER,
  `cs_customername` TEXT,
  `cs_customerphone` TEXT,
  `cs_address` TEXT,
  `cs_email` TEXT,
  `cs_tin_number` TEXT,
  `cs_payment` REAL,
  `cs_discount` REAL,
  `cs_paid` REAL,
  `cs_balance` REAL,
  `cs_paymethod` TEXT,
  `cs_note` TEXT,
  `cs_updateddate` TEXT,
  `cs_updatedby` TEXT,
  `cs_isactive` INTEGER DEFAULT 0,
  `cs_acntid` TEXT NOT NULL DEFAULT '',
  `cs_statecode` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_employee` (
  `emp_id` INTEGER PRIMARY KEY AUTOINCREMENT,
  `emp_name` TEXT NOT NULL DEFAULT '',
  `emp_dept` INTEGER NOT NULL DEFAULT 0,
  `emp_bp` TEXT NOT NULL DEFAULT '',
  `emp_phno` INTEGER NOT NULL DEFAULT 0,
  `emp_add` TEXT NOT NULL DEFAULT '',
  `emp_isactive` INTEGER NOT NULL DEFAULT 0,
  `user_id` INTEGER NOT NULL DEFAULT 0,
  `emp_acntid` INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `vm_estimation` (
  `be_billid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `be_billnumber` INTEGER,
  `be_customername` TEXT,
  `be_customermobile` TEXT,
  `be_customer_tin_num` TEXT NOT NULL DEFAULT '',
  `be_vehicle_number` TEXT NOT NULL DEFAULT '',
  `be_billdate` TEXT,
  `be_total` REAL,
  `be_gtotal` TEXT NOT NULL DEFAULT '',
  `be_paidamount` REAL,
  `be_paymethod` TEXT,
  `be_note` TEXT,
  `be_updateddate` TEXT,
  `be_updatedby` TEXT,
  `be_isactive` INTEGER DEFAULT 0,
  `be_discount` REAL,
  `be_mode` TEXT NOT NULL DEFAULT '',
  `be_paydate` TEXT NOT NULL DEFAULT '',
  `be_balance` INTEGER NOT NULL DEFAULT 0,
  `be_customerid` INTEGER NOT NULL DEFAULT 0,
  `be_coolie` TEXT NOT NULL DEFAULT '',
  `be_oldbal` TEXT NOT NULL DEFAULT '',
  `be_totvat` REAL NOT NULL DEFAULT 0,
  `be_debitid` TEXT NOT NULL DEFAULT '',
  `be_creditid` TEXT NOT NULL DEFAULT '',
  `be_statecode` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_estimationitems` (
  `bi_billitemid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `bi_billid` INTEGER,
  `bi_productid` INTEGER,
  `bi_price` REAL,
  `bi_quantity` REAL,
  `bi_discount` TEXT NOT NULL DEFAULT '',
  `bi_total` REAL,
  `bi_updatedon` TEXT,
  `bi_isactive` INTEGER DEFAULT 0,
  `bi_vatamount` REAL,
  `bi_vatper` REAL,
  `be_coolie` TEXT NOT NULL DEFAULT '',
  `bi_sgst` REAL,
  `bi_sgst_amt` REAL,
  `bi_cgst` REAL,
  `bi_cgst_amt` REAL,
  `bi_igst` REAL,
  `bi_igst_amt` REAL
);

CREATE TABLE IF NOT EXISTS `vm_garmententry` (
  `ga_productid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `ga_productcode` TEXT,
  `ga_productname` TEXT,
  `ga_hsn` TEXT,
  `ga_purchaseprice` REAL,
  `ga_saleprice` REAL,
  `ga_retail` TEXT NOT NULL DEFAULT '',
  `ga_wholesale` TEXT NOT NULL DEFAULT '',
  `ga_description` TEXT,
  `ga_stock` REAL,
  `ga_isactive` INTEGER DEFAULT 0,
  `ga_updateddate` TEXT,
  `ga_type` INTEGER,
  `ga_unit` TEXT NOT NULL DEFAULT '',
  `ga_pecentage` TEXT,
  `ga_colorcode` TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS `vm_garmentitems` (
  `gi_id` INTEGER PRIMARY KEY AUTOINCREMENT,
  `gi_batchno` TEXT NOT NULL DEFAULT '',
  `gi_date` TEXT NOT NULL DEFAULT '',
  `gi_productid` INTEGER NOT NULL DEFAULT 0,
  `gi_purchaseprice` TEXT NOT NULL DEFAULT '',
  `gi_qty` INTEGER NOT NULL DEFAULT 0,
  `gi_billid` INTEGER NOT NULL DEFAULT 0,
  `user_id` INTEGER NOT NULL DEFAULT 0,
  `gi_isactive` INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `vm_model` (
  `md_modelid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `md_modelname` TEXT,
  `md_isactive` INTEGER DEFAULT 0,
  `md_updatedtime` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_parts` (
  `pr_productid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `pr_productcode` TEXT NOT NULL DEFAULT '',
  `pr_productname` TEXT,
  `pr_hsn` TEXT,
  `pr_purchaseprice` REAL,
  `pr_saleprice` REAL,
  `pr_retail` TEXT NOT NULL DEFAULT '',
  `pr_wholesale` TEXT NOT NULL DEFAULT '',
  `pr_description` TEXT,
  `pr_stock` REAL,
  `pr_isactive` INTEGER DEFAULT 0,
  `pr_updateddate` TEXT,
  `pr_type` INTEGER,
  `pr_unit` TEXT NOT NULL DEFAULT '',
  `pr_pecentage` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_payment` (
  `pa_paymentid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER,
  `pa_customerid` INTEGER,
  `pa_billid` INTEGER,
  `pa_balance` REAL,
  `pa_newpayment` REAL,
  `pa_newbalance` REAL,
  `pa_isactive` INTEGER DEFAULT 0,
  `pa_updatedon` TEXT,
  `pa_mode` TEXT NOT NULL DEFAULT '',
  `pa_transactionid` TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS `vm_processentry` (
  `be_billid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `be_billnumber` INTEGER,
  `be_customername` TEXT,
  `be_customermobile` TEXT,
  `be_customer_tin_num` TEXT NOT NULL DEFAULT '',
  `be_vehicle_number` TEXT NOT NULL DEFAULT '',
  `be_billdate` TEXT,
  `be_total` REAL,
  `be_gtotal` TEXT NOT NULL DEFAULT '',
  `be_paidamount` REAL,
  `be_paymethod` TEXT,
  `be_note` TEXT,
  `be_updateddate` TEXT,
  `be_updatedby` TEXT,
  `be_isactive` INTEGER DEFAULT 0,
  `be_discount` REAL,
  `be_mode` TEXT NOT NULL DEFAULT '',
  `be_paydate` TEXT NOT NULL DEFAULT '',
  `be_balance` INTEGER NOT NULL DEFAULT 0,
  `be_customerid` INTEGER NOT NULL DEFAULT 0,
  `be_coolie` TEXT NOT NULL DEFAULT '',
  `be_oldbal` TEXT NOT NULL DEFAULT '',
  `be_totvat` REAL NOT NULL DEFAULT 0,
  `be_debitid` TEXT NOT NULL DEFAULT '',
  `be_creditid` TEXT NOT NULL DEFAULT '',
  `be_statecode` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_processitems` (
  `bi_billitemid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `bi_billid` INTEGER,
  `bi_productid` INTEGER,
  `bi_batchid` INTEGER NOT NULL DEFAULT 0,
  `bi_price` REAL,
  `bi_quantity` REAL,
  `bi_discount` TEXT NOT NULL DEFAULT '',
  `bi_total` REAL,
  `bi_updatedon` TEXT,
  `bi_isactive` INTEGER DEFAULT 0,
  `bi_vatamount` REAL,
  `bi_vatper` REAL,
  `be_coolie` TEXT NOT NULL DEFAULT '',
  `bi_sgst` REAL,
  `bi_sgst_amt` REAL,
  `bi_cgst` REAL,
  `bi_cgst_amt` REAL,
  `bi_igst` REAL,
  `bi_igst_amt` REAL
);

CREATE TABLE IF NOT EXISTS `vm_products` (
  `pr_productid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `pr_productcode` TEXT,
  `pr_productname` TEXT,
  `pr_hsn` TEXT,
  `pr_purchaseprice` REAL,
  `pr_saleprice` REAL,
  `pr_retail` TEXT NOT NULL DEFAULT '',
  `pr_wholesale` TEXT NOT NULL DEFAULT '',
  `pr_description` TEXT,
  `pr_stock` REAL,
  `pr_isactive` INTEGER DEFAULT 0,
  `pr_updateddate` TEXT,
  `pr_type` INTEGER,
  `pr_unit` TEXT NOT NULL DEFAULT '',
  `pr_pecentage` TEXT,
  `pr_model` TEXT NOT NULL DEFAULT '',
  `pr_size` TEXT NOT NULL DEFAULT '',
  `pr_barcode` TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS `vm_purentry` (
  `pe_billid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER,
  `pe_billnumber` INTEGER,
  `pe_customername` TEXT,
  `pe_customermobile` INTEGER,
  `pe_billdate` TEXT,
  `pe_total` REAL,
  `pe_gtotal` TEXT NOT NULL DEFAULT '',
  `pe_oldbal` TEXT NOT NULL DEFAULT '',
  `pe_paidamount` REAL,
  `pe_paymethod` TEXT,
  `pe_note` TEXT,
  `pe_updateddate` TEXT,
  `pe_updatedby` TEXT,
  `pe_isactive` INTEGER DEFAULT 0,
  `pe_discount` REAL,
  `pe_mode` TEXT,
  `pe_paydate` TEXT,
  `pe_unitprice` INTEGER NOT NULL DEFAULT 0,
  `pe_balance` REAL NOT NULL DEFAULT 0,
  `pe_supplierid` TEXT NOT NULL DEFAULT '',
  `pe_vehicle_number` TEXT NOT NULL DEFAULT '',
  `pe_invoice_number` TEXT NOT NULL DEFAULT '',
  `pe_invoice_date` TEXT NOT NULL DEFAULT '',
  `pe_statecode` TEXT,
  `pe_debitid` TEXT NOT NULL DEFAULT '',
  `pe_creditid` INTEGER NOT NULL DEFAULT 0,
  `pe_purmode` INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `vm_puritems` (
  `pi_billitemid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER,
  `pi_billid` INTEGER,
  `pi_productid` INTEGER,
  `pi_price` REAL,
  `pi_quantity` REAL,
  `pi_total` REAL,
  `pi_updatedon` TEXT,
  `pi_isactive` INTEGER DEFAULT 0,
  `pi_vatamount` REAL,
  `pi_vatper` REAL,
  `pi_unitprice` INTEGER NOT NULL DEFAULT 0,
  `pi_sgst` REAL,
  `pi_sgstamt` REAL,
  `pi_cgst` REAL,
  `pi_cgstamt` REAL,
  `pi_igst` REAL,
  `pi_igstamt` REAL,
  `pi_discount` TEXT,
  `pi_taxamount` TEXT,
  `pi_prrate` TEXT,
  `pi_hsn` TEXT,
  `pi_billdate` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_purorder` (
  `pe_billid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER,
  `pe_billnumber` INTEGER,
  `pe_customername` TEXT,
  `pe_customermobile` INTEGER,
  `pe_billdate` TEXT,
  `pe_total` REAL,
  `pe_gtotal` TEXT NOT NULL DEFAULT '',
  `pe_oldbal` TEXT NOT NULL DEFAULT '',
  `pe_paidamount` REAL,
  `pe_paymethod` TEXT,
  `pe_note` TEXT,
  `pe_updateddate` TEXT,
  `pe_updatedby` TEXT,
  `pe_isactive` INTEGER DEFAULT 0,
  `pe_discount` REAL,
  `pe_mode` TEXT,
  `pe_paydate` TEXT,
  `pe_unitprice` INTEGER NOT NULL DEFAULT 0,
  `pe_balance` REAL NOT NULL DEFAULT 0,
  `pe_supplierid` TEXT NOT NULL DEFAULT '',
  `pe_vehicle_number` TEXT NOT NULL DEFAULT '',
  `pe_invoice_number` TEXT NOT NULL DEFAULT '',
  `pe_invoice_date` TEXT NOT NULL DEFAULT '',
  `pe_statecode` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_purorderitems` (
  `pi_billitemid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER,
  `pi_billid` INTEGER,
  `pi_productid` INTEGER,
  `pi_price` REAL,
  `pi_quantity` REAL,
  `pi_total` REAL,
  `pi_updatedon` TEXT,
  `pi_isactive` INTEGER DEFAULT 0,
  `pi_vatamount` REAL,
  `pi_vatper` REAL,
  `pi_unitprice` INTEGER NOT NULL DEFAULT 0,
  `pi_sgst` REAL,
  `pi_sgstamt` REAL,
  `pi_cgst` REAL,
  `pi_cgstamt` REAL,
  `pi_igst` REAL,
  `pi_igstamt` REAL
);

CREATE TABLE IF NOT EXISTS `vm_purreturnentry` (
  `pre_billid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `pre_billnumber` INTEGER,
  `pre_customername` TEXT,
  `pre_customermobile` TEXT,
  `pre_customer_tin_num` TEXT NOT NULL DEFAULT '',
  `pre_vehicle_number` TEXT NOT NULL DEFAULT '',
  `pre_billdate` TEXT,
  `pre_total` REAL,
  `pre_gtotal` REAL NOT NULL DEFAULT 0,
  `pre_paidamount` REAL,
  `pre_paymethod` TEXT,
  `pre_note` TEXT,
  `pre_updateddate` TEXT,
  `pre_updatedby` TEXT,
  `pre_isactive` INTEGER DEFAULT 0,
  `pre_discount` REAL,
  `pre_mode` TEXT NOT NULL DEFAULT '',
  `pre_paydate` TEXT NOT NULL DEFAULT '',
  `pre_balance` INTEGER NOT NULL DEFAULT 0,
  `pre_customerid` INTEGER NOT NULL DEFAULT 0,
  `pre_coolie` TEXT NOT NULL DEFAULT '',
  `pre_oldbal` TEXT NOT NULL DEFAULT '',
  `pre_totvat` REAL,
  `pre_invoice_number` TEXT NOT NULL DEFAULT '',
  `pre_invoice_date` TEXT NOT NULL DEFAULT '',
  `pre_rebill` TEXT,
  `pre_statecode` TEXT,
  `pre_debitid` INTEGER
);

CREATE TABLE IF NOT EXISTS `vm_purreturnitem` (
  `pri_billitemid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `pri_billid` INTEGER NOT NULL DEFAULT 0,
  `pri_returnbillid` INTEGER NOT NULL DEFAULT 0,
  `pri_productid` INTEGER NOT NULL DEFAULT 0,
  `pri_price` TEXT NOT NULL DEFAULT '',
  `pri_quantity` REAL NOT NULL DEFAULT 0,
  `pri_total` REAL NOT NULL DEFAULT 0,
  `pri_updatedon` TEXT NOT NULL DEFAULT '',
  `pri_isactive` INTEGER NOT NULL DEFAULT 0,
  `pri_vatamount` REAL NOT NULL DEFAULT 0,
  `pri_vatper` REAL NOT NULL DEFAULT 0,
  `pri_coolie` REAL NOT NULL DEFAULT 0,
  `pri_sgst` REAL,
  `pri_sgstamt` REAL,
  `pri_cgst` REAL,
  `pr_cgstamt` REAL,
  `pri_igst` REAL,
  `pri_igstamt` REAL,
  `pri_billdate` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_repairentry` (
  `be_billid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `be_billnumber` INTEGER,
  `be_customername` TEXT,
  `be_customermobile` TEXT,
  `be_customer_tin_num` TEXT NOT NULL DEFAULT '',
  `be_vehicle_number` TEXT NOT NULL DEFAULT '',
  `be_billdate` TEXT,
  `be_total` REAL,
  `be_gtotal` TEXT NOT NULL DEFAULT '',
  `be_paidamount` REAL,
  `be_paymethod` TEXT,
  `be_note` TEXT,
  `be_updateddate` TEXT,
  `be_updatedby` TEXT,
  `be_isactive` INTEGER DEFAULT 0,
  `be_discount` REAL,
  `be_mode` TEXT NOT NULL DEFAULT '',
  `be_paydate` TEXT NOT NULL DEFAULT '',
  `be_balance` INTEGER NOT NULL DEFAULT 0,
  `be_customerid` INTEGER NOT NULL DEFAULT 0,
  `be_coolie` TEXT NOT NULL DEFAULT '',
  `be_oldbal` TEXT NOT NULL DEFAULT '',
  `be_totvat` REAL NOT NULL DEFAULT 0,
  `be_debitid` TEXT NOT NULL DEFAULT '',
  `be_creditid` TEXT NOT NULL DEFAULT '',
  `be_statecode` TEXT,
  `be_model` TEXT NOT NULL DEFAULT '',
  `be_workdone` TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS `vm_repairitem` (
  `bi_billitemid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` TEXT NOT NULL DEFAULT '',
  `bi_billid` INTEGER,
  `bi_productid` INTEGER,
  `bi_price` REAL,
  `bi_quantity` REAL,
  `bi_taxamount` REAL,
  `bi_discount` TEXT NOT NULL DEFAULT '',
  `bi_total` REAL,
  `bi_updatedon` TEXT,
  `bi_isactive` INTEGER DEFAULT 0,
  `bi_vatamount` REAL,
  `bi_vatper` REAL,
  `be_coolie` TEXT NOT NULL DEFAULT '',
  `bi_sgst` REAL,
  `bi_sgst_amt` REAL,
  `bi_cgst` REAL,
  `bi_cgst_amt` REAL,
  `bi_igst` REAL,
  `bi_igst_amt` REAL,
  `bi_billdate` TEXT,
  `bi_purprice` REAL,
  `bi_disc` REAL NOT NULL DEFAULT 0,
  `bi_prenet` REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `vm_salreturnentry` (
  `sre_billid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER,
  `sre_billnumber` INTEGER,
  `sre_customername` TEXT,
  `sre_customeraddress` TEXT,
  `sre_customermobile` TEXT,
  `sre_customer_tin_num` TEXT NOT NULL DEFAULT '',
  `sre_billdate` TEXT,
  `sre_total` REAL,
  `sre_gtotal` TEXT NOT NULL DEFAULT '',
  `sre_oldbal` TEXT NOT NULL DEFAULT '',
  `sre_paidamount` REAL,
  `sre_paymethod` TEXT,
  `sre_note` TEXT,
  `sre_updateddate` TEXT,
  `sre_updatedby` TEXT,
  `sre_isactive` INTEGER DEFAULT 0,
  `sre_discount` REAL,
  `sre_mode` TEXT,
  `sre_paydate` TEXT,
  `sre_unitprice` INTEGER NOT NULL DEFAULT 0,
  `sre_balance` REAL NOT NULL DEFAULT 0,
  `sre_supplierid` INTEGER,
  `sre_vehicle_number` TEXT NOT NULL DEFAULT '',
  `sre_rebill` TEXT,
  `sre_statecode` TEXT,
  `sre_debitid` INTEGER,
  `sre_creditid` INTEGER
);

CREATE TABLE IF NOT EXISTS `vm_salreturnitem` (
  `sri_billitemid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER,
  `sri_billid` INTEGER NOT NULL DEFAULT 0,
  `sri_returnbillid` INTEGER NOT NULL DEFAULT 0,
  `sri_productid` INTEGER NOT NULL DEFAULT 0,
  `sri_price` REAL NOT NULL DEFAULT 0,
  `sri_quantity` REAL NOT NULL DEFAULT 0,
  `sri_total` REAL NOT NULL DEFAULT 0,
  `sri_updatedon` TEXT NOT NULL DEFAULT '',
  `sri_isactive` INTEGER NOT NULL DEFAULT 0,
  `sri_vatamount` REAL NOT NULL DEFAULT 0,
  `sri_vatper` REAL NOT NULL DEFAULT 0,
  `sri_unitprice` REAL NOT NULL DEFAULT 0,
  `sri_sgst` REAL,
  `sri_sgstamt` REAL,
  `sri_cgst` REAL,
  `sri_cgstamt` REAL,
  `sri_igst` REAL,
  `sri_igstamt` REAL,
  `sri_billdate` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_shopprofile` (
  `sp_shopid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `sp_shopname` TEXT,
  `sp_shopaddress` TEXT,
  `sp_phone` TEXT,
  `sp_mobile` TEXT,
  `sp_email` TEXT,
  `sp_logo` TEXT,
  `sp_username` TEXT,
  `sp_password` TEXT,
  `sp_isactive` INTEGER DEFAULT 0,
  `sp_vatreadymades` REAL,
  `sp_vatmillgoods` REAL,
  `sp_tin` TEXT NOT NULL DEFAULT '',
  `sp_cst` TEXT NOT NULL DEFAULT '',
  `sp_adddate` TEXT NOT NULL DEFAULT '',
  `sp_acnttype` INTEGER NOT NULL DEFAULT 0,
  `sp_trlprd` INTEGER NOT NULL DEFAULT 0,
  `sp_barcode` TEXT NOT NULL DEFAULT '',
  `sp_stcode` TEXT,
  `sp_accno` TEXT NOT NULL DEFAULT '',
  `sp_ifsc` TEXT NOT NULL DEFAULT '',
  `sp_bank` TEXT NOT NULL DEFAULT '',
  `sp_branch` TEXT NOT NULL DEFAULT '',
  `sp_shop_type` TEXT DEFAULT 'designer_apparels'
);

CREATE TABLE IF NOT EXISTS `vm_staffattendanceentry` (
  `at_id` INTEGER PRIMARY KEY AUTOINCREMENT,
  `at_date` TEXT NOT NULL DEFAULT '',
  `at_department` TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS `vm_staffattendanceitems` (
  `ati_id` INTEGER PRIMARY KEY AUTOINCREMENT,
  `ati_workerid` INTEGER NOT NULL DEFAULT 0,
  `ati_present` INTEGER NOT NULL DEFAULT 0,
  `ati_eid` INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS `vm_student` (
  `st_stdid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `user_id` INTEGER,
  `st_billnumber` INTEGER,
  `st_stdname` TEXT,
  `st_stdsex` TEXT,
  `st_stdage` TEXT,
  `st_stdphone` TEXT,
  `st_stdaddress` TEXT,
  `st_stdemail` TEXT,
  `st_tin_number` TEXT,
  `st_stdcourse` TEXT,
  `st_stdduration` TEXT,
  `st_stdschedule` TEXT,
  `st_stdfee` REAL,
  `st_paid` REAL,
  `st_balance` REAL,
  `st_note` TEXT,
  `st_updateddate` TEXT,
  `st_isactive` INTEGER DEFAULT 0,
  `st_acntid` TEXT NOT NULL DEFAULT '',
  `st_statecode` TEXT,
  `st_gaurd` TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS `vm_supplier` (
  `rs_supplierid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `rs_company_name` TEXT NOT NULL DEFAULT '',
  `rs_name` TEXT,
  `rs_phone` TEXT,
  `rs_mobile` TEXT,
  `rs_address` TEXT,
  `rs_email` TEXT NOT NULL DEFAULT '',
  `rs_balance` TEXT NOT NULL DEFAULT '',
  `rs_isactive` INTEGER DEFAULT 0,
  `rs_tinnum` TEXT NOT NULL DEFAULT '',
  `user_id` TEXT NOT NULL DEFAULT '',
  `rs_acntid` TEXT NOT NULL DEFAULT '',
  `rs_statecode` TEXT
);

CREATE TABLE IF NOT EXISTS `vm_transaction` (
  `tr_id` INTEGER PRIMARY KEY AUTOINCREMENT,
  `tr_billid` INTEGER NOT NULL DEFAULT 0,
  `tr_particulars` TEXT NOT NULL DEFAULT '',
  `tr_openingbalance` REAL NOT NULL DEFAULT 0,
  `tr_transactionamount` REAL NOT NULL DEFAULT 0,
  `tr_closingbalance` REAL NOT NULL DEFAULT 0,
  `tr_date` TEXT NOT NULL DEFAULT '',
  `tr_transactiontype` TEXT NOT NULL DEFAULT '',
  `tr_isactive` INTEGER NOT NULL DEFAULT 0,
  `tr_updateddate` TEXT NOT NULL DEFAULT '',
  `user_id` INTEGER NOT NULL DEFAULT 0,
  `tr_mode` INTEGER NOT NULL DEFAULT 0,
  `pay_type` TEXT NOT NULL DEFAULT '',
  `cus_id` TEXT NOT NULL DEFAULT '',
  `tr_name` TEXT NOT NULL DEFAULT '',
  `tr_accid` INTEGER
);
