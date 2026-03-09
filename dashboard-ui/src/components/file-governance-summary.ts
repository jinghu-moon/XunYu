import type { TaskProcessOutput } from '../types'

import type { TaskFormState, WorkspaceTaskDefinition } from '../workspace-tools'



export interface GovernanceSummaryItem {

  label: string

  value: string

}



export interface GovernanceSummaryModel {

  title: string

  note?: string

  items: GovernanceSummaryItem[]

}



function readText(values: TaskFormState, key: string): string {

  const value = values[key]

  return typeof value === 'string' ? value.trim() : ''

}



function readBool(values: TaskFormState, key: string): boolean {

  return values[key] === true

}



function splitItems(raw: string): string[] {

  return raw

    .split(/[\n,]+/)

    .map((item) => item.trim())

    .filter(Boolean)

}



function normalizePath(path: string): string {

  return path.trim().replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()

}



function joinValues(values: string[]): string {

  return values.length ? values.join(' / ') : '-'

}



function parseJson<T>(raw: string): T | null {

  if (!raw.trim()) return null

  try {

    return JSON.parse(raw) as T

  } catch {

    return null

  }

}



function deriveEncryptOutput(path: string, out: string): string {

  if (out) return out

  if (!path) return '-'

  const normalized = path.replace(/\\/g, '/')

  const idx = normalized.lastIndexOf('/')

  if (idx < 0) return `${normalized}.age`

  return `${normalized.slice(0, idx + 1)}${normalized.slice(idx + 1)}.age`

}



function deriveDecryptOutput(path: string, out: string): string {

  if (out) return out

  if (!path) return '-'

  const normalized = path.replace(/\\/g, '/')

  if (normalized.toLowerCase().endsWith('.age')) {

    return normalized.slice(0, -4)

  }

  return `${normalized}.decrypted`

}



function parseProtectRules(raw: string): Array<{ path: string; deny: string[]; require: string[] }> {

  const parsed = parseJson<Array<{ path?: string; deny?: string[]; require?: string[] }>>(raw)

  if (!parsed) return []

  return parsed

    .filter((item) => typeof item?.path === 'string')

    .map((item) => ({

      path: item.path ?? '',

      deny: Array.isArray(item.deny) ? item.deny.filter((value) => typeof value === 'string') : [],

      require: Array.isArray(item.require) ? item.require.filter((value) => typeof value === 'string') : [],

    }))

}



function parseAclSummary(raw: string): {

  owner?: string

  inherit?: string

  total?: number

  allow?: number

  deny?: number

  explicit?: number

  inherited?: number

  orphan?: number

} {

  const ownerMatch = raw.match(/Owner:\s*(.+?)\s+\|\s+Inherit:\s*(.+)/)

  const totalMatch = raw.match(

    /Total:\s*(\d+)\s*\(Allow\s*(\d+)\s*\/\s*Deny\s*(\d+)\)\s*Explicit\s*(\d+)\s*Inherited\s*(\d+)\s*Orphan\s*(\d+)/,

  )

  return {

    owner: ownerMatch?.[1]?.trim(),

    inherit: ownerMatch?.[2]?.trim(),

    total: totalMatch ? Number(totalMatch[1]) : undefined,

    allow: totalMatch ? Number(totalMatch[2]) : undefined,

    deny: totalMatch ? Number(totalMatch[3]) : undefined,

    explicit: totalMatch ? Number(totalMatch[4]) : undefined,

    inherited: totalMatch ? Number(totalMatch[5]) : undefined,

    orphan: totalMatch ? Number(totalMatch[6]) : undefined,

  }

}



