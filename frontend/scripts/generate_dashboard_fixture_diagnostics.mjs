import path from 'node:path';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

import { buildDashboardDiagnosticsArtifact } from '../dashboard_diagnostics_artifact.mjs';
import { executiveFixtureSnapshot } from '../tests/dashboard_fixture_data.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const frontendRoot = path.resolve(__dirname, '..');
const defaultOutputPath = path.resolve(
  frontendRoot,
  'test-results',
  'dashboard_fixture_diagnostics.json'
);
const outputPath = process.argv[2]
  ? path.resolve(process.cwd(), process.argv[2])
  : defaultOutputPath;

const artifact = buildDashboardDiagnosticsArtifact(executiveFixtureSnapshot);

await mkdir(path.dirname(outputPath), { recursive: true });
await writeFile(`${outputPath}`, `${JSON.stringify(artifact, null, 2)}\n`, 'utf8');

console.log(outputPath);
