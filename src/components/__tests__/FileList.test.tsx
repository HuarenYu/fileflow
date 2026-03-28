import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { vi } from 'vitest'
import { FileList } from '../FileList'
import type { SearchResult } from '../../lib/types'

// Mock hooks at module level — cleaner than controlling invoke return values
vi.mock('../../hooks/useFiles', () => ({
  useFiles: vi.fn(() => ({ files: [], loading: false })),
}))
vi.mock('../../hooks/useSearch', () => ({
  useSearch: vi.fn(() => ({ results: [], loading: false })),
}))

import { useFiles } from '../../hooks/useFiles'
import { useSearch } from '../../hooks/useSearch'

const mockResult: SearchResult = {
  file_id: 'f1',
  path: '/docs/report.pdf',
  name: 'report.pdf',
  extension: 'pdf',
  size: 2048,
  modified_at: '2026-01-01T00:00:00Z',
  category: 'document',
  score: 0.95,
  thumbnail_path: null,
}

describe('FileList', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows empty state when no query and no files', () => {
    render(<FileList category="all" query="" onSelect={vi.fn()} />)
    expect(screen.getByText('此分类暂无文件')).toBeInTheDocument()
  })

  it('shows "未找到匹配文件" when searching with no results', () => {
    render(<FileList category="all" query="xyz123" onSelect={vi.fn()} />)
    expect(screen.getByText('未找到匹配文件')).toBeInTheDocument()
  })

  it('renders search results when query is non-empty', () => {
    vi.mocked(useSearch).mockReturnValue({ results: [mockResult], loading: false })

    render(<FileList category="all" query="report" onSelect={vi.fn()} />)

    expect(screen.getByText('report.pdf')).toBeInTheDocument()
  })

  it('calls onSelect when a file item is clicked', async () => {
    vi.mocked(useSearch).mockReturnValue({ results: [mockResult], loading: false })
    const onSelect = vi.fn()

    render(<FileList category="all" query="report" onSelect={onSelect} />)

    await userEvent.click(screen.getByText('report.pdf'))

    expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ file_id: 'f1' }))
  })
})