function parseAclDiffSummary(raw: string): {

  reference?: string

  ownerDiff: boolean

  inheritDiff: boolean

  onlyInA?: number

  onlyInB?: number

  common?: number

  exportedRows?: number

  exportPath?: string

} {

  const referenceMatch = raw.match(/Reference:\s*(.+)/)

  const onlyInAMatch = raw.match(/Only in A:\s*(\d+)/)

  const onlyInBMatch = raw.match(/Only in B:\s*(\d+)/)

  const commonMatch = raw.match(/Common:\s*(\d+)/)

  const exportMatch = raw.match(/Exported\s+(\d+)\s+rows\s+to\s+(.+)/)

  return {

    reference: referenceMatch?.[1]?.trim(),

    ownerDiff: raw.includes('Owner differs'),

    inheritDiff: raw.includes('Inheritance differs'),

    onlyInA: onlyInAMatch ? Number(onlyInAMatch[1]) : undefined,

    onlyInB: onlyInBMatch ? Number(onlyInBMatch[1]) : undefined,

    common: commonMatch ? Number(commonMatch[1]) : undefined,

    exportedRows: exportMatch ? Number(exportMatch[1]) : undefined,

    exportPath: exportMatch?.[2]?.trim(),

  }

}



function parseFindDecision(raw: string): { decision?: string; source?: string } {

  const match = raw.match(/Decision:\s*(INCLUDE|EXCLUDE)\s*\(source:\s*([^\)]+)\)/i)

  return {

    decision: match?.[1]?.toUpperCase(),

    source: match?.[2]?.trim(),

  }

}



function parseBackupResult(raw: string): { entries?: number; output?: string } {

  const match = raw.match(/Backed up\s+(\d+)\s+entries\s*->\s*(.+)/)

  return {

    entries: match ? Number(match[1]) : undefined,

    output: match?.[2]?.trim(),

  }

}



function parsePurgeResult(raw: string): { removed?: number; principal?: string } {

  const match = raw.match(/Removed\s+(\d+)\s+entries\s+for\s+(.+)/)

  return {

    removed: match ? Number(match[1]) : undefined,

    principal: match?.[2]?.trim(),

  }

}



function parseRepairExport(raw: string): { count?: number; output?: string } {

  const match = raw.match(/Exported\s+(\d+)\s+errors\s+to\s+(.+)/)

  return {

    count: match ? Number(match[1]) : undefined,

    output: match?.[2]?.trim(),

  }

}



function buildProtectSummary(

  action: 'protect:set' | 'protect:clear',

  phase: 'preview' | 'execute',

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const rules = parseProtectRules(raw)

  const exactRule = rules.find((rule) => normalizePath(rule.path) === normalizePath(target))

  const isSet = action === 'protect:set'

  const items: GovernanceSummaryItem[] = [{ label: '治理对象', value: target || '-' }]



  if (phase === 'preview') {

    items.push({ label: '当前命中规则', value: `${rules.length} 条` })

    items.push({

      label: '预期变更',

      value: isSet

        ? exactRule

          ? '更新现有保护规则'

          : '新增保护规则'

        : exactRule

          ? '移除现有保护规则'

          : '未命中精确规则，执行后可能无变化',

    })

  } else {

    items.push({

      label: '执行结果',

      value: process.success

        ? isSet

          ? '保护规则已写入'

          : raw.includes('No protection rule found')

            ? '未找到可清除的规则'

            : '保护规则已清除'

        : '执行失败',

    })

  }



  if (isSet) {

    items.push({ label: '拒绝动作', value: joinValues(splitItems(readText(form, 'deny'))) })

    items.push({ label: '绕过要求', value: joinValues(splitItems(readText(form, 'require'))) })

  }



  items.push({ label: '同步系统 ACL', value: readBool(form, 'systemAcl') ? '是' : '否' })



  return {

    title: phase === 'preview' ? '保护变更预演摘要' : '保护变更执行摘要',

    items,

  }

}



function buildAclAddSummary(

  phase: 'preview' | 'execute',

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const summary = parseAclSummary(raw)

  const items: GovernanceSummaryItem[] = [

    { label: '治理对象', value: target || '-' },

    { label: '主体', value: readText(form, 'principal') || '-' },

    { label: '权限', value: readText(form, 'rights') || 'Read' },

    { label: '类型', value: readText(form, 'aceType') || 'Allow' },

    { label: '继承', value: readText(form, 'inherit') || 'BothInherit' },

  ]



  if (phase === 'preview') {

    items.push({ label: '当前 Owner', value: summary.owner || '-' })

    items.push({ label: '继承状态', value: summary.inherit || '-' })

    items.push({ label: '现有 ACE', value: summary.total !== undefined ? `${summary.total} 条` : '-' })

    items.push({

      label: '显式 / 继承',

      value:

        summary.explicit !== undefined && summary.inherited !== undefined

          ? `${summary.explicit} / ${summary.inherited}`

          : '-',

    })

  } else {

    items.push({ label: '执行结果', value: process.success ? 'ACL 规则已添加' : '执行失败' })

  }



  return {

    title: phase === 'preview' ? 'ACL 变更预演摘要' : 'ACL 变更执行摘要',

    items,

  }

}



