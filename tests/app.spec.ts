import { test, expect } from '@playwright/test'

test('preview shows the app and clearly blocks desktop-only actions', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', error => errors.push(error.message))
  await page.goto('/')
  await expect(page.getByRole('heading', { name: '拾影', exact: true })).toBeVisible()
  const parse = page.getByRole('button', { name: '解析', exact: true })
  await expect(parse).toBeDisabled()
  await page.getByRole('textbox', { name: 'X 视频链接或推文 ID' }).fill('https://x.com/user/status/12345')
  await parse.click()
  await expect(page.getByRole('alert')).toContainText('当前为界面预览')
  await expect(page.getByRole('alert')).toHaveClass(/el-message--info/)
  await expect(parse).toBeEnabled()
  await page.getByRole('button', { name: '清除链接' }).click()
  await expect(page.getByRole('textbox', { name: 'X 视频链接或推文 ID' })).toHaveValue('')
  await expect(parse).toBeDisabled()
  expect(errors).toEqual([])
})

test('settings exposes locally saved credentials and closes with Escape', async ({ page }) => {
  await page.setViewportSize({ width: 740, height: 460 })
  await page.goto('/')
  await page.getByRole('button', { name: '设置', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: '拾影设置' })
  await expect(dialog).toBeVisible()
  await expect(dialog).not.toContainText('自动保存到本地')
  await expect(dialog.getByLabel('数据目录', { exact: true })).toHaveAttribute('readonly', '')
  await expect(dialog.getByLabel('X 登录 Cookie', { exact: false })).toBeVisible()
  await expect(dialog.getByLabel('Authorization', { exact: false })).toBeVisible()
  await expect(dialog.getByText('高级设置', { exact: true })).toHaveCount(0)
  await expect(dialog.getByRole('button', { name: '登录 X 自动获取', exact: true })).toHaveCount(0)
  await expect(dialog.getByRole('button', { name: '保存设置' })).toBeInViewport()
  await expect(dialog.getByRole('button', { name: '取消', exact: true })).toBeInViewport()
  await dialog.hover()
  await page.screenshot({ path: 'artifacts/settings-scrollbar.png', fullPage: true })
  await page.keyboard.press('Escape')
  await expect(dialog).not.toBeVisible()
})

test('minimum window width has no horizontal overflow', async ({ page }) => {
  await page.setViewportSize({ width: 600, height: 460 })
  await page.goto('/')
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy()
})

test('desktop-sized interface screenshot', async ({ page }) => {
  await page.setViewportSize({ width: 740, height: 560 })
  await page.goto('/')
  await expect.poll(() => page.evaluate(() => document.documentElement.scrollHeight <= innerHeight)).toBeTruthy()
  await page.screenshot({ path: 'artifacts/ui-preview.png', fullPage: true })
})


