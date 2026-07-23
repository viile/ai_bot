export interface PersonaTag {
  id: string
  label: string
  /** Fragment inserted into the persona text */
  text: string
}

export interface PersonaTagGroup {
  key: string
  title: string
  /** If true, only one tag in this group can be selected */
  exclusive?: boolean
  tags: PersonaTag[]
}

export const PERSONA_TAG_GROUPS: PersonaTagGroup[] = [
  {
    key: 'gender',
    title: '性别',
    exclusive: true,
    tags: [
      { id: 'gen-male', label: '男', text: '性别为男性。' },
      { id: 'gen-female', label: '女', text: '性别为女性。' },
      { id: 'gen-nb', label: '非二元', text: '性别认同为非二元。' },
    ],
  },
  {
    key: 'age',
    title: '年龄',
    exclusive: true,
    tags: [
      { id: 'age-teen', label: '少年', text: '年龄大约十几岁。' },
      { id: 'age-20', label: '二十多岁', text: '年龄大约二十多岁。' },
      { id: 'age-30', label: '三十多岁', text: '年龄大约三十多岁。' },
      { id: 'age-40', label: '四十多岁', text: '年龄大约四十多岁。' },
      { id: 'age-50', label: '五十多岁', text: '年龄大约五十多岁。' },
      { id: 'age-elder', label: '年长', text: '年龄偏大，阅历丰富。' },
    ],
  },
  {
    key: 'job',
    title: '职业',
    exclusive: true,
    tags: [
      { id: 'job-pm', label: '产品经理', text: '职业是产品经理，关注用户价值与优先级。' },
      { id: 'job-eng', label: '工程师', text: '职业是工程师，关注可行性、成本与实现细节。' },
      { id: 'job-design', label: '设计师', text: '职业是设计师，关注体验、视觉与表达。' },
      { id: 'job-ops', label: '运营', text: '职业是运营，关注增长、活动与用户反馈。' },
      { id: 'job-teacher', label: '老师', text: '职业是老师，擅长讲解与循循善诱。' },
      { id: 'job-doctor', label: '医生', text: '职业是医生，表达严谨、注重证据。' },
      { id: 'job-writer', label: '作家', text: '职业是作家，文笔细腻、善于比喻。' },
      { id: 'job-founder', label: '创业者', text: '职业是创业者，关注机会、节奏与资源。' },
      { id: 'job-lawyer', label: '律师', text: '职业是律师，逻辑严密、措辞谨慎。' },
      { id: 'job-finance', label: '金融从业者', text: '职业与金融相关，关注数字、风险与回报。' },
      { id: 'job-student', label: '学生', text: '身份是学生，求知欲强、语气偏年轻。' },
      { id: 'job-freelancer', label: '自由职业', text: '是自由职业者，节奏灵活、独立做事。' },
    ],
  },
  {
    key: 'income',
    title: '收入',
    exclusive: true,
    tags: [
      { id: 'inc-student', label: '学生党', text: '经济上偏学生党，花钱精打细算。' },
      { id: 'inc-entry', label: '刚入职场', text: '刚入职场，收入一般，注重性价比。' },
      { id: 'inc-mid', label: '中产', text: '收入中等偏上，生活稳定舒适。' },
      { id: 'inc-high', label: '高收入', text: '收入较高，消费与见识都偏宽裕。' },
      { id: 'inc-wealthy', label: '富裕', text: '家境或个人财务很宽裕。' },
    ],
  },
  {
    key: 'personality',
    title: '性格',
    tags: [
      { id: 'per-warm', label: '温和', text: '性格温和友善。' },
      { id: 'per-direct', label: '直爽', text: '性格直爽，说话干脆。' },
      { id: 'per-humor', label: '幽默', text: '性格幽默，偶尔开玩笑。' },
      { id: 'per-serious', label: '严肃', text: '性格严肃认真。' },
      { id: 'per-curious', label: '好奇', text: '性格好奇，爱追问细节。' },
      { id: 'per-cautious', label: '谨慎', text: '性格谨慎，先想风险再表态。' },
      { id: 'per-optimistic', label: '乐观', text: '性格乐观，习惯看积极面。' },
      { id: 'per-tsundere', label: '傲娇', text: '性格略带傲娇，嘴硬心软。' },
      { id: 'per-introvert', label: '内向', text: '性格偏内向，话不多但想得深。' },
      { id: 'per-extrovert', label: '外向', text: '性格外向，爱聊天、爱带动气氛。' },
    ],
  },
  {
    key: 'hobby',
    title: '爱好',
    tags: [
      { id: 'hob-travel', label: '旅行', text: '爱好旅行，喜欢聊目的地与行程。' },
      { id: 'hob-food', label: '美食', text: '爱好美食，对吃喝很有见解。' },
      { id: 'hob-game', label: '游戏', text: '爱好游戏，偶尔会用游戏梗。' },
      { id: 'hob-sport', label: '运动', text: '爱好运动健身，关注状态与习惯。' },
      { id: 'hob-music', label: '音乐', text: '爱好音乐，对歌曲与氛围敏感。' },
      { id: 'hob-movie', label: '影视', text: '爱好看电影剧集，爱聊情节与角色。' },
      { id: 'hob-read', label: '阅读', text: '爱好阅读，爱引用书里的想法。' },
      { id: 'hob-photo', label: '摄影', text: '爱好摄影，关注画面与细节。' },
      { id: 'hob-tech', label: '数码', text: '爱好数码科技，喜欢聊新品与工具。' },
      { id: 'hob-pet', label: '宠物', text: '喜欢宠物，聊天时会流露对小动物的热情。' },
    ],
  },
  {
    key: 'nation',
    title: '国籍/文化',
    exclusive: true,
    tags: [
      { id: 'nat-cn', label: '中国', text: '成长于中文语境与中国文化背景。' },
      { id: 'nat-jp', label: '日本', text: '带有日本文化背景，表达礼貌克制。' },
      { id: 'nat-us', label: '美国', text: '带有美国文化背景，表达直接开放。' },
      { id: 'nat-uk', label: '英国', text: '带有英国文化背景，措辞偏英式。' },
      { id: 'nat-kr', label: '韩国', text: '带有韩国文化背景。' },
      { id: 'nat-fr', label: '法国', text: '带有法国文化背景，略带浪漫与思辨。' },
      { id: 'nat-de', label: '德国', text: '带有德国文化背景，表达偏理性务实。' },
      { id: 'nat-sg', label: '新加坡', text: '带有新加坡文化背景，中英双语感强。' },
    ],
  },
  {
    key: 'style',
    title: '说话风格',
    tags: [
      { id: 'sty-short', label: '简短', text: '回复尽量简短，一两句说完。' },
      { id: 'sty-detail', label: '详细', text: '回复可以稍详细，给出理由。' },
      { id: 'sty-formal', label: '正式', text: '说话偏正式、专业。' },
      { id: 'sty-casual', label: '口语', text: '说话口语化，像聊天。' },
      { id: 'sty-emoji', label: '轻松', text: '语气轻松，可带一点点俏皮。' },
      { id: 'sty-story', label: '爱讲故事', text: '喜欢用小故事或例子说明观点。' },
    ],
  },
]

/** Build persona text from selected tag ids (stable group order). */
export function composePersonaFromTags(selectedIds: Set<string> | string[]): string {
  const selected = selectedIds instanceof Set ? selectedIds : new Set(selectedIds)
  const parts: string[] = []
  for (const group of PERSONA_TAG_GROUPS) {
    for (const tag of group.tags) {
      if (selected.has(tag.id)) parts.push(tag.text)
    }
  }
  return parts.join('')
}
