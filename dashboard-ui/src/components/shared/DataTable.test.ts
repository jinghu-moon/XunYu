import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import DataTable from './DataTable.vue'
import type { Table, ColumnDef, ValueKind } from '../../generated/types'

// Mock @tanstack/vue-virtual for jsdom (no real layout dimensions)
vi.mock('@tanstack/vue-virtual', () => ({
  useVirtualizer: (opts: any) => {
    const { computed } = require('vue')
    return computed(() => {
      const count = typeof opts.count === 'function' ? opts.count() : opts.count?.value ?? opts.count ?? 0
      const size = opts.estimateSize?.() ?? 36
      const overscan = opts.overscan ?? 10
      // Return first `overscan` items to simulate virtualization
      const items = Array.from({ length: Math.min(count, overscan) }, (_, i) => ({
        index: i,
        key: i,
        size,
        start: i * size,
        end: (i + 1) * size,
      }))
      return {
        getVirtualItems: () => items,
        getTotalSize: () => count * size,
      }
    })
  },
}))

function makeColumn(name: string, kind: ValueKind = 'string', sortable = true): ColumnDef {
  return { name, kind, sortable }
}

function makeTable(columns: ColumnDef[], rows: Record<string, unknown>[]): Table {
  return { columns, rows: rows as Table['rows'] }
}

describe('DataTable', () => {
  describe('auto column generation', () => {
    it('renders columns from Table schema', () => {
      const table = makeTable(
        [makeColumn('name'), makeColumn('path'), makeColumn('visits', 'int')],
        [],
      )
      const wrapper = mount(DataTable, { props: { table } })

      const headers = wrapper.findAll('th')
      expect(headers).toHaveLength(3)
      expect(headers[0].text()).toBe('name')
      expect(headers[1].text()).toBe('path')
      expect(headers[2].text()).toBe('visits')
    })

    it('renders rows from Table data', () => {
      const table = makeTable(
        [makeColumn('name'), makeColumn('count', 'int')],
        [
          { name: 'alpha', count: 10 },
          { name: 'beta', count: 20 },
        ],
      )
      const wrapper = mount(DataTable, { props: { table } })

      const rows = wrapper.findAll('tbody tr')
      expect(rows).toHaveLength(2)
      expect(rows[0].findAll('td')[0].text()).toBe('alpha')
      expect(rows[0].findAll('td')[1].text()).toBe('10')
      expect(rows[1].findAll('td')[0].text()).toBe('beta')
    })

    it('handles empty table', () => {
      const table = makeTable([makeColumn('name')], [])
      const wrapper = mount(DataTable, { props: { table } })

      expect(wrapper.findAll('tbody tr')).toHaveLength(0)
      expect(wrapper.find('[data-testid="empty-state"]').exists()).toBe(true)
    })
  })

  describe('sorting', () => {
    it('sorts by column click', async () => {
      const table = makeTable(
        [makeColumn('name'), makeColumn('count', 'int')],
        [
          { name: 'gamma', count: 30 },
          { name: 'alpha', count: 10 },
          { name: 'beta', count: 20 },
        ],
      )
      const wrapper = mount(DataTable, { props: { table } })

      // Click "name" header to sort ascending
      await wrapper.findAll('th')[0].trigger('click')
      const rowsAfterAsc = wrapper.findAll('tbody tr')
      expect(rowsAfterAsc[0].findAll('td')[0].text()).toBe('alpha')
      expect(rowsAfterAsc[1].findAll('td')[0].text()).toBe('beta')
      expect(rowsAfterAsc[2].findAll('td')[0].text()).toBe('gamma')

      // Click again for descending
      await wrapper.findAll('th')[0].trigger('click')
      const rowsAfterDesc = wrapper.findAll('tbody tr')
      expect(rowsAfterDesc[0].findAll('td')[0].text()).toBe('gamma')
      expect(rowsAfterDesc[2].findAll('td')[0].text()).toBe('alpha')
    })
  })

  describe('filtering', () => {
    it('filters by search input', async () => {
      const table = makeTable(
        [makeColumn('name'), makeColumn('tag')],
        [
          { name: 'bookmark-a', tag: 'shell' },
          { name: 'bookmark-b', tag: 'dev' },
          { name: 'bookmark-c', tag: 'shell' },
        ],
      )
      const wrapper = mount(DataTable, { props: { table, searchable: true } })

      const input = wrapper.find('input[data-testid="search-input"]')
      await input.setValue('shell')

      const rows = wrapper.findAll('tbody tr')
      expect(rows).toHaveLength(2)
      expect(rows[0].findAll('td')[0].text()).toBe('bookmark-a')
      expect(rows[1].findAll('td')[0].text()).toBe('bookmark-c')
    })
  })

  describe('virtual scroll', () => {
    it('enables virtual scroll for >100 rows', () => {
      const rows = Array.from({ length: 200 }, (_, i) => ({
        name: `item-${i}`,
        value: i,
      }))
      const table = makeTable(
        [makeColumn('name'), makeColumn('value', 'int')],
        rows,
      )
      const wrapper = mount(DataTable, { props: { table } })

      // Virtual container should exist
      const container = wrapper.find('[data-testid="virtual-container"]')
      expect(container.exists()).toBe(true)

      // Should render fewer DOM rows than total rows (mock returns overscan=10 items)
      const renderedRows = wrapper.findAll('tbody tr')
      expect(renderedRows.length).toBe(10)
      expect(renderedRows.length).toBeLessThan(200)
    })

    it('does not use virtual scroll for <=100 rows', () => {
      const rows = Array.from({ length: 50 }, (_, i) => ({
        name: `item-${i}`,
        value: i,
      }))
      const table = makeTable(
        [makeColumn('name'), makeColumn('value', 'int')],
        rows,
      )
      const wrapper = mount(DataTable, { props: { table } })

      expect(wrapper.find('[data-testid="virtual-container"]').exists()).toBe(false)
      expect(wrapper.findAll('tbody tr')).toHaveLength(50)
    })
  })

  describe('row selection', () => {
    it('supports row selection', async () => {
      const table = makeTable(
        [makeColumn('name')],
        [{ name: 'a' }, { name: 'b' }, { name: 'c' }],
      )
      const wrapper = mount(DataTable, { props: { table, selectable: true } })

      // Click first row checkbox
      const checkboxes = wrapper.findAll('input[type="checkbox"]')
      await checkboxes[1].trigger('change') // [0] is select-all

      expect(wrapper.emitted('selection-change')).toBeTruthy()
      const emitted = wrapper.emitted('selection-change')!
      expect(emitted[0][0]).toEqual([{ name: 'a' }])
    })
  })
})