test('quality dropdown is styled, keyboard accessible and downloads the selected variant', async ({ page }) => {
  await page.setViewportSize({ width: 740, height: 560 })
  await page.addInitScript(() => {
    Object.defineProperty(window, 'isTauri', { value: true })
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {
      transformCallback: () => 1,
      invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === 'plugin:event|listen') return 1
        if (command === 'list_tasks') return []
        if (command === 'get_settings') return { downloadDir: 'C:\\Downloads\\拾影', dataDir: 'C:/Users/Example/AppData/Local/site.crazysand.shiying', queryId: 'query', cookie: 'auth_token=example; ct0=csrf', authorization: 'Bearer example-test-token', hasCookie: true, hasAuthorization: true }
        if (command === 'parse_video') return [{
          id: '123_1', tweetId: '123', title: '视频预览', thumbnail: '',
          variants: [
            { url: 'https://video.twimg.com/high.mp4', bitrate: 832000, label: '656 × 848 · 推荐' },
            { url: 'https://video.twimg.com/medium.mp4', bitrate: 256000, label: '480 × 620' },
            { url: 'https://video.twimg.com/low.mp4', bitrate: 128000, label: '320 × 412' },
          ],
        }]
        if (command === 'start_download') {
          Object.defineProperty(window, '__selectedVariant', { value: args.variantIndex, configurable: true })
          return 'task-1'
        }
        throw new Error('Unexpected mocked command: ' + command)
      },
    } })
  })
  await page.goto('/')
  await page.getByRole('textbox', { name: 'X 视频链接或推文 ID' }).fill('123')
  await page.getByRole('button', { name: '解析', exact: true }).click()
  const select = page.getByRole('combobox', { name: '下载清晰度' })
  await page.locator('.quality-select .el-select__wrapper').click()
  await expect(page.getByRole('listbox')).toBeVisible()
  await expect(page.getByRole('option', { name: /656 × 848.*推荐/ })).toHaveAttribute('aria-selected', 'true')
  await page.getByRole('option', { name: '480 × 620', exact: true }).click()
  await expect(page.locator('.quality-select')).toContainText('480 × 620')
  await page.getByRole('button', { name: '下载', exact: true }).click()
  expect(await page.evaluate(() => Reflect.get(window, '__selectedVariant'))).toBe(1)
  await page.locator('.quality-select .el-select__wrapper').click()
  await page.screenshot({ path: 'artifacts/quality-dropdown.png', fullPage: true, animations: 'disabled' })
  await select.press('ArrowDown')
  await select.press('Enter')
  await expect(page.locator('.quality-select')).toContainText('320 × 412')
  await page.getByRole('button', { name: '下载', exact: true }).click()
  expect(await page.evaluate(() => Reflect.get(window, '__selectedVariant'))).toBe(2)
  await page.setViewportSize({ width: 740, height: 320 })
  const scroll = page.locator('.page-scroll-wrap')
  expect(await scroll.evaluate(el => el.scrollHeight > el.clientHeight)).toBeTruthy()
  await expect.poll(() => page.evaluate(() => document.documentElement.scrollHeight <= innerHeight)).toBeTruthy()
  await page.locator('.app-header').hover()
  await page.mouse.wheel(0, 400)
  await expect.poll(() => scroll.evaluate(el => el.scrollTop)).toBeGreaterThan(0)
})


test('user-provided authorization is submitted and missing credentials block parsing', async ({ page }) => {
  await page.addInitScript(() => {
    let configured = false
    Object.defineProperty(window, 'isTauri', { value: true })
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {
      transformCallback: () => 1,
      invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === 'plugin:event|listen') return 1
        if (command === 'list_tasks') return []
        if (command === 'get_settings') return { downloadDir: 'C:\\Downloads\\拾影', dataDir: 'C:/Users/Example/AppData/Local/site.crazysand.shiying', queryId: 'query', cookie: 'auth_token=example; ct0=csrf', authorization: configured ? 'Bearer example-test-token' : '', hasCookie: true, hasAuthorization: configured }
        if (command === 'save_settings') {
          Object.defineProperty(window, '__savedCredentials', { value: { cookie: args.cookie, authorization: args.authorization }, configurable: true })
          configured = true
          return { downloadDir: 'C:\\Downloads\\拾影', dataDir: 'C:/Users/Example/AppData/Local/site.crazysand.shiying', queryId: 'query', cookie: 'auth_token=example; ct0=csrf', authorization: 'Bearer example-test-token', hasCookie: true, hasAuthorization: true }
        }
        if (command === 'parse_video') {
          if (!configured) throw new Error('Parsed before Authorization was configured')
          Object.defineProperty(window, '__parseResumed', { value: true, configurable: true })
          return []
        }
        throw new Error('Unexpected mocked command: ' + command)
      },
    } })
  })
  await page.goto('/')
  await page.getByRole('textbox', { name: 'X 视频链接或推文 ID' }).fill('123')
  await page.getByRole('button', { name: '解析', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: '拾影设置' })
  await expect(dialog).toBeVisible()
  await expect(dialog.getByLabel('数据目录', { exact: true })).toHaveValue('C:/Users/Example/AppData/Local/site.crazysand.shiying')
  const authorization = dialog.getByLabel('Authorization', { exact: false })
  await expect(authorization).toHaveAttribute('type', 'text')
  await expect(authorization).toHaveValue('')
  await authorization.fill('example-test-token')
  await dialog.getByRole('button', { name: '保存设置' }).click()
  await expect(dialog).not.toBeVisible()
  expect(await page.evaluate(() => Reflect.get(window, '__savedCredentials'))).toEqual({ cookie: 'auth_token=example; ct0=csrf', authorization: 'example-test-token' })
  await expect.poll(() => page.evaluate(() => Reflect.get(window, '__parseResumed'))).toBe(true)
  await page.getByRole('button', { name: '设置', exact: true }).click()
  await expect(dialog.getByLabel('Authorization', { exact: false })).toHaveValue('Bearer example-test-token')
  await expect(dialog.getByLabel('X 登录 Cookie', { exact: false })).toHaveValue('auth_token=example; ct0=csrf')
})

