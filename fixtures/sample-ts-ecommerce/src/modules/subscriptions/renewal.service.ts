import { TaxCalculationResult } from "../tax/tax.types";

export class RenewalService {
    renew(taxResult: TaxCalculationResult): boolean {
        return taxResult.total > 0;
    }
}
