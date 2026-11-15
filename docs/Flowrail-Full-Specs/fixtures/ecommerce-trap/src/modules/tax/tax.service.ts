import { Logger } from '../../core/logger/pino.logger.js';
import { Metrics, Tracer } from '../../core/telemetry/tracer.js';
import { assertNonNegativeCents, roundBankersCents } from '../../shared/utils/currency.js';
import { TaxRepository } from './tax.repository.js';
import { TaxCalculationInput, TaxCalculationResult } from './tax.types.js';

export class TaxService {
  private readonly taxRepo: TaxRepository;

  constructor(taxRepo?: TaxRepository) {
    this.taxRepo = taxRepo ?? new TaxRepository();
  }

  public calculateVat(input: TaxCalculationInput): TaxCalculationResult {
    const span = Tracer.startSpan('TaxService.calculateVat');
    Logger.info('TaxService', 'Starting VAT calculation', { merchantId: input.merchantId });
    assertNonNegativeCents(input.subtotalCents, 'subtotalCents');

    if (input.isTaxExempt) {
      Metrics.increment('tax.calculation.exempt');
      span.end();
      return {
        jurisdiction: input.jurisdiction,
        taxableBaseCents: input.subtotalCents,
        taxAmountCents: 0,
        effectiveRateBps: 0,
        ruleAppliedId: 'EXEMPT_ZERO'
      };
    }

    const rule = this.taxRepo.findActiveRuleByJurisdiction(input.jurisdiction);
    const rawTax = (input.subtotalCents * rule.standardRateBps) / 10000;
    const taxAmountCents = roundBankersCents(rawTax);

    Metrics.increment('tax.calculation.completed', { jurisdiction: input.jurisdiction });
    span.end();

    return {
      jurisdiction: input.jurisdiction,
      taxableBaseCents: input.subtotalCents,
      taxAmountCents,
      effectiveRateBps: rule.standardRateBps,
      ruleAppliedId: rule.ruleId
    };
  }
}
