import { CartLineItem } from '../orders/order.types.js';
import { JurisdictionCode } from '../tax/tax.types.js';

export interface CheckoutRequestDto {
  readonly merchantId: string;
  readonly customerId: string;
  readonly jurisdiction: JurisdictionCode;
  readonly items: readonly CartLineItem[];
  readonly taxExemptCertificateId?: string;
}

export interface CheckoutResponseDto {
  readonly orderId: string;
  readonly subtotalCents: number;
  readonly taxCents: number;
  readonly grandTotalCents: number;
  readonly appliedTaxRuleId: string;
  readonly processedAtIso: string;
}
