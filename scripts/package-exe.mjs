import { copyFileSync, mkdirSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { join } from 'node:path'

try {
  const root = fileURLToPath(new URL('../', import.meta.url))
  const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))
  const config = JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8'))
  const release = join(root, 'src-tauri/target/release')
  const executable = join(release, pkg.name + '.exe')
  const installer = join(release, 'bundle/nsis', config.productName + '_' + pkg.version + '_x64-setup.exe')
  const bytes = readFileSync(executable)
  const pe = bytes.length >= 64 ? bytes.readUInt32LE(0x3c) : -1
  if (pe < 0 || pe + 6 > bytes.length || bytes.toString('ascii', pe, pe + 4) !== 'PE\0\0' || bytes.readUInt16LE(pe + 4) !== 0x8664) {
    throw new Error('可执行文件不是 Windows x64，停止生成带 x64 标记的发布文件')
  }
  // Read both inputs before exporting, so a missing installer cannot produce a partial release.
  readFileSync(installer)
  const output = join(root, 'artifacts/releases', pkg.version)
  mkdirSync(output, { recursive: true })
  const base = pkg.name + '_' + pkg.version + '_windows_x64'
  for (const [source, name] of [[executable, base + '.exe'], [installer, base + '_setup.exe']]) {
    const destination = join(output, name)
    copyFileSync(source, destination)
    console.log('发布文件：' + destination)
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
}
