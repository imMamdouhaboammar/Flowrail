const assert = require('node:assert/strict');
const path = require('node:path');
const ts = require('typescript');
const expected = require('./expected_flowrail.json');

const root = __dirname;
const configFile = path.join(root, 'tsconfig.json');
const config = ts.getParsedCommandLineOfConfigFile(configFile, {}, {
  ...ts.sys,
  onUnRecoverableConfigFileDiagnostic: diagnostic => { throw new Error(ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n')); },
});
assert.ok(config);
assert.deepEqual(config.errors, []);
const gate = expected.expected_mutation_gates_sequence[0];
const contract = path.join(root, gate.file_path);
const original = ts.sys.readFile(contract);
const field = '  readonly taxAmountCents: number;';
assert.equal(original.split(field).length, 2, 'Expected exactly one target field');
const external = gate.expected_blast_radius.external_dependents;

function diagnostics(source) {
  const host = ts.createCompilerHost(config.options);
  const readFile = host.readFile.bind(host);
  host.readFile = file => path.resolve(file) === contract ? source : readFile(file);
  return ts.getPreEmitDiagnostics(ts.createProgram(config.fileNames, config.options, host));
}

const baseline = diagnostics(original);
assert.deepEqual(baseline.map(d => d.code), [], 'Baseline must type-check');
const compatible = original.replace(field, field + '\n  readonly municipalTaxCents?: number;');
assert.deepEqual(diagnostics(compatible).map(d => d.code), [], 'Optional addition must stay compatible');

const breaking = original.replace(field, '  readonly nationalVatCents: number;\n  readonly municipalTaxCents: number;');
const broken = diagnostics(breaking);
for (const dependent of external) {
  const matches = broken.filter(d => d.code === 2339 && d.file && d.start !== undefined &&
    path.relative(root, d.file.fileName) === dependent.file_path &&
    d.file.text.slice(d.start, d.start + d.length) === dependent.property_accessed);
  const lines = matches.map(d => d.file.getLineAndCharacterOfPosition(d.start).line + 1).sort((a, b) => a - b);
  assert.deepEqual(lines, dependent.breaking_diagnostic_lines, dependent.file_path);
  for (const d of matches) {
    const { line } = d.file.getLineAndCharacterOfPosition(d.start);
    console.log(dependent.file_path + ':' + (line + 1) + ' TS' + d.code);
  }
}
console.log('Baseline and compatible mutation clean; breaking mutation reached all listed external consumers.');
