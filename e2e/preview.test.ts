// e2e/preview.test.ts
// This test validates TWO fixes simultaneously:
// 1. Frontend uses convertFileSrc() — produces asset://localhost URLs
// 2. tauri.conf.json has assetProtocol.enable: true — allows file loading
// If either is missing, the img src will have the wrong protocol.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

describe('Preview Panel', () => {
  let fixtureDir: string

  before(async () => {
    fixtureDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fileflow-e2e-preview-'))
    fs.copyFileSync('e2e/fixtures/sample.txt', path.join(fixtureDir, 'sample.txt'))
    fs.copyFileSync('e2e/fixtures/sample.png', path.join(fixtureDir, 'sample.png'))
    fs.copyFileSync('e2e/fixtures/sample.pdf', path.join(fixtureDir, 'sample.pdf'))

    await browser.execute(
      (dirPath: string) => {
        // @ts-expect-error
        window.__TAURI__.core.invoke('add_directory', { path: dirPath })
      },
      fixtureDir
    )

    await browser.waitUntil(
      async () => {
        const footer = await $('footer')
        const text = await footer.getText()
        return text.includes('个文件已索引') && !text.includes('0 个文件已索引')
      },
      { timeout: 30000 }
    )
  })

  after(() => {
    fs.rmSync(fixtureDir, { recursive: true, force: true })
  })

  it('image preview uses asset://localhost (not tauri://localhost)', async () => {
    // List all files and click on sample.png
    const input = await $('input[type="text"]')
    await input.setValue('sample')
    await browser.waitUntil(async () => {
      const items = await $$('[data-testid="file-item"]')
      return items.length > 0
    }, { timeout: 10000 })

    const items = await $$('[data-testid="file-item"]')
    const pngItem = await Promise.all(
      items.map(async (i) => ({ el: i, text: await i.getText() }))
    ).then((all) => all.find((x) => x.text.includes('sample.png'))?.el)

    expect(pngItem).toBeDefined()
    await pngItem!.click()

    // Wait for preview panel to show an img
    await browser.waitUntil(
      async () => !!(await $('img[src*="asset://localhost"]').isExisting()),
      { timeout: 5000, timeoutMsg: 'Image preview did not appear with asset:// URL' }
    )

    const img = await $('img[src*="asset://localhost"]')
    const src = await img.getAttribute('src')
    expect(src).toContain('asset://localhost')
    expect(src).not.toContain('tauri://localhost')
  })

  it('text preview shows file content in <pre>', async () => {
    const input = await $('input[type="text"]')
    await input.setValue('')
    await browser.pause(500)

    const items = await $$('[data-testid="file-item"]')
    const txtItem = await Promise.all(
      items.map(async (i) => ({ el: i, text: await i.getText() }))
    ).then((all) => all.find((x) => x.text.includes('sample.txt'))?.el)

    if (txtItem) await txtItem.click()

    await browser.waitUntil(
      async () => !!(await $('pre').isExisting()),
      { timeout: 5000 }
    )

    const pre = await $('pre')
    expect(await pre.getText()).toContain('fileflow-test-keyword')
  })

  it('PDF preview shows a canvas element', async () => {
    const items = await $$('[data-testid="file-item"]')
    const pdfItem = await Promise.all(
      items.map(async (i) => ({ el: i, text: await i.getText() }))
    ).then((all) => all.find((x) => x.text.includes('sample.pdf'))?.el)

    if (pdfItem) await pdfItem.click()

    await browser.waitUntil(
      async () => !!(await $('canvas').isExisting()),
      { timeout: 5000, timeoutMsg: 'PDF canvas did not appear' }
    )
  })
})
