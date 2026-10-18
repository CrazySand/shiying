import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'

const root = fileURLToPath(new URL('../', import.meta.url))
const args = process.argv.slice(2)
const checking = args.includes('--check')
const versions = args.filter(arg => arg !== '--check')
if (versions.length > 1 || (checking && versions.length) || versions.some(arg => arg.startsWith('-'))) {
  console.error('用法：npm run version:set -- 0.1.1，或 npm run version:sync / version:check')
  process.exit(1)
}

try {
  const files = ['package.json', 'package-lock.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock', 'src-tauri/tauri.conf.json']
  const original = new Map(files.map(file => [file, readFileSync(resolve(root, file), 'utf8')]))
  const pkg = JSON.parse(original.get('package.json'))
  const version = versions[0] ?? pkg.version
  const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/
  const match = typeof version === 'string' && version.match(semver)
  if (!match || match[4]?.split('.').some(part => /^\d+$/.test(part) && part.length > 1 && part[0] === '0')) {
    throw new Error('版本号必须符合 SemVer，例如 0.1.1 或 0.2.0-beta.1')
  }

  const next = new Map(original)
  const json = value => JSON.stringify(value, null, 2) + '\n'
  pkg.version = version
  next.set('package.json', json(pkg))

  const lock = JSON.parse(original.get('package-lock.json'))
  if (lock.name !== pkg.name || lock.packages?.['']?.name !== pkg.name) {
    throw new Error('package-lock.json 的根项目名称与 package.json 不一致')
  }
  lock.version = version
  lock.packages[''].version = version
  next.set('package-lock.json', json(lock))

  const toml = original.get('src-tauri/Cargo.toml')
  const packageVersion = /(^\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m
  if (!packageVersion.test(toml)) throw new Error('Cargo.toml 中未找到 [package] version')
  next.set('src-tauri/Cargo.toml', toml.replace(packageVersion, (_, prefix, suffix) => prefix + version + suffix))

  const cargo = original.get('src-tauri/Cargo.lock').split(/(?=^\[\[package\]\])/m)
  const blocks = cargo.map((block, index) => ({ block, index })).filter(({ block }) => {
    const name = block.match(/^name\s*=\s*"([^"]+)"/m)
    return name?.[1] === pkg.name
  })
  if (blocks.length !== 1 || !/^version\s*=\s*"[^"]+"/m.test(blocks[0].block)) {
    throw new Error('Cargo.lock 中未找到唯一的项目版本条目')
  }
  cargo[blocks[0].index] = blocks[0].block.replace(/(^version\s*=\s*")[^"]+(")/m, (_, prefix, suffix) => prefix + version + suffix)
  next.set('src-tauri/Cargo.lock', cargo.join(''))

  const config = JSON.parse(original.get('src-tauri/tauri.conf.json'))
  config.version = '../package.json'
  next.set('src-tauri/tauri.conf.json', json(config))

  // Validate every file before writing; only the root package version is authoritative.
  const changed = files.filter(file => next.get(file) !== original.get(file))
  if (checking && changed.length) {
    console.error('版本配置不同步，请执行 npm run version:sync：\n' + changed.join('\n'))
    process.exitCode = 1
  } else if (checking) {
    console.log('版本检查通过：' + version)
  } else {
    for (const file of changed) writeFileSync(resolve(root, file), next.get(file))
    console.log('项目版本：' + version + (changed.length ? '；已同步 ' + changed.length + ' 个文件' : '；所有文件已同步'))
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
}
