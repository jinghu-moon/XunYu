type CsvValue = string | number | boolean | null | undefined

const pad2 = (value: number) => String(value).padStart(2, '0')

const timestampSlug = () => {
  const now = new Date()
  return `${now.getFullYear()}${pad2(now.getMonth() + 1)}${pad2(now.getDate())}_${pad2(now.getHours())}${pad2(now.getMinutes())}${pad2(now.getSeconds())}`
}

const escapeCsv = (value: CsvValue) => {
  const text = value == null ? '' : String(value)
  return `"${text.replace(/"/g, '""')}"`
}

export const toCsv = (headers: string[], rows: CsvValue[][]) => {
  const head = headers.map(escapeCsv).join(',')
  const body = rows.map(row => row.map(escapeCsv).join(',')).join('\n')
  return [head, body].filter(Boolean).join('\n')
}

export const downloadTextFile = (filename: string, content: string, mime: string) => {
  const blob = new Blob([content], { type: mime })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  document.body.appendChild(link)
  link.click()
  link.remove()
  URL.revokeObjectURL(url)
}

export const downloadCsv = (prefix: string, headers: string[], rows: CsvValue[][]) => {
  const csv = toCsv(headers, rows)
  const filename = `${prefix}_${timestampSlug()}.csv`
  downloadTextFile(filename, csv, 'text/csv;charset=utf-8')
}

export const downloadJson = (prefix: string, data: unknown) => {
  const json = JSON.stringify(data, null, 2)
  const filename = `${prefix}_${timestampSlug()}.json`
  downloadTextFile(filename, json, 'application/json;charset=utf-8')
}
