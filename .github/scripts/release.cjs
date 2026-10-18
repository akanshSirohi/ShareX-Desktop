const { readFileSync } = require('node:fs');
const path = require('node:path');

function readVersion(root = process.cwd()) {
  const json = (file) => JSON.parse(readFileSync(path.join(root, file), 'utf8'));
  const version = json('package.json').version;
  const lock = json('package-lock.json');
  const cargo = readFileSync(path.join(root, 'src-tauri/Cargo.toml'), 'utf8');
  const packageSection = cargo.split(/^\[package\]\s*$/m)[1]?.split(/^\[/m)[0];
  const cargoVersion = packageSection?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (!/^\d+\.\d+\.\d+$/.test(version) || [
    json('src-tauri/tauri.conf.json').version, cargoVersion,
    lock.version, lock.packages[''].version,
  ].some((value) => value !== version)) {
    throw new Error('Use the same stable version in package.json, package-lock.json, Cargo.toml, and tauri.conf.json.');
  }
  return version;
}

async function optional(request) {
  try { return (await request()).data; }
  catch (error) { if (error.status === 404) return null; throw error; }
}

async function prepare({ github, context, core }, root) {
  const version = readVersion(root);
  const tag = `v${version}`;
  const repo = context.repo;
  core.setOutput('publish', 'false');
  core.setOutput('tag', tag);
  let release = await optional(() => github.rest.repos.getReleaseByTag({ ...repo, tag }));
  if (release && !release.draft) {
    core.info(`${tag} is already published. Bump the application version for another release.`);
    return;
  }
  const ref = await optional(() => github.rest.git.getRef({ ...repo, ref: `tags/${tag}` }));
  if (ref && (ref.object.type !== 'commit' || ref.object.sha !== context.sha)) {
    throw new Error(`${tag} belongs to another commit. Re-run the original workflow, or bump the application version. Tags are never moved.`);
  }
  if (!ref) await github.rest.git.createRef({ ...repo, ref: `refs/tags/${tag}`, sha: context.sha });
  if (!release) {
    release = (await github.rest.repos.createRelease({
      ...repo, tag_name: tag, target_commitish: context.sha,
      name: `ShareX Desktop ${tag}`, draft: true, prerelease: false,
      body: [
        `ShareX Desktop ${tag}`,
        '',
        'Download the installer for your system from Assets:',
        '- Windows x64: setup.exe',
        '- macOS: Apple Silicon (aarch64) or Intel (x64) DMG',
        '- Linux x64: AppImage or Debian package',
        '',
        'macOS and Linux are experimental. Packages are not signed or notarized.',
        'Use single-folder sharing on macOS and Linux; encrypted-volume detection is Windows-specific.',
        '',
        `Build instructions and limitations: https://github.com/${repo.owner}/${repo.repo}/tree/${tag}/docs`,
      ].join('\n'),
    })).data;
  }
  core.setOutput('release_id', String(release.id));
  core.setOutput('publish', 'true');
}

async function publish({ github, context, core }, env = process.env) {
  const repo = context.repo;
  const releaseId = Number(env.RELEASE_ID);
  const { data: release } = await github.rest.repos.getRelease({ ...repo, release_id: releaseId });
  if (release.tag_name !== env.RELEASE_TAG || !release.draft) throw new Error('Expected the prepared draft release.');
  const assets = await github.paginate(github.rest.repos.listReleaseAssets, { ...repo, release_id: releaseId, per_page: 100 });
  const names = assets.filter((a) => a.size > 0 && a.state === 'uploaded').map((a) => a.name);
  const required = [
    /_windows_(?:x64|x86_64)-setup\.exe$/,
    /_darwin_(?:aarch64|arm64)\.dmg$/,
    /_darwin_(?:x64|x86_64)\.dmg$/,
    /_linux_(?:amd64|x64|x86_64)\.AppImage$/,
    /_linux_(?:amd64|x64|x86_64)\.deb$/,
  ];
  if (required.some((pattern) => !names.some((name) => pattern.test(name)))) {
    throw new Error(`Required platform packages are missing. Draft retained. Found: ${names.join(', ')}`);
  }
  await github.rest.repos.updateRelease({ ...repo, release_id: releaseId, draft: false, make_latest: 'true' });
  await core.summary.addLink('Download ShareX Desktop', release.html_url).write();
}

module.exports = { readVersion, prepare, publish };
