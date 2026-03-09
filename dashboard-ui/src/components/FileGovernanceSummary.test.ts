import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { TaskProcessOutput, WorkspaceTaskDetails } from '../types'
import type { TaskFormState, WorkspaceTaskDefinition } from '../workspace-tools'
import FileGovernanceSummary from './FileGovernanceSummary.vue'

function createTask(action: WorkspaceTaskDefinition['action']): WorkspaceTaskDefinition {
  return {
    id: action,
    workspace: 'files-security',
    title: action,
    description: action,
    action,
    mode: 'guarded',
    fields: [],
    target: (values) => String(values.path ?? ''),
  }
}

function createProcess(stdout: string, success = true): TaskProcessOutput {
  return {
    command_line: 'xun demo',
    exit_code: success ? 0 : 1,
    success,
    stdout,
    stderr: '',
    duration_ms: 8,
  }
}

function createAclDiffDetails(): WorkspaceTaskDetails {
  return {
    kind: 'acl_diff',
    diff: {
      target: 'D:/repo/a.txt',
      reference: 'D:/repo/b.txt',
      common_count: 5,
      has_diff: true,
      owner_diff: {
        target: 'BUILTIN\\Administrators',
        reference: 'NT AUTHORITY\\SYSTEM',
      },
      inheritance_diff: {
        target_protected: false,
        reference_protected: true,
      },
      only_in_target: [
        {
          principal: 'BUILTIN\\Users',
          sid: 'S-1-5-32-545',
          rights: 'Read',
          ace_type: 'Allow',
          source: 'explicit',
          inheritance: 'BothInherit',
          propagation: 'None',
          orphan: false,
        },
      ],
      only_in_reference: [
        {
          principal: 'DOMAIN\\alice',
          sid: 'S-1-5-21-100',
          rights: 'Modify',
          ace_type: 'Allow',
          source: 'inherited',
          inheritance: 'ContainerInherit',
          propagation: 'InheritOnly',
          orphan: true,
        },
      ],
    },
  }
}

