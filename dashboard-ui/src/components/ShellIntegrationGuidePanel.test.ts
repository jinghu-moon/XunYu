import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ShellIntegrationGuidePanel from './ShellIntegrationGuidePanel.vue'

describe('ShellIntegrationGuidePanel', () => {
  const writeText = vi.fn<(_: string) => Promise<void>>()

  beforeEach(() => {
    writeText.mockReset()
    writeText.mockResolvedValue(undefined)
    Object.defineProperty(globalThis.navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('renders profile guidance and copies commands for the active shell', async () => {
    const wrapper = mount(ShellIntegrationGuidePanel)

    expect(wrapper.get('[data-testid="shell-guide-profile-path"]').text()).toBe('$PROFILE')
    expect(wrapper.get('[data-testid="shell-guide-profile-command"]').text()).toContain('xun init powershell')

    await wrapper.get('[data-testid="shell-guide-shell-bash"]').trigger('click')
    expect(wrapper.get('[data-testid="shell-guide-profile-path"]').text()).toBe('~/.bashrc')
    expect(wrapper.get('[data-testid="shell-guide-completion-command"]').text()).toBe('xun completion bash')

    await wrapper.get('[data-testid="shell-guide-copy-profile"]').trigger('click')
    expect(writeText).toHaveBeenCalledWith('eval "$(xun init bash)"')
    expect(wrapper.get('[data-testid="shell-guide-copy-feedback"]').text()).toContain('已复制')
  })

  it('emits task presets for init, completion and complete verification', async () => {
    const wrapper = mount(ShellIntegrationGuidePanel)

    await wrapper.get('[data-testid="shell-guide-shell-zsh"]').trigger('click')
    await wrapper.get('[data-testid="shell-guide-apply-presets"]').trigger('click')

    expect(wrapper.emitted('apply-task-presets')).toEqual([
      [
        {
          init: { shell: 'zsh' },
          completion: { shell: 'zsh' },
          complete: { args: 'alias ls --j' },
        },
      ],
    ])
  })
})