test('deleting a download record updates the list only after a successful save', async ({ page }) => {
  await page.addInitScript(() => {
    let calls = 0
    const record = { id: 'done', title: '已保存的视频', thumbnail: '', filename: 'example.mp4', path: 'C:/Downloads/example.mp4', status: 'completed', downloaded: 1024, total: 1024, speed: 0, error: null }
    Object.defineProperty(window, 'isTauri', { value: true })
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {
      transformCallback: () => 1,
      invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === 'plugin:event|listen') return 1
        if (command === 'get_settings') return { downloadDir: 'C:/Downloads', dataDir: 'C:/Users/Example/AppData/Local/site.crazysand.shiying', queryId: 'query', cookie: '', authorization: '', hasCookie: false, hasAuthorization: false }
        if (command === 'list_tasks') return localStorage.getItem('removed') ? [] : [record]
        if (command === 'reveal_download') throw new Error('文件已被移动或删除')
        if (command === 'remove_download') {
          if (args.id !== record.id) throw new Error('Wrong task')
          if (++calls === 1) throw new Error('保存下载记录失败')
          localStorage.setItem('removed', 'true')
          return
        }
        throw new Error('Unexpected mocked command: ' + command)
      },
    } })
  })
  await page.goto('/')
  const record = page.locator('.task-row')
  await expect(record).toContainText('已保存的视频')
  await record.getByRole('button', { name: '在文件夹中显示' }).click()
  await expect(page.getByRole('alert')).toContainText('文件已被移动或删除')
  await expect(page.getByRole('alert')).toHaveClass(/el-message--warning/)
  await record.getByRole('button', { name: '删除下载记录' }).click()
  await expect(page.getByText('删除此记录？视频文件会保留。', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: '取消', exact: true }).click()
  await expect(record).toBeVisible()
  await expect(page.getByRole('button', { name: '删除', exact: true })).not.toBeVisible()
  await record.getByRole('button', { name: '删除下载记录' }).click()
  await page.getByRole('button', { name: '删除', exact: true }).click()
  await expect(page.getByRole('alert').filter({ hasText: '保存下载记录失败' })).toBeVisible()
  await expect(page.getByRole('alert').filter({ hasText: '保存下载记录失败' })).toHaveClass(/el-message--error/)
  await expect(record).toBeVisible()
  await record.getByRole('button', { name: '删除下载记录' }).click()
  await page.getByRole('button', { name: '删除', exact: true }).click()
  await expect(record).toHaveCount(0)
  await expect(page.getByRole('alert').filter({ hasText: '已删除下载记录' })).toHaveClass(/el-message--success/)
  await expect(page.getByText('暂无下载', { exact: true })).toBeVisible()
  await page.reload()
  await expect(page.getByText('暂无下载', { exact: true })).toBeVisible()
})
