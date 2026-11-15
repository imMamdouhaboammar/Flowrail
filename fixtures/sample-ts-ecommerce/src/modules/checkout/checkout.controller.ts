import { TaxService } from "../tax/tax.service";
import { TaxCalculationResult } from "../tax/tax.types";

export class CheckoutController {
    processCheckout(amount: number): TaxCalculationResult {
        const taxService = new TaxService();
        return taxService.calculateVat({ amount, currency: "USD" });
    }
}
