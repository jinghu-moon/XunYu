import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import RecipePanel from './RecipePanel.vue'

const apiMocks = vi.hoisted(() => ({
  fetchWorkspaceRecipes: vi.fn(),
  saveWorkspaceRecipe: vi.fn(),
  previewWorkspaceRecipe: vi.fn(),
  executeWorkspaceRecipe: vi.fn(),
}))

vi.mock('../api', () => ({
  fetchWorkspaceRecipes: apiMocks.fetchWorkspaceRecipes,
  saveWorkspaceRecipe: apiMocks.saveWorkspaceRecipe,
  previewWorkspaceRecipe: apiMocks.previewWorkspaceRecipe,
  executeWorkspaceRecipe: apiMocks.executeWorkspaceRecipe,
}))

const builtinRecipe = {
  id: 'file-cleanup-target',
  name: '单目标清理',
  description: '先看目录摘要，再删除目标。',
  category: 'files-security',
  source: 'builtin',
  supports_dry_run: true,
  params: [
    {
      key: 'target',
      label: '删除目标',
      description: '要删除的文件或目录。',
      default_value: '',
      required: true,
      placeholder: 'D:/tmp/demo.log',
    },
  ],
  steps: [
    {
      kind: 'guarded',
      id: 'guarded-rm',
      title: '受保护删除',
      workspace: 'files-security',
      action: 'rm',
      target: '{{target}}',
      summary: '删除 {{target}}',
      preview_args: ['rm', '--dry-run', '-f', 'json', '{{target}}'],
      execute_args: ['rm', '-y', '-f', 'json', '{{target}}'],
      preview_summary: '删除 {{target}}',
    },
  ],
}

describe('RecipePanel', () => {
  beforeEach(() => {
    apiMocks.fetchWorkspaceRecipes.mockReset()
    apiMocks.saveWorkspaceRecipe.mockReset()
    apiMocks.previewWorkspaceRecipe.mockReset()
    apiMocks.executeWorkspaceRecipe.mockReset()
  })

  it('loads recipes and saves builtin copy to local store', async () => {
    apiMocks.fetchWorkspaceRecipes
      .mockResolvedValueOnce({ recipes: [builtinRecipe] })
      .mockResolvedValueOnce({
        recipes: [
          builtinRecipe,
          { ...builtinRecipe, id: 'file-cleanup-target-local', name: '单目标清理（本地副本）', source: 'custom' },
        ],
      })
    apiMocks.saveWorkspaceRecipe.mockResolvedValue({
      ...builtinRecipe,
      id: 'file-cleanup-target-local',
      name: '单目标清理（本地副本）',
      source: 'custom',
    })

    const wrapper = mount(RecipePanel)
    await flushPromises()
    await wrapper.get('[data-testid="save-recipe-button"]').trigger('click')
    await flushPromises()

    expect(apiMocks.saveWorkspaceRecipe).toHaveBeenCalledWith({
      ...builtinRecipe,
      id: 'file-cleanup-target-local',
      name: '单目标清理（本地副本）',
      source: 'custom',
    })
    expect(wrapper.text()).toContain('本地 1')
    expect(wrapper.text()).toContain('单目标清理（本地副本）')
  })

  it('previews recipe and executes only after explicit confirm', async () => {
    apiMocks.fetchWorkspaceRecipes.mockResolvedValue({ recipes: [builtinRecipe] })
    apiMocks.previewWorkspaceRecipe.mockResolvedValue({
      token: 'recipe-token-1',
      recipe_id: 'file-cleanup-target',
      recipe_name: '单目标清理',
      status: 'previewed',
      guarded: true,
      dry_run: true,
      ready_to_execute: true,
      summary: '已预演，共 1 步，可确认执行。',
      total_steps: 1,
      expires_in_secs: 300,
      steps: [
        {
          id: 'guarded-rm',
          title: '受保护删除',
          workspace: 'files-security',
          action: 'rm',
          target: 'D:/tmp/demo.log',
          status: 'previewed',
          guarded: true,
          dry_run: true,
          summary: '删除 D:/tmp/demo.log',
          process: {
            command_line: 'xun rm --dry-run -f json D:/tmp/demo.log',
            exit_code: 0,
            success: true,
            stdout: 'preview ok',
            stderr: '',
            duration_ms: 10,
          },
        },
      ],
    })
    apiMocks.executeWorkspaceRecipe.mockResolvedValue({
      token: 'recipe-token-1',
      recipe_id: 'file-cleanup-target',
      recipe_name: '单目标清理',
      status: 'succeeded',
      guarded: true,
      dry_run: false,
      summary: '单目标清理 执行完成。',
      total_steps: 1,
      completed_steps: 1,
      failed_step_id: null,
      audited_at: 1700000000,
      steps: [
        {
          id: 'guarded-rm',
          title: '受保护删除',
          workspace: 'files-security',
          action: 'rm',
          target: 'D:/tmp/demo.log',
          status: 'succeeded',
          guarded: true,
          dry_run: false,
          summary: '删除 D:/tmp/demo.log',
          audit_action: 'dashboard.task.execute.rm',
          process: {
            command_line: 'xun rm -y -f json D:/tmp/demo.log',
            exit_code: 0,
            success: true,
            stdout: 'deleted',
            stderr: '',
            duration_ms: 15,
          },
        },
      ],
    })

    const wrapper = mount(RecipePanel)
    await flushPromises()

    await wrapper.get('[data-testid="recipe-param-target"]').setValue('D:/tmp/demo.log')
    await wrapper.get('[data-testid="preview-recipe-button"]').trigger('click')
    await flushPromises()

    expect(apiMocks.previewWorkspaceRecipe).toHaveBeenCalledWith({
      recipe_id: 'file-cleanup-target',
      values: { target: 'D:/tmp/demo.log' },
    })
    expect(apiMocks.executeWorkspaceRecipe).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('预演结果')

    await wrapper.get('[data-testid="execute-recipe-button"]').trigger('click')
    await flushPromises()

    expect(apiMocks.executeWorkspaceRecipe).toHaveBeenCalledWith({ token: 'recipe-token-1', confirm: true })
    expect(wrapper.text()).toContain('执行回执')
    expect(wrapper.text()).toContain('deleted')
  })

  it('stays blocked when preview fails', async () => {
    apiMocks.fetchWorkspaceRecipes.mockResolvedValue({ recipes: [builtinRecipe] })
    apiMocks.previewWorkspaceRecipe.mockRejectedValue(new Error('400 Bad Request: preview failed'))

    const wrapper = mount(RecipePanel)
    await flushPromises()

    await wrapper.get('[data-testid="recipe-param-target"]').setValue('D:/tmp/demo.log')
    await wrapper.get('[data-testid="preview-recipe-button"]').trigger('click')
    await flushPromises()

    expect(apiMocks.previewWorkspaceRecipe).toHaveBeenCalledTimes(1)
    expect(apiMocks.executeWorkspaceRecipe).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('400 Bad Request: preview failed')
    expect(wrapper.find('[data-testid="execute-recipe-button"]').exists()).toBe(false)
  })
})