function buildAclDiffSummary(

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const summary = parseAclDiffSummary(raw)

  const output = readText(form, 'output')

  const items: GovernanceSummaryItem[] = [

    { label: '治理对象', value: target || '-' },

    { label: '参考路径', value: readText(form, 'reference') || summary.reference || '-' },

    { label: '仅目标侧', value: summary.onlyInA !== undefined ? `${summary.onlyInA} 条` : '-' },

    { label: '仅参考侧', value: summary.onlyInB !== undefined ? `${summary.onlyInB} 条` : '-' },

    { label: '共同 ACE', value: summary.common !== undefined ? `${summary.common} 条` : '-' },

    { label: 'Owner 差异', value: summary.ownerDiff ? '有差异' : '无差异' },

    { label: '继承差异', value: summary.inheritDiff ? '有差异' : '无差异' },

  ]



  if (output || summary.exportPath) {

    items.push({ label: '导出路径', value: output || summary.exportPath || '-' })

  }



  if (summary.exportedRows !== undefined) {

    items.push({ label: '导出行数', value: `${summary.exportedRows} 行` })

  }



  return {

    title: 'ACL 差异摘要',

    note: '上方展示 ACL 差异统计；如接口返回结构化 details，下方会同步渲染 ACE 级差异明细。',

    items,

  }

}





function buildAclEffectiveSummary(target: string, form: TaskFormState, process: TaskProcessOutput): GovernanceSummaryModel {

  const user = readText(form, 'user')



  return {

    title: 'ACL 有效权限摘要',

    note: process.success ? undefined : '有效权限查询失败，请结合原始输出定位主体、继承或拒绝规则。',

    items: [

      { label: '治理对象', value: target || '-' },

      { label: '查询用户', value: user || '当前用户' },

      { label: '执行结果', value: process.success ? '有效权限已返回' : '查询失败' },

    ],

  }

}



function buildAclBackupSummary(target: string, form: TaskFormState, process: TaskProcessOutput): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const summary = parseBackupResult(raw)

  const items: GovernanceSummaryItem[] = [

    { label: '治理对象', value: target || '-' },

    { label: '输出文件', value: readText(form, 'output') || summary.output || '-' },

    { label: '备份条目', value: summary.entries !== undefined ? `${summary.entries} 条` : '-' },

    { label: '执行结果', value: process.success ? 'ACL 备份已导出' : '执行失败' },

  ]



  return {

    title: 'ACL 备份摘要',

    items,

  }

}



function buildAclCopySummary(

  phase: 'preview' | 'execute',

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  if (phase === 'preview') {

    const diffSummary = buildAclDiffSummary(target, form, process)

    return {

      title: 'ACL 覆盖预演摘要',

      note: '预演阶段通过 acl diff 对比目标与参考 ACL；确认执行后会整体覆盖目标 ACL。',

      items: diffSummary.items,

    }

  }



  return {

    title: 'ACL 覆盖执行摘要',

    items: [

      { label: '治理对象', value: target || '-' },

      { label: '参考路径', value: readText(form, 'reference') || '-' },

      { label: '执行结果', value: process.success ? '目标 ACL 已覆盖' : '执行失败' },

    ],

  }

}



function buildAclRestoreSummary(

  phase: 'preview' | 'execute',

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const decision = parseFindDecision(raw)

  const items: GovernanceSummaryItem[] = [

    { label: '治理对象', value: target || '-' },

    { label: '备份文件', value: readText(form, 'from') || '-' },

  ]



  if (phase === 'preview') {

    items.push({

      label: '规则测试',

      value: decision.decision ? `${decision.decision} (${decision.source || 'unknown'})` : '未解析到决策结果',

    })

  } else {

    items.push({ label: '执行结果', value: process.success ? 'ACL 已恢复' : '执行失败' })

  }



  return {

    title: phase === 'preview' ? 'ACL 恢复预演摘要' : 'ACL 恢复执行摘要',

    note:
      phase === 'preview'
        ? 'CLI 预演只验证备份文件路径，Dashboard 会额外读取备份快照推导预期 ACL，但不会写回目标。'
        : undefined,

    items,

  }

}



