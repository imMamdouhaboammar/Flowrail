import { Logger } from '../../core/logger/pino.logger.js';
import { Metrics } from '../../core/telemetry/tracer.js';
import { formatCentsToIso } from '../../shared/utils/currency.js';
import { PersistedOrderRecord } from './order.types.js';

export class OrderRepository {
  private readonly memoryStore: Map<string, PersistedOrderRecord> = new Map();

  public saveOrder(record: PersistedOrderRecord): PersistedOrderRecord {
    Logger.info('OrderRepository', 'Persisting order to database', {
      orderId: record.orderId,
      formattedTotal: formatCentsToIso(record.grandTotalCents, 'USD')
    });
    this.memoryStore.set(record.orderId, record);
    Metrics.increment('db.orders.insert');
    return record;
  }
}
