/** Split message text into plain text / @mention segments for display. */
export type MentionPart = { type: 'text' | 'mention'; value: string }

/** Special tokens that mean “everyone in the group”. */
export const MENTION_EVERYONE_LABEL = '所有人'
export const MENTION_EVERYONE_ALIASES = ['所有人', '全体成员', 'all'] as const

/** Longest-first so 「产品经理小周」不会被「产品」截断. */
export function sortNamesLongestFirst(names: string[]): string[] {
  return [...names]
    .filter((n) => n.trim().length > 0)
    .sort((a, b) => b.length - a.length || a.localeCompare(b, 'en', { sensitivity: 'base' }))
}

export function mentionDisplayNames(botNicknames: string[], userNickname?: string | null): string[] {
  const me = (userNickname || '我').trim() || '我'
  const extras = me === '我' ? [me] : [me, '我']
  return sortNamesLongestFirst([...MENTION_EVERYONE_ALIASES, ...extras, ...botNicknames])
}

export function splitMentions(content: string, knownNames: string[]): MentionPart[] {
  if (!content) return []
  const names = sortNamesLongestFirst(knownNames)
  if (!names.length) return [{ type: 'text', value: content }]

  const parts: MentionPart[] = []
  let i = 0
  while (i < content.length) {
    if (content[i] === '@') {
      let matched: string | null = null
      for (const name of names) {
        const slice = content.slice(i + 1, i + 1 + name.length)
        const same =
          slice === name || (name.toLowerCase() === 'all' && slice.toLowerCase() === 'all')
        if (same) {
          matched = name.toLowerCase() === 'all' ? slice : name
          break
        }
      }
      if (matched) {
        parts.push({ type: 'mention', value: matched })
        i += matched.length + 1
        continue
      }
    }
    const nextAt = content.indexOf('@', i + 1)
    const end = nextAt === -1 ? content.length : nextAt
    const chunk = content.slice(i, end)
    if (chunk) parts.push({ type: 'text', value: chunk })
    i = end
  }
  return parts
}

/** Active `@query` at caret for autocomplete. */
export function mentionQueryAt(
  text: string,
  caret: number,
): { start: number; query: string } | null {
  const before = text.slice(0, caret)
  const m = before.match(/@([^\s@]*)$/)
  if (!m || m.index === undefined) return null
  return { start: m.index, query: m[1] ?? '' }
}

export function insertMention(
  text: string,
  caret: number,
  start: number,
  nickname: string,
): { text: string; caret: number } {
  const after = text.slice(caret)
  const inserted = `@${nickname} `
  const next = text.slice(0, start) + inserted + after
  return { text: next, caret: start + inserted.length }
}

export function everyoneMentionMatchesQuery(query: string): boolean {
  const raw = query.trim()
  const q = raw.toLowerCase()
  if (!q) return true
  return '所有人'.includes(raw) || '全体成员'.includes(raw) || 'all'.startsWith(q)
}
