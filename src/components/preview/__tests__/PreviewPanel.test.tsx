// src/components/preview/__tests__/PreviewPanel.test.tsx
import { render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { vi } from 'vitest'
import { PreviewPanel } from '../PreviewPanel'
import type { SearchResult } from '../../../lib/types'

const mockFile: SearchResult = {
  file_id: 'file-1',
  path: '/home/user/docs/report.pdf',
  name: 'report.pdf',
  extension: 'pdf',
  size: 1024,
  modified_at: '2026-01-01T00:00:00Z',
  category: 'document',
  score: 0.9,
  thumbnail_path: null,
}

describe('PreviewPanel', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset()
  })

  it('shows placeholder when no file selected', () => {
    render(<PreviewPanel file={null} />)
    expect(screen.getByText('选择文件以预览')).toBeInTheDocument()
  })

  it('renders image with asset:// URL (not tauri://localhost)', async () => {
    const imageFile = { ...mockFile, path: '/home/user/photo.jpg', extension: 'jpg', name: 'photo.jpg' }
    vi.mocked(invoke).mockResolvedValue({ type: 'image', path: '/home/user/photo.jpg' })

    render(<PreviewPanel file={imageFile} />)

    await waitFor(() => {
      const img = document.querySelector('img')
      expect(img).toBeInTheDocument()
      expect(img).toHaveAttribute('src', expect.stringContaining('asset://localhost'))
      expect(img).not.toHaveAttribute('src', expect.stringContaining('tauri://localhost'))
    })
  })

  it('renders video with asset:// URL', async () => {
    const videoFile = { ...mockFile, path: '/home/user/clip.mp4', extension: 'mp4', name: 'clip.mp4' }
    vi.mocked(invoke).mockResolvedValue({ type: 'video', path: '/home/user/clip.mp4' })

    render(<PreviewPanel file={videoFile} />)

    await waitFor(() => {
      const video = document.querySelector('video')
      expect(video).toBeInTheDocument()
      expect(video?.src).toContain('asset://localhost')
    })
  })

  it('renders canvas for PDF type', async () => {
    vi.mocked(invoke).mockResolvedValue({ type: 'pdf', path: '/home/user/docs/report.pdf' })

    render(<PreviewPanel file={mockFile} />)

    await waitFor(() => {
      expect(document.querySelector('canvas')).toBeInTheDocument()
    })
  })

  it('renders text content in <pre>', async () => {
    const txtFile = { ...mockFile, path: '/home/user/notes.txt', extension: 'txt', name: 'notes.txt' }
    vi.mocked(invoke).mockResolvedValue({
      type: 'text',
      content: 'hello fileflow content',
      language: 'txt',
    })

    render(<PreviewPanel file={txtFile} />)

    await waitFor(() => {
      expect(screen.getByText('hello fileflow content')).toBeInTheDocument()
    })
  })

  it('does not crash when get_preview throws', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('file not found'))

    // Should render without throwing
    expect(() => render(<PreviewPanel file={mockFile} />)).not.toThrow()

    // After the rejection resolves, should show loading or empty, not crash
    await waitFor(() => {
      expect(screen.queryByText('选择文件以预览')).not.toBeInTheDocument()
    })
  })
})
