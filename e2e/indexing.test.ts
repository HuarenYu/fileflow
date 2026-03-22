import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

describe('Directory Indexing', () => {
  let fixtureDir: string

  before(async () => {
    // Copy fixtures to a temp dir so we don't pollute the source tree
    fixtureDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fileflow-e2e-'))
    fs.copyFileSync('e2e/fixtures/sample.txt', path.join(fixtureDir, 'sample.txt'))
    fs.copyFileSync('e2e/fixtures/sample.png', path.join(fixtureDir, 'sample.png'))
  })

  after(() => {
    fs.rmSync(fixtureDir, { recursive: true, force: true })
  })

  it('indexes a directory and shows progress in StatusBar', async () => {
    // Click "添加目录" button
    const addBtn = await $('[data-testid="add-directory"]')
    await addBtn.click()

    // The dialog is handled by Tauri — inject the path via keyboard shortcut
    // Since file dialogs are OS-native, we instead invoke the command directly
    // through WebdriverIO's execute to call the Tauri command
    await browser.execute(
      (dirPath: string) => {
        // @ts-expect-error tauri is available in the WebView
        window.__TAURI__.core.invoke('add_directory', { path: dirPath })
      },
      fixtureDir
    )

    // Wait for StatusBar to show indexed > 0
    await browser.waitUntil(
      async () => {
        const footer = await $('footer')
        const text = await footer.getText()
        return text.includes('个文件已索引') && !text.includes('0 个文件已索引')
      },
      { timeout: 30000, timeoutMsg: 'Indexing did not complete within 30s' }
    )

    const footer = await $('footer')
    expect(await footer.getText()).not.toContain('个失败')
  })
})
