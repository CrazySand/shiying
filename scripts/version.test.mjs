import { test } from 'node:test'
import assert from 'node:assert/strict'
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'

test('version updates stay consistent without changing dependency versions', () => {
  const source = fileURLToPath(new URL('../', import.meta.url))
  const fixture = mkdtempSync(join(tmpdir(), 'shiying-version-'))
  const files = ['package.json', 'package-lock.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock', 'src-tauri/tauri.conf.json']
  const json = file => JSON.parse(readFileSync(join(fixture, file), 'utf8'))
  const snapshot = () => files.map(file => readFileSync(join(fixture, file), 'utf8'))
  const run = (...args) => spawnSync(process.execPath, [join(fixture, 'scripts/version.mjs'), ...args], { encoding: 'utf8' })
  try {
    for (const file of [...files, 'scripts/version.mjs']) {
      mkdirSync(dirname(join(fixture, file)), { recursive: true })
      copyFileSync(join(source, file), join(fixture, file))
    }
    const dependencies = json('package-lock.json').packages
    delete dependencies['']
    assert.equal(run('0.2.0-beta.1').status, 0)
    assert.equal(run('--check').status, 0)
    assert.equal(json('package.json').version, '0.2.0-beta.1')
    assert.equal(json('package-lock.json').version, '0.2.0-beta.1')
    assert.equal(json('package-lock.json').packages[''].version, '0.2.0-beta.1')
    assert.equal(json('src-tauri/tauri.conf.json').version, '../package.json')
    assert.match(readFileSync(join(fixture, 'src-tauri/Cargo.toml'), 'utf8'), /version = "0.2.0-beta.1"/)
    const actualDependencies = json('package-lock.json').packages
    delete actualDependencies['']
    assert.deepEqual(actualDependencies, dependencies)

    let before = snapshot()
    assert.equal(run('01.2.3').status, 1)
    assert.equal(run('0.2.0-beta.01').status, 1)
    assert.deepEqual(snapshot(), before)

    const pkg = json('package.json')
    pkg.version = '0.2.1'
    writeFileSync(join(fixture, 'package.json'), JSON.stringify(pkg, null, 2) + '\n')
    before = snapshot()
    assert.equal(run('--check').status, 1)
    assert.deepEqual(snapshot(), before)
    assert.equal(run().status, 0)
    assert.equal(run('--check').status, 0)
    const cargo = readFileSync(join(fixture, 'src-tauri/Cargo.lock'), 'utf8')
    assert.match(cargo, /name = "shiying"\r?\nversion = "0.2.1"/)
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
})
