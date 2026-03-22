// src/components/__tests__/StatusBar.test.tsx
import { render, screen } from '@testing-library/react'
import { vi } from 'vitest'
import { StatusBar } from '../StatusBar'

vi.mock('../../hooks/useIndexProgress', () => ({
  useIndexProgress: vi.fn(() => ({
    total: 0,
    indexed: 0,
    failed: 0,
    is_running: false,
  })),
}))

import { useIndexProgress } from '../../hooks/useIndexProgress'

describe('StatusBar', () => {
  it('shows indexed count when idle', () => {
    vi.mocked(useIndexProgress).mockReturnValue({
      total: 10, indexed: 8, failed: 0, is_running: false,
    })
    render(<StatusBar />)
    expect(screen.getByText('8 个文件已索引')).toBeInTheDocument()
  })

  it('shows failed count when failed > 0', () => {
    vi.mocked(useIndexProgress).mockReturnValue({
      total: 10, indexed: 8, failed: 2, is_running: false,
    })
    render(<StatusBar />)
    expect(screen.getByText(/2 个失败/)).toBeInTheDocument()
  })

  it('shows indexing state when is_running', () => {
    vi.mocked(useIndexProgress).mockReturnValue({
      total: 20, indexed: 5, failed: 0, is_running: true,
    })
    render(<StatusBar />)
    expect(screen.getByText('● 索引中')).toBeInTheDocument()
    expect(screen.getByText('5 / 20 文件')).toBeInTheDocument()
  })
})
