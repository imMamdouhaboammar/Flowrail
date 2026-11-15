import { TaxCalculationInput, TaxCalculationResult } from "./tax.types";

export class TaxService {
    calculateVat(input: TaxCalculationInput): TaxCalculationResult {
        const vat = input.amount * 0.2;
        return {
            vat,
            total: input.amount + vat,
        };
    }
}
