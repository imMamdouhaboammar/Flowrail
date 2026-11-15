import { Logger } from '../../core/logger/pino.logger.js';
import { TaxCalculationResult } from '../tax/tax.types.js';

export class VatAuditJob {
  public summarizeQuarterlyLiability(records: readonly TaxCalculationResult[]): {
    totalTaxableCents: number;
    totalLiabilityCents: number;
  } {
    Logger.info('VatAuditJob', 'Summarizing quarterly VAT liability', {
      recordCount: records.length
    });

    let totalTaxableCents = 0;
    let totalLiabilityCents = 0;

    for (const rec of records) {
      totalTaxableCents += rec.taxableBaseCents;
      totalLiabilityCents += rec.taxAmountCents;
    }

    return {
      totalTaxableCents,
      totalLiabilityCents
    };
  }
}
