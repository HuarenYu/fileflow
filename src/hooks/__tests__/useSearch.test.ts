// src/hooks/__tests__/useSearch.test.ts
import { renderHook, act } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { vi } from 'vitest'
import { useSearch } from '../useSearch'

describe('useSearch', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset()
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('calls invoke("search_files") after 300ms debounce', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const { rerender } = renderHook(({ q }) => useSearch(q), {
      initialProps: { q: '' },
    })

    rerender({ q: 'hello' })

    // Before debounce fires — invoke should NOT have been called
    expect(invoke).not.toHaveBeenCalled()

    // Advance past debounce
    await act(async () => {
      vi.advanceTimersByTime(300)
    })

    expect(invoke).toHaveBeenCalledWith('search_files', expect.objectContaining({
      query: 'hello',
    }))
  })

  it('does not call invoke for empty or whitespace query', async () => {
    const { rerender } = renderHook(({ q }) => useSearch(q), {
      initialProps: { q: 'hello' },
    })

    rerender({ q: '   ' })

    await act(async () => {
      vi.advanceTimersByTime(500)
    })

    expect(invoke).not.toHaveBeenCalled()
  })

  it('updates results state after invoke resolves', async () => {
    const mockResults = [{
      file_id: 'f1', path: '/a.txt', name: 'a.txt',
      extension: 'txt', size: 100, modified_at: '2026-01-01',
      category: 'document', score: 0.9, thumbnail_path: null,
    }]
    vi.mocked(invoke).mockResolvedValue(mockResults)

    const { result, rerender } = renderHook(({ q }) => useSearch(q), {
      initialProps: { q: '' },
    })

    rerender({ q: 'search term' })

    await act(async () => {
      vi.advanceTimersByTime(300)
      // Let the promise resolve
      await Promise.resolve()
    })

    expect(result.current.results).toHaveLength(1)
    expect(result.current.results[0].name).toBe('a.txt')
  })
})
