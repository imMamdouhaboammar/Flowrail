import { TaxCalculationResult } from "../tax/tax.types";

export class InvoiceGenerator {
    generate(taxResult: TaxCalculationResult): string {
        return `Invoice total: ${taxResult.total}, vat: ${taxResult.vat}`;
    }
}
