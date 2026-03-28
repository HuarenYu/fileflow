// src/components/__tests__/SearchBar.test.tsx
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SearchBar } from '../SearchBar'

describe('SearchBar', () => {
  it('calls onQuery with typed value', async () => {
    const onQuery = vi.fn()
    render(<SearchBar query="" onQuery={onQuery} />)

    const input = screen.getByPlaceholderText('搜索文件，支持自然语言...')
    await userEvent.type(input, 'hello')

    expect(onQuery).toHaveBeenCalledTimes(5)
    expect(onQuery).toHaveBeenCalledWith('h')
  })

  it('shows clear button when query is non-empty', () => {
    render(<SearchBar query="test" onQuery={vi.fn()} />)
    expect(screen.getByText('✕')).toBeInTheDocument()
  })

  it('hides clear button when query is empty', () => {
    render(<SearchBar query="" onQuery={vi.fn()} />)
    expect(screen.queryByText('✕')).not.toBeInTheDocument()
  })

  it('calls onQuery("") when clear button clicked', async () => {
    const onQuery = vi.fn()
    render(<SearchBar query="some text" onQuery={onQuery} />)

    await userEvent.click(screen.getByText('✕'))

    expect(onQuery).toHaveBeenCalledWith('')
  })
})
