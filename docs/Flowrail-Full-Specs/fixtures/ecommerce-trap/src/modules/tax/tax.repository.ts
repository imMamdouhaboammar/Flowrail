import { Logger } from '../../core/logger/pino.logger.js';
import { Metrics } from '../../core/telemetry/tracer.js';
import { JurisdictionCode } from './tax.types.js';

export interface StoredTaxRule {
  readonly ruleId: string;
  readonly jurisdiction: JurisdictionCode;
  readonly standardRateBps: number;
  readonly active: boolean;
}

export class TaxRepository {
  public findActiveRuleByJurisdiction(jurisdiction: JurisdictionCode): StoredTaxRule {
    Logger.info('TaxRepository', 'Fetching active tax rule', { jurisdiction });
    Metrics.increment('db.tax_rule.lookup', { jurisdiction });

    const rateMap: Record<JurisdictionCode, number> = {
      EG: 1400,
      SA: 1500,
      AE: 500,
      US: 825
    };

    return {
      ruleId: `RULE_${jurisdiction}_STD_2026`,
      jurisdiction,
      standardRateBps: rateMap[jurisdiction],
      active: true
    };
  }
}
