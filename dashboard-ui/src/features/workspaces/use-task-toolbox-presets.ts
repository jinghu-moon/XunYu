import { nextTick, ref } from 'vue'

export function useTaskToolboxPresets<TPresetMap>() {
  const toolboxAnchor = ref<HTMLElement | null>(null)
  const taskPresets = ref<TPresetMap | null>(null)
  const presetVersion = ref(0)

  async function applyTaskPresets(presets: TPresetMap) {
    taskPresets.value = presets
    presetVersion.value += 1
    await nextTick()
    toolboxAnchor.value?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
  }

  return {
    toolboxAnchor,
    taskPresets,
    presetVersion,
    applyTaskPresets,
  }
}
