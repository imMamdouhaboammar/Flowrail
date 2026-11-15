const { test } = require('node:test');
const assert = require('node:assert/strict');
const { CheckoutController } = require('./dist/modules/checkout/checkout.controller.js');

const request = {
  merchantId: 'merchant', customerId: 'customer', jurisdiction: 'SA',
  items: [{ quantity: 2, unitPriceCents: 5000 }],
};

test('checkout returns subtotal, tax and persisted total', () => {
  const result = new CheckoutController().processCheckout(request);
  assert.equal(result.subtotalCents, 10000);
  assert.equal(result.taxCents, 1500);
  assert.equal(result.grandTotalCents, 11500);
  assert.ok(result.orderId);
});

test('tax-exempt checkout charges no tax', () => {
  const result = new CheckoutController().processCheckout({
    ...request, taxExemptCertificateId: 'fixture-cert',
  });
  assert.equal(result.taxCents, 0);
  assert.equal(result.grandTotalCents, 10000);
});

test('empty cart is rejected at the checkout entrypoint', () => {
  assert.throws(() => new CheckoutController().processCheckout({ ...request, items: [] }), /empty cart/);
});