function buildAclPurgeSummary(

  phase: 'preview' | 'execute',

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const summary = parseAclSummary(raw)

  const purgeResult = parsePurgeResult(raw)

  const items: GovernanceSummaryItem[] = [

    { label: '治理对象', value: target || '-' },

    { label: '清理主体', value: readText(form, 'principal') || purgeResult.principal || '-' },

  ]



  if (phase === 'preview') {

    items.push({ label: '当前 Owner', value: summary.owner || '-' })

    items.push({ label: '现有 ACE', value: summary.total !== undefined ? `${summary.total} 条` : '-' })

    items.push({

      label: '显式 / 继承',

      value:

        summary.explicit !== undefined && summary.inherited !== undefined

          ? `${summary.explicit} / ${summary.inherited}`

          : '-',

    })

  } else {

    items.push({

      label: '移除条目',

      value: purgeResult.removed !== undefined ? `${purgeResult.removed} 条` : process.success ? '已执行' : '-',

    })

    items.push({ label: '执行结果', value: process.success ? '显式 ACL 已清理' : '执行失败' })

  }



  return {

    title: phase === 'preview' ? 'ACL 清理预演摘要' : 'ACL 清理执行摘要',

    items,

  }

}



function buildAclInheritSummary(

  phase: 'preview' | 'execute',

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const summary = parseAclSummary(raw)

  const mode = readText(form, 'mode') || 'enable'

  const preserveLabel = mode === 'disable' ? (readBool(form, 'preserve') ? '保留继承 ACE' : '移除继承 ACE') : '-'

  const items: GovernanceSummaryItem[] = [

    { label: '治理对象', value: target || '-' },

    { label: '目标状态', value: mode === 'enable' ? '启用继承' : '禁用继承' },

    { label: '禁用策略', value: preserveLabel },

  ]



  if (phase === 'preview') {

    items.push({ label: '当前继承', value: summary.inherit || '-' })

    items.push({ label: '当前 Owner', value: summary.owner || '-' })

  } else {

    items.push({ label: '执行结果', value: process.success ? '继承状态已切换' : '执行失败' })

  }



  return {

    title: phase === 'preview' ? 'ACL 继承预演摘要' : 'ACL 继承执行摘要',

    items,

  }

}



function buildAclOwnerSummary(

  phase: 'preview' | 'execute',

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const summary = parseAclSummary(raw)

  const items: GovernanceSummaryItem[] = [

    { label: '治理对象', value: target || '-' },

    { label: '新 Owner', value: readText(form, 'set') || '-' },

  ]



  if (phase === 'preview') {

    items.push({ label: '当前 Owner', value: summary.owner || '-' })

    items.push({ label: '当前继承', value: summary.inherit || '-' })

  } else {

    items.push({ label: '执行结果', value: process.success ? 'Owner 已更新' : '执行失败' })

  }



  return {

    title: phase === 'preview' ? 'ACL Owner 预演摘要' : 'ACL Owner 执行摘要',

    items,

  }

}



function buildAclRepairSummary(

  phase: 'preview' | 'execute',

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const summary = parseAclSummary(raw)

  const exportInfo = parseRepairExport(raw)

  const items: GovernanceSummaryItem[] = [

    { label: '治理对象', value: target || '-' },

    { label: '导出失败明细', value: readBool(form, 'exportErrors') ? '是' : '否' },

  ]



  if (phase === 'preview') {

    items.push({ label: '当前 Owner', value: summary.owner || '-' })

    items.push({ label: '当前继承', value: summary.inherit || '-' })

    items.push({ label: '现有 ACE', value: summary.total !== undefined ? `${summary.total} 条` : '-' })

  } else {

    items.push({ label: '执行结果', value: process.success ? 'ACL 强制修复完成' : '修复存在失败' })

    if (exportInfo.output) {

      items.push({ label: '错误导出', value: exportInfo.output })

    }

    if (exportInfo.count !== undefined) {

      items.push({ label: '错误数量', value: `${exportInfo.count} 条` })

    }

  }



  return {

    title: phase === 'preview' ? 'ACL 修复预演摘要' : 'ACL 修复执行摘要',

    note: '强制修复会接管所有权并授予 FullControl，属于高风险治理动作。',

    items,

  }

}



