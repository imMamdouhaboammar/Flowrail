import { JurisdictionCode, TaxCalculationResult } from '../tax/tax.types.js';

export interface CartLineItem {
  readonly sku: string;
  readonly quantity: number;
  readonly unitPriceCents: number;
}

export interface PersistedOrderRecord {
  readonly orderId: string;
  readonly customerId: string;
  readonly merchantId: string;
  readonly jurisdiction: JurisdictionCode;
  readonly subtotalCents: number;
  readonly totalTaxCents: number;
  readonly grandTotalCents: number;
  readonly taxBreakdown: TaxCalculationResult;
  readonly createdAtIso: string;
}
