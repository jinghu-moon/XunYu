const CATEGORY_KEYS = ['lang', 'tool', 'env', 'work', 'path', 'general'] as const
type TagCategory = (typeof CATEGORY_KEYS)[number]

const KEYWORDS: Record<TagCategory, string[]> = {
  lang: ['lang', 'language', 'rust', 'go', 'python', 'js', 'ts', 'node', 'java', 'c#', 'c++', 'cpp'],
  tool: ['tool', 'cli', 'git', 'docker', 'k8s', 'npm', 'pnpm', 'cargo', 'pip', 'brew'],
  env: ['env', 'dev', 'prod', 'test', 'stage', 'staging'],
  work: ['work', 'proj', 'project', 'client', 'team'],
  path: ['path', 'doc', 'docs', 'note', 'readme', 'ref'],
  general: [],
}

function hashString(input: string): number {
  let h = 0
  for (let i = 0; i < input.length; i += 1) {
    h = (h * 31 + input.charCodeAt(i)) | 0
  }
  return Math.abs(h)
}

export function resolveTagCategory(tag: string): TagCategory {
  const raw = tag.trim().toLowerCase()
  if (!raw) return 'general'
  const prefix = raw.split(':')[0]
  if (CATEGORY_KEYS.includes(prefix as TagCategory)) return prefix as TagCategory
  for (const key of CATEGORY_KEYS) {
    const list = KEYWORDS[key]
    if (list.length && list.some(word => raw.includes(word))) return key
  }
  const idx = hashString(raw) % CATEGORY_KEYS.length
  return CATEGORY_KEYS[idx]
}

export function tagCategoryClass(tag: string): string {
  return `tag-pill--${resolveTagCategory(tag)}`
}
