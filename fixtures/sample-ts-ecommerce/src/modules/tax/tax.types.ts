export interface TaxCalculationInput {
    amount: number;
    currency: string;
}

export interface TaxCalculationResult {
    vat: number;
    total: number;
}