function buildCryptSummary(

  action: 'encrypt' | 'decrypt',

  phase: 'preview' | 'execute',

  target: string,

  form: TaskFormState,

  process: TaskProcessOutput,

): GovernanceSummaryModel {

  const raw = process.stdout || process.stderr || ''

  const isEncrypt = action === 'encrypt'

  const efs = readBool(form, 'efs')

  const recipients = splitItems(readText(form, 'to'))

  const identities = splitItems(readText(form, 'identity'))

  const out = readText(form, 'out')

  const decision = parseFindDecision(raw)



  const mode = efs

    ? 'Windows EFS'

    : isEncrypt

      ? recipients.length

        ? 'age 收件人'

        : '配置不完整'

      : identities.length

        ? 'age 身份文件'

        : '配置不完整'



  const items: GovernanceSummaryItem[] = [

    { label: '治理对象', value: target || '-' },

    { label: '执行模式', value: mode },

    {

      label: '输出路径',

      value: isEncrypt ? deriveEncryptOutput(target, out) : deriveDecryptOutput(target, out),

    },

  ]



  if (isEncrypt && !efs) {

    items.push({ label: '收件公钥', value: recipients.length ? `${recipients.length} 个` : '未提供' })

  }



  if (!isEncrypt && !efs) {

    items.push({ label: '身份文件', value: identities.length ? `${identities.length} 个` : '未提供' })

  }



  if (phase === 'preview') {

    items.push({

      label: '规则测试',

      value: decision.decision ? `${decision.decision} (${decision.source || 'unknown'})` : '未解析到决策结果',

    })

  } else {

    items.push({ label: '执行结果', value: process.success ? (isEncrypt ? '加密完成' : '解密完成') : '执行失败' })

  }



  let note = ''

  if (phase === 'preview') {

    note = '当前预演只执行规则测试，不会真正改写文件内容。'

  }

  if (!efs && isEncrypt && recipients.length === 0) {

    note = '当前表单未提供 EFS 或收件公钥，真正执行时会被 CLI 拒绝。'

  }

  if (!efs && !isEncrypt && identities.length === 0) {

    note = '当前表单未提供 EFS 或身份文件，真正执行时会被 CLI 拒绝。'

  }



  return {

    title: phase === 'preview' ? `${isEncrypt ? '加密' : '解密'}预演摘要` : `${isEncrypt ? '加密' : '解密'}执行摘要`,

    note: note || undefined,

    items,

  }

}



export function buildFileGovernanceSummary(

  task: Pick<WorkspaceTaskDefinition, 'workspace' | 'action'>,

  form: TaskFormState,

  phase: 'preview' | 'execute',

  process: TaskProcessOutput,

  target: string,

): GovernanceSummaryModel | null {

  if (task.workspace !== 'files-security') return null



  switch (task.action) {

    case 'protect:set':

    case 'protect:clear':

      return buildProtectSummary(task.action, phase, target, form, process)

    case 'acl:add':

      return buildAclAddSummary(phase, target, form, process)

    case 'acl:diff':

      return buildAclDiffSummary(target, form, process)

    case 'acl:effective':

      return buildAclEffectiveSummary(target, form, process)

    case 'acl:backup':

      return buildAclBackupSummary(target, form, process)

    case 'acl:copy':

      return buildAclCopySummary(phase, target, form, process)

    case 'acl:restore':

      return buildAclRestoreSummary(phase, target, form, process)

    case 'acl:purge':

      return buildAclPurgeSummary(phase, target, form, process)

    case 'acl:inherit':

      return buildAclInheritSummary(phase, target, form, process)

    case 'acl:owner':

      return buildAclOwnerSummary(phase, target, form, process)

    case 'acl:repair':

      return buildAclRepairSummary(phase, target, form, process)

    case 'encrypt':

    case 'decrypt':

      return buildCryptSummary(task.action, phase, target, form, process)

    default:

      return null

  }

}

