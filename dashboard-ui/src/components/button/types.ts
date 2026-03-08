export type ButtonPreset = 'secondary' | 'primary' | 'danger' | 'ghost'
export type ButtonSize = 'sm' | 'md' | 'lg'

export interface ButtonProps {
  /** 预设样式 */
  preset?: ButtonPreset
  /** 尺寸 */
  size?: ButtonSize
  /** 仅图标模式（正方形按钮） */
  square?: boolean
  /** 禁用 */
  disabled?: boolean
  /** 加载中 */
  loading?: boolean
  /** 原生 type */
  type?: 'button' | 'submit' | 'reset'
}
