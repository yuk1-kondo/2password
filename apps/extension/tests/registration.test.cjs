const { test } = require('node:test');
const assert = require('node:assert/strict');
const { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const root = path.resolve(__dirname, '../../..');

test('extension manifest references existing files', () => {
  const directory = path.join(root, 'apps/extension');
  const manifest = JSON.parse(readFileSync(path.join(directory, 'manifest.json')));
  const files = [manifest.background.service_worker, manifest.action.default_popup,
    ...manifest.content_scripts.flatMap(script => script.js),
    ...Object.values(manifest.icons || {})];
  for (const file of files) assert.ok(existsSync(path.join(directory, file)), file);
});

test('installer rejects wildcards and registers only the supplied extension', () => {
  const directory = mkdtempSync(path.join(tmpdir(), '2password-registration-'));
  try {
    // Isolated project and HOME: never modify the real browser registrations.
    const scripts = path.join(directory, 'scripts');
    const bin = path.join(directory, 'bin');
    mkdirSync(scripts);
    mkdirSync(bin);
    mkdirSync(path.join(directory, 'target/release'), { recursive: true });
    writeFileSync(path.join(scripts, 'install-native-host.sh'),
      readFileSync(path.join(root, 'scripts/install-native-host.sh')));
    writeFileSync(path.join(bin, 'cargo'), '#!/bin/sh\nexit 0\n', { mode: 0o755 });
    writeFileSync(path.join(directory, 'target/release/2pw-nmh'), '#!/bin/sh\n', { mode: 0o755 });
    const env = { ...process.env, HOME: directory, PATH: `${bin}:${process.env.PATH}` };
    const run = (...args) => spawnSync('/bin/zsh', [path.join(scripts, 'install-native-host.sh'), ...args], { env, encoding: 'utf8' });
    for (const id of ['', '*', 'z'.repeat(32), 'a'.repeat(31)]) {
      assert.equal(run(id).status, 1);
      assert.equal(existsSync(path.join(directory, 'Library')), false);
    }
    const id = 'abcdefghijklmnopabcdefghijklmnop';
    const result = run(id);
    assert.equal(result.status, 0, result.stderr);
    for (const browser of ['Google/Chrome', 'Chromium', 'Microsoft Edge']) {
      const manifest = JSON.parse(readFileSync(path.join(directory,
        `Library/Application Support/${browser}/NativeMessagingHosts/dev.twopassword.native.json`)));
      assert.deepEqual(manifest.allowed_origins, [`chrome-extension://${id}/`]);
      assert.equal(manifest.path, path.join(directory, 'target/release/2pw-nmh'));
    }
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
