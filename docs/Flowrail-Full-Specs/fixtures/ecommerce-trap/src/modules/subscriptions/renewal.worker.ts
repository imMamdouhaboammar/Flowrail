import { Logger } from '../../core/logger/pino.logger.js';
import { Metrics } from '../../core/telemetry/tracer.js';
import { addDaysIso, nowIsoUtc } from '../../shared/utils/date.helper.js';
import { TaxService } from '../tax/tax.service.js';
import { JurisdictionCode, TaxCalculationResult } from '../tax/tax.types.js';

export interface SubscriptionRenewalJob {
  readonly subscriptionId: string;
  readonly merchantId: string;
  readonly customerId: string;
  readonly jurisdiction: JurisdictionCode;
  readonly recurringFeeCents: number;
}

export class RenewalWorker {
  private readonly taxService: TaxService;

  constructor(taxService?: TaxService) {
    this.taxService = taxService ?? new TaxService();
  }

  public processRecurringCharge(job: SubscriptionRenewalJob): {
    chargedCents: number;
    taxAppliedCents: number;
    nextBillingIso: string;
  } {
    Logger.info('RenewalWorker', 'Processing subscription renewal', {
      subscriptionId: job.subscriptionId
    });

    const taxOutcome: TaxCalculationResult = this.taxService.calculateVat({
      merchantId: job.merchantId,
      customerId: job.customerId,
      jurisdiction: job.jurisdiction,
      subtotalCents: job.recurringFeeCents,
      isTaxExempt: false
    });

    Metrics.increment('subscriptions.renewal.billed');

    return {
      chargedCents: job.recurringFeeCents + taxOutcome.taxAmountCents,
      taxAppliedCents: taxOutcome.taxAmountCents,
      nextBillingIso: addDaysIso(nowIsoUtc(), 30)
    };
  }
}
