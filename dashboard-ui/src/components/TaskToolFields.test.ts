import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import type { TaskFieldDefinition, TaskFormState } from '../features/tasks'
import TaskToolFields from './TaskToolFields.vue'

const fields: TaskFieldDefinition[] = [
  { key: 'path', label: '\u8def\u5f84', type: 'text', help: '\u8f93\u5165\u6587\u4ef6\u8def\u5f84' },
  {
    key: 'format',
    label: '\u683c\u5f0f',
    type: 'select',
    options: [
      { label: 'json', value: 'json' },
      { label: 'yaml', value: 'yaml' },
    ],
  },
  { key: 'recursive', label: '\u9012\u5f52', type: 'checkbox' },
  { key: 'notes', label: '\u5907\u6ce8', type: 'textarea' },
]

const form: TaskFormState = {
  path: 'D:/repo/demo.txt',
  format: 'json',
  recursive: false,
  notes: '',
}

describe('TaskToolFields', () => {
  it('renders field help and emits updates for text-like controls', async () => {
    const wrapper = mount(TaskToolFields, { props: { fields, form } })

    expect(wrapper.text()).toContain('\u8f93\u5165\u6587\u4ef6\u8def\u5f84')

    await wrapper.get('[data-testid="task-field-path"]').setValue('D:/repo/next.txt')
    await wrapper.get('[data-testid="task-field-format"]').setValue('yaml')
    await wrapper.get('[data-testid="task-field-notes"]').setValue('memo')

    expect(wrapper.emitted('update-field')).toEqual([
      [{ key: 'path', value: 'D:/repo/next.txt' }],
      [{ key: 'format', value: 'yaml' }],
      [{ key: 'notes', value: 'memo' }],
    ])
  })

  it('emits boolean updates for checkbox controls', async () => {
    const wrapper = mount(TaskToolFields, { props: { fields, form } })

    await wrapper.get('[data-testid="task-field-recursive"]').setValue(true)

    expect(wrapper.emitted('update-field')).toEqual([
      [{ key: 'recursive', value: true }],
    ])
  })
})
