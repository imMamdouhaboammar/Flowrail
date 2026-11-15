export type JurisdictionCode = 'EG' | 'SA' | 'AE' | 'US';

export interface TaxCalculationInput {
  readonly merchantId: string;
  readonly customerId: string;
  readonly jurisdiction: JurisdictionCode;
  readonly subtotalCents: number;
  readonly isTaxExempt: boolean;
}

export interface TaxCalculationResult {
  readonly jurisdiction: JurisdictionCode;
  readonly taxableBaseCents: number;
  readonly taxAmountCents: number;
  readonly effectiveRateBps: number;
  readonly ruleAppliedId: string;
}
