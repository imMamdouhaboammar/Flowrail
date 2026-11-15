import { Logger } from '../../core/logger/pino.logger.js';
import { Metrics } from '../../core/telemetry/tracer.js';
import { CheckoutRequestDto, CheckoutResponseDto } from './checkout.dto.js';
import { CheckoutOrchestrator } from './checkout.orchestrator.js';

export class CheckoutController {
  private readonly orchestrator: CheckoutOrchestrator;

  constructor(orchestrator?: CheckoutOrchestrator) {
    this.orchestrator = orchestrator ?? new CheckoutOrchestrator();
  }

  public processCheckout(payload: CheckoutRequestDto): CheckoutResponseDto {
    Logger.info('CheckoutController', 'POST /api/v1/checkout received', {
      merchantId: payload.merchantId
    });
    Metrics.increment('http.post.checkout');

    if (payload.items.length === 0) {
      Logger.warn('CheckoutController', 'Rejected empty cart checkout');
      throw new Error('Cannot process checkout with an empty cart');
    }

    return this.orchestrator.executeCheckout(payload);
  }
}