describe('FileGovernanceSummary', () => {
  it('renders protect:set preview summary from protect status json', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('protect:set'),
        phase: 'preview',
        form: {
          path: 'D:/repo/demo.txt',
          deny: 'delete,move,rename',
          require: 'force,reason',
          systemAcl: true,
        } satisfies TaskFormState,
        process: createProcess('[{"path":"D:/repo/demo.txt","deny":["delete"],"require":["force"]}]'),
      },
    })

    expect(wrapper.get('[data-testid="governance-summary-preview"]').text()).toContain('保护变更预演摘要')
    expect(wrapper.text()).toContain('当前命中规则')
    expect(wrapper.text()).toContain('1 条')
    expect(wrapper.text()).toContain('更新现有保护规则')
    expect(wrapper.text()).toContain('delete / move / rename')
    expect(wrapper.text()).toContain('force / reason')
    expect(wrapper.text()).toContain('同步系统 ACL')
    expect(wrapper.text()).toContain('是')
  })

  it('renders acl:add preview summary from textual acl output', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('acl:add'),
        phase: 'preview',
        form: {
          path: 'D:/repo/demo.txt',
          principal: 'BUILTIN\\Users',
          rights: 'Read',
          aceType: 'Allow',
          inherit: 'BothInherit',
        } satisfies TaskFormState,
        process: createProcess(
          [
            'Path: D:/repo/demo.txt',
            'Owner: BUILTIN\\Administrators | Inherit: enabled',
            'Total: 5 (Allow 4 / Deny 1)  Explicit 2  Inherited 3  Orphan 0',
            '',
            '#1 Allow BUILTIN\\Users',
          ].join('\n'),
        ),
      },
    })

    expect(wrapper.text()).toContain('ACL 变更预演摘要')
    expect(wrapper.text()).toContain('BUILTIN\\Users')
    expect(wrapper.text()).toContain('BUILTIN\\Administrators')
    expect(wrapper.text()).toContain('5 条')
    expect(wrapper.text()).toContain('2 / 3')
  })

  it('renders acl:diff execute summary from textual diff output', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('acl:diff'),
        phase: 'execute',
        form: {
          path: 'D:/repo/a.txt',
          reference: 'D:/repo/b.txt',
          output: 'D:/repo/acl-diff.csv',
        } satisfies TaskFormState,
        process: createProcess(
          [
            'Path: D:/repo/a.txt',
            'Reference: D:/repo/b.txt',
            'Owner differs',
            'Inheritance differs',
            'Only in A: 2',
            'Only in B: 1',
            'Common: 5',
            'Exported 3 rows to D:/repo/acl-diff.csv',
          ].join('\n'),
        ),
      },
    })

    expect(wrapper.text()).toContain('ACL 差异摘要')
    expect(wrapper.text()).toContain('D:/repo/b.txt')
    expect(wrapper.text()).toContain('2 条')
    expect(wrapper.text()).toContain('1 条')
    expect(wrapper.text()).toContain('5 条')
    expect(wrapper.text()).toContain('有差异')
    expect(wrapper.text()).toContain('D:/repo/acl-diff.csv')
  })

  it('renders acl:backup execute summary from backup output', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('acl:backup'),
        phase: 'execute',
        form: {
          path: 'D:/repo/demo.txt',
          output: 'D:/repo/demo.acl.json',
        } satisfies TaskFormState,
        process: createProcess('Backed up 6 entries -> D:/repo/demo.acl.json'),
      },
    })

    expect(wrapper.text()).toContain('ACL 备份摘要')
    expect(wrapper.text()).toContain('D:/repo/demo.acl.json')
    expect(wrapper.text()).toContain('6 条')
    expect(wrapper.text()).toContain('ACL 备份已导出')
  })

  it('renders acl:copy preview summary using diff output', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('acl:copy'),
        phase: 'preview',
        form: {
          path: 'D:/repo/a.txt',
          reference: 'D:/repo/template.txt',
        } satisfies TaskFormState,
        process: createProcess(
          [
            'Path: D:/repo/a.txt',
            'Reference: D:/repo/template.txt',
            'Only in A: 2',
            'Only in B: 4',
            'Common: 1',
          ].join('\n'),
        ),
      },
    })

    expect(wrapper.text()).toContain('ACL 覆盖预演摘要')
    expect(wrapper.text()).toContain('D:/repo/template.txt')
    expect(wrapper.text()).toContain('2 条')
    expect(wrapper.text()).toContain('4 条')
    expect(wrapper.text()).toContain('整体覆盖目标 ACL')
  })

  it('renders acl:owner preview summary from acl view output', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('acl:owner'),
        phase: 'preview',
        form: {
          path: 'D:/repo/demo.txt',
          set: 'BUILTIN\\Administrators',
        } satisfies TaskFormState,
        process: createProcess(
          [
            'Path: D:/repo/demo.txt',
            'Owner: NT AUTHORITY\\SYSTEM | Inherit: enabled',
            'Total: 4 (Allow 4 / Deny 0)  Explicit 1  Inherited 3  Orphan 0',
          ].join('\n'),
        ),
      },
    })

    expect(wrapper.text()).toContain('ACL Owner 预演摘要')
    expect(wrapper.text()).toContain('NT AUTHORITY\\SYSTEM')
    expect(wrapper.text()).toContain('BUILTIN\\Administrators')
  })

  it('renders acl:repair execute summary and export hint', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('acl:repair'),
        phase: 'execute',
        form: {
          path: 'D:/repo/demo.txt',
          exportErrors: true,
        } satisfies TaskFormState,
        process: createProcess('Exported 3 errors to D:/repo/repair-errors.csv', false),
      },
    })

    expect(wrapper.text()).toContain('ACL 修复执行摘要')
    expect(wrapper.text()).toContain('修复存在失败')
    expect(wrapper.text()).toContain('D:/repo/repair-errors.csv')
    expect(wrapper.text()).toContain('3 条')
    expect(wrapper.text()).toContain('高风险治理动作')
  })

  it('renders encrypt preview summary and explains rule-test boundary', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('encrypt'),
        phase: 'preview',
        form: {
          path: 'D:/repo/demo.txt',
          efs: false,
          to: 'age1abc\nage1def',
          out: '',
        } satisfies TaskFormState,
        process: createProcess('path: "D:/repo/demo.txt"  (is_dir=false)\n  -> Decision: INCLUDE (source: inherited)'),
      },
    })

    expect(wrapper.text()).toContain('加密预演摘要')
    expect(wrapper.text()).toContain('age 收件人')
    expect(wrapper.text()).toContain('2 个')
    expect(wrapper.text()).toContain('D:/repo/demo.txt.age')
    expect(wrapper.text()).toContain('INCLUDE (inherited)')
    expect(wrapper.text()).toContain('当前预演只执行规则测试')
  })

  it('stays hidden for non-governance actions', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('rm'),
        phase: 'preview',
        form: { path: 'D:/repo/demo.txt' } satisfies TaskFormState,
        process: createProcess('preview ok'),
      },
    })

    expect(wrapper.find('[data-testid="governance-summary-preview"]').exists()).toBe(false)
  })


  it('renders acl:effective execute summary', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('acl:effective'),
        phase: 'execute',
        form: {
          path: 'D:/repo/demo.txt',
          user: 'DOMAIN\\alice',
        } satisfies TaskFormState,
        process: createProcess('Allow: Read, Write'),
      },
    })

    expect(wrapper.text()).toContain('ACL 有效权限摘要')
    expect(wrapper.text()).toContain('DOMAIN\\alice')
    expect(wrapper.text()).toContain('有效权限已返回')
  })

  it('renders structured acl diff details when provided', () => {
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('acl:diff'),
        phase: 'execute',
        form: {
          path: 'D:/repo/a.txt',
          reference: 'D:/repo/b.txt',
          output: '',
        } satisfies TaskFormState,
        process: createProcess('Only in A: 1\nOnly in B: 1\nCommon: 5'),
        details: createAclDiffDetails(),
      },
    })

    expect(wrapper.find('[data-testid="acl-diff-details"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('S-1-5-32-545')
    expect(wrapper.text()).toContain('S-1-5-21-100')
    expect(wrapper.text()).toContain('S-1-5-32-545')
  })

  it('renders acl copy transition details when receipt carries before and after diff', () => {
    const details = createAclDiffDetails()
    if (details.kind !== 'acl_diff') throw new Error('expected acl diff details')
    const diff = details.diff
    const wrapper = mount(FileGovernanceSummary, {
      props: {
        task: createTask('acl:copy'),
        phase: 'execute',
        form: {
          path: 'D:/repo/a.txt',
          reference: 'D:/repo/template.txt',
        } satisfies TaskFormState,
        process: createProcess('ACL copied'),
        details: {
          kind: 'acl_diff_transition',
          before: diff,
          after: {
            ...diff,
            has_diff: false,
            only_in_target: [],
            only_in_reference: [],
          },
        } satisfies WorkspaceTaskDetails,
      },
    })

    expect(wrapper.find('[data-testid="acl-diff-panel-before"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="acl-diff-panel-after"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('S-1-5-32-545')
    expect(wrapper.get('[data-testid="acl-diff-panel-after"]').text()).toContain('D:/repo/b.txt')
  })

})
