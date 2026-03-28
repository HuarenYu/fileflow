import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

describe('Search', () => {
  let fixtureDir: string

  before(async () => {
    fixtureDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fileflow-e2e-search-'))
    fs.copyFileSync('e2e/fixtures/sample.txt', path.join(fixtureDir, 'sample.txt'))

    // Index the directory
    await browser.execute(
      (dirPath: string) => {
        // @ts-expect-error
        return window.__TAURI__.core.invoke('add_directory', { path: dirPath })
      },
      fixtureDir
    )

    // Wait for indexing to complete
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

  it('returns results for a keyword present in indexed file', async () => {
    const input = await $('input[type="text"]')
    await input.setValue('fileflow-test-keyword')

    await browser.waitUntil(
      async () => {
        const items = await $$('[data-testid="file-item"]')
        return items.length > 0
      },
      { timeout: 10000, timeoutMsg: 'No search results appeared' }
    )

    const items = await $$('[data-testid="file-item"]')
    expect(items.length).toBeGreaterThan(0)

    // sample.txt should appear in results
    const texts = await Promise.all(items.map((i) => i.getText()))
    expect(texts.some((t) => t.includes('sample.txt'))).toBe(true)
  })
})
