import { Logger } from '../../core/logger/pino.logger.js';
import { Metrics, Tracer } from '../../core/telemetry/tracer.js';
import { assertNonNegativeCents } from '../../shared/utils/currency.js';
import { nowIsoUtc } from '../../shared/utils/date.helper.js';
import { OrderRepository } from '../orders/order.repository.js';
import { PersistedOrderRecord } from '../orders/order.types.js';
import { TaxService } from '../tax/tax.service.js';
import type { TaxCalculationResult } from '../tax/tax.types.js';
import { CheckoutRequestDto, CheckoutResponseDto } from './checkout.dto.js';

export class CheckoutOrchestrator {
  private readonly taxService: TaxService;
  private readonly orderRepo: OrderRepository;

  constructor(taxService?: TaxService, orderRepo?: OrderRepository) {
    this.taxService = taxService ?? new TaxService();
    this.orderRepo = orderRepo ?? new OrderRepository();
  }

  public executeCheckout(request: CheckoutRequestDto): CheckoutResponseDto {
    const span = Tracer.startSpan('CheckoutOrchestrator.executeCheckout');
    Logger.info('CheckoutOrchestrator', 'Executing checkout pipeline', {
      customerId: request.customerId
    });

    const subtotalCents = request.items.reduce(
      (acc, item) => acc + item.quantity * item.unitPriceCents,
      0
    );
    assertNonNegativeCents(subtotalCents, 'subtotalCents');

    const taxResult: TaxCalculationResult = this.taxService.calculateVat({
      merchantId: request.merchantId,
      customerId: request.customerId,
      jurisdiction: request.jurisdiction,
      subtotalCents,
      isTaxExempt: Boolean(request.taxExemptCertificateId)
    });

    const grandTotalCents = subtotalCents + taxResult.taxAmountCents;
    const createdAtIso = nowIsoUtc();

    const orderRecord: PersistedOrderRecord = {
      orderId: `ORD_${request.customerId}_${Date.now()}`,
      customerId: request.customerId,
      merchantId: request.merchantId,
      jurisdiction: request.jurisdiction,
      subtotalCents,
      totalTaxCents: taxResult.taxAmountCents,
      grandTotalCents,
      taxBreakdown: taxResult,
      createdAtIso
    };

    const saved = this.orderRepo.saveOrder(orderRecord);
    Metrics.increment('checkout.pipeline.success');
    span.end();

    return {
      orderId: saved.orderId,
      subtotalCents: saved.subtotalCents,
      taxCents: taxResult.taxAmountCents,
      grandTotalCents: saved.grandTotalCents,
      appliedTaxRuleId: taxResult.ruleAppliedId,
      processedAtIso: saved.createdAtIso
    };
  }
}
