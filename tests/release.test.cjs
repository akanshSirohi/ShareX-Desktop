const { test } = require('node:test');
const assert = require('node:assert/strict');
const { mkdtempSync, mkdirSync, copyFileSync, readFileSync, writeFileSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const path = require('node:path');
const { readVersion, prepare, publish } = require('../.github/scripts/release.cjs');

function fixture({ release = null, ref = null, assets = [] } = {}) {
  const outputs = {}, writes = [];
  const context = { repo: { owner: 'example', repo: 'desktop' }, sha: 'a'.repeat(40) };
  const core = {
    setOutput: (key, value) => { outputs[key] = value; }, info() {},
    summary: { addLink() { return this; }, async write() {} },
  };
  const response = async (data) => {
    if (!data) throw Object.assign(new Error('Not found'), { status: 404 });
    return { data };
  };
  const github = {
    rest: {
      git: {
        getRef: () => response(ref),
        createRef: async (data) => { writes.push(['tag', data]); return { data }; },
      },
      repos: {
        getReleaseByTag: () => response(release),
        getRelease: () => response(release),
        createRelease: async (data) => { writes.push(['create', data]); return { data: { id: 42, ...data } }; },
        updateRelease: async (data) => { writes.push(['publish', data]); return { data }; },
        listReleaseAssets() {},
      },
    },
    paginate: async () => assets,
  };
  return { github, context, core, outputs, writes };
}

test('release versions agree across manifests and stale versions fail', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'sharex-version-'));
  try {
    mkdirSync(path.join(root, 'src-tauri'));
    for (const file of ['package.json', 'package-lock.json', 'src-tauri/Cargo.toml', 'src-tauri/tauri.conf.json']) {
      copyFileSync(file, path.join(root, file));
    }
    const version = readVersion(root);
    assert.match(version, /^\d+\.\d+\.\d+$/);
    const target = path.join(root, 'src-tauri/tauri.conf.json');
    const config = JSON.parse(readFileSync(target, 'utf8'));
    writeFileSync(target, JSON.stringify({ ...config, version: '0.0.0' }));
    assert.throws(() => readVersion(root), /same stable version/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('first release pins the triggering commit and creates a draft', async () => {
  const f = fixture();
  await prepare(f);
  assert.equal(f.outputs.publish, 'true');
  assert.equal(f.outputs.release_id, '42');
  assert.equal(f.writes[0][1].sha, f.context.sha);
  assert.equal(f.writes[1][1].draft, true);
});

test('published versions are skipped without rewriting assets or notes', async () => {
  const f = fixture({ release: { id: 42, draft: false } });
  await prepare(f);
  assert.equal(f.outputs.publish, 'false');
  assert.deepEqual(f.writes, []);
});

test('retry resumes the existing draft without replacing edited release notes', async () => {
  const f = fixture({ release: { id: 42, draft: true }, ref: { object: { type: 'commit', sha: 'a'.repeat(40) } } });
  await prepare(f);
  assert.equal(f.outputs.release_id, '42');
  assert.equal(f.outputs.publish, 'true');
  assert.deepEqual(f.writes, []);
});

test('an existing version tag is never moved to a different commit', async () => {
  const f = fixture({ ref: { object: { type: 'commit', sha: 'b'.repeat(40) } } });
  await assert.rejects(() => prepare(f), /Tags are never moved/);
  assert.deepEqual(f.writes, []);
});

test('API permission failures do not look like missing releases', async () => {
  const f = fixture();
  f.github.rest.repos.getReleaseByTag = async () => { throw Object.assign(new Error('Forbidden'), { status: 403 }); };
  await assert.rejects(() => prepare(f), /Forbidden/);
  assert.deepEqual(f.writes, []);
});

const assets = [
  'ShareX-Desktop_0.1.3_windows_x64-setup.exe',
  'ShareX-Desktop_0.1.3_darwin_aarch64.dmg',
  'ShareX-Desktop_0.1.3_darwin_x64.dmg',
  'ShareX-Desktop_0.1.3_linux_amd64.AppImage',
  'ShareX-Desktop_0.1.3_linux_amd64.deb',
].map((name) => ({ name, size: 100, state: 'uploaded' }));
const env = { RELEASE_ID: '42', RELEASE_TAG: 'v0.1.3' };
const release = { id: 42, tag_name: env.RELEASE_TAG, draft: true, html_url: 'https://example.com/release' };

test('complete packages allow publication', async () => {
  const f = fixture({ release, assets });
  await publish(f, env);
  assert.equal(f.writes[0][0], 'publish');
  assert.equal(f.writes[0][1].draft, false);
});

test('missing or empty packages leave the release as a draft', async () => {
  for (const incomplete of [assets.slice(1), assets.map((a, i) => i ? a : { ...a, size: 0 })]) {
    const f = fixture({ release, assets: incomplete });
    await assert.rejects(() => publish(f, env), /packages are missing/);
    assert.deepEqual(f.writes, []);
  }
});

test('the publication job cannot modify another release', async () => {
  const f = fixture({ release: { ...release, tag_name: 'v0.0.0' }, assets });
  await assert.rejects(() => publish(f, env), /prepared draft/);
  assert.deepEqual(f.writes, []);
});
