import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const queryCommandMock = vi.hoisted(() => vi.fn())
const useOperationMock = vi.hoisted(() =>
  vi.fn(() => ({
    state: { value: 'idle' },
    preview: { value: null },
    result: { value: null },
    error: { value: null },
    requestPreview: vi.fn(),
    confirm: vi.fn(),
    cancel: vi.fn(),
    reset: vi.fn(),
  })),
)

vi.mock('../api/commands', () => ({ queryCommand: queryCommandMock }))
vi.mock('../composables/useOperation', () => ({ useOperation: useOperationMock }))
vi.mock('../api', () => ({
  bookmarksBatchAddTags: vi.fn(),
  bookmarksBatchDelete: vi.fn(),
  bookmarksBatchRemoveTags: vi.fn(),
  upsertBookmark: vi.fn(),
  renameBookmark: vi.fn(),
}))
vi.mock('../ui/feedback', () => ({ pushToast: vi.fn() }))
vi.mock('../ui/tags', () => ({ tagCategoryClass: () => '' }))
vi.mock('../ui/export', () => ({ downloadCsv: vi.fn(), downloadJson: vi.fn() }))

describe('BookmarksPanel', () => {
  beforeEach(() => {
    queryCommandMock.mockReset()
    queryCommandMock.mockResolvedValue({ columns: [], rows: [] })
    useOperationMock.mockClear()
  })

  it('fetches bookmarks via WS queryCommand on mount', async () => {
    queryCommandMock.mockResolvedValue({
      columns: [
        { name: 'name', kind: 'string' },
        { name: 'path', kind: 'string' },
        { name: 'tags', kind: 'string' },
        { name: 'visits', kind: 'int' },
        { name: 'last_visited', kind: 'string' },
      ],
      rows: [
        { name: 'proj', path: 'C:\\proj', tags: 'work,rust', visits: '5', last_visited: '2026-05-14 10:00:00' },
      ],
    })

    const { default: BookmarksPanel } = await import('./BookmarksPanel.vue')
    mount(BookmarksPanel, {
      global: {
        stubs: {
          IconPlus: true, IconX: true, IconTrash: true, IconSearch: true,
          Button: defineComponent({ template: '<button><slot /></button>' }),
          SkeletonTable: true,
          OperationDialog: true,
        },
      },
    })

    await flushPromises()
    expect(queryCommandMock).toHaveBeenCalledWith('bookmark.list')
  })

  it('renders bookmark rows from WS Table data', async () => {
    queryCommandMock.mockResolvedValue({
      columns: [
        { name: 'name', kind: 'string' },
        { name: 'path', kind: 'string' },
        { name: 'tags', kind: 'string' },
        { name: 'visits', kind: 'int' },
        { name: 'last_visited', kind: 'string' },
      ],
      rows: [
        { name: 'alpha', path: 'C:\\alpha', tags: 'tag1', visits: '3', last_visited: '' },
        { name: 'beta', path: 'D:\\beta', tags: '', visits: '0', last_visited: '' },
      ],
    })

    const { default: BookmarksPanel } = await import('./BookmarksPanel.vue')
    const wrapper = mount(BookmarksPanel, {
      global: {
        stubs: {
          IconPlus: true, IconX: true, IconTrash: true, IconSearch: true,
          Button: defineComponent({ template: '<button><slot /></button>' }),
          SkeletonTable: true,
          OperationDialog: true,
        },
      },
    })

    await flushPromises()
    const rows = wrapper.findAll('tbody tr.bookmark-row')
    expect(rows.length).toBe(2)
    expect(rows[0].text()).toContain('alpha')
    expect(rows[1].text()).toContain('beta')
  })
})
