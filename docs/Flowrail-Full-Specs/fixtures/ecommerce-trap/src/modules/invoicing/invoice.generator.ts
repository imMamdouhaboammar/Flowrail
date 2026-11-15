import { Logger } from '../../core/logger/pino.logger.js';
import { formatCentsToIso } from '../../shared/utils/currency.js';
import { PersistedOrderRecord } from '../orders/order.types.js';
import { TaxCalculationResult } from '../tax/tax.types.js';

export interface InvoicePdfDocument {
  readonly invoiceNumber: string;
  readonly formattedSubtotal: string;
  readonly formattedTax: string;
  readonly formattedTotal: string;
  readonly taxRuleReference: string;
}

export class InvoiceGenerator {
  public buildComplianceInvoice(order: PersistedOrderRecord): InvoicePdfDocument {
    Logger.info('InvoiceGenerator', 'Generating tax invoice PDF', { orderId: order.orderId });
    const tax: TaxCalculationResult = order.taxBreakdown;

    return {
      invoiceNumber: `INV-${order.orderId}`,
      formattedSubtotal: formatCentsToIso(tax.taxableBaseCents, 'USD'),
      formattedTax: formatCentsToIso(tax.taxAmountCents, 'USD'),
      formattedTotal: formatCentsToIso(order.grandTotalCents, 'USD'),
      taxRuleReference: `${tax.jurisdiction}:${tax.ruleAppliedId}@${tax.effectiveRateBps}bps`
    };
  }
}
