import type {
  TaskFieldType,
  TaskFieldValue,
  TaskFormState,
  TaskFieldOption,
  TaskFieldDefinition,
  TaskNoticeTone,
  TaskNotice,
  WorkspaceTaskDefinition,
  WorkspaceTaskGroup,
  WorkspaceTabDefinition
} from './catalog-shared'

import {
  JSON_FORMAT,
  desktopWindowNotices,
  desktopHostsNotices,
  desktopColorNotices,
  shellInitOptions,
  shellCompletionOptions,
  aliasTypeOptions,
  aliasModeOptions,
  aliasShellOptions,
  dedupModeOptions,
  brnCaseOptions,
  imgFormatOptions,
  imgSvgMethodOptions,
  imgJpegBackendOptions,
  aliasAppScanSourceOptions,
  videoModeOptions,
  videoEngineOptions,
  aclRightsOptions,
  aclAceTypeOptions,
  aclInheritOptions,
  aclInheritModeOptions,
  readText,
  readBool,
  splitItems,
  splitCommand,
  pushOption,
  pushRepeatableOption,
  runTask,
  guardedTask,
  pathTarget,
  previewPath,
  moveLikeArgs
} from './catalog-shared'

export const mediaConversionTaskGroups: WorkspaceTaskGroup[] = [
  {
    id: 'image-tools',
    title: '图像处理',
    description: 'img 统一以任务卡形式接入，并暴露高级编码参数。',
    tasks: [
      runTask({
        id: 'img',
        workspace: 'media-conversion',
        title: '图像转换',
        description: '压缩或转换图片目录。',
        action: 'img',
        feature: 'img',
        fields: [
          { key: 'input', label: '输入', type: 'text', required: true },
          { key: 'output', label: '输出目录', type: 'text', required: true },
          { key: 'format', label: '格式', type: 'select', defaultValue: 'webp', options: imgFormatOptions },
          { key: 'quality', label: '质量', type: 'number', defaultValue: '80', min: 1, max: 100 },
          { key: 'svg_method', label: 'SVG 方法', type: 'select', defaultValue: 'bezier', options: imgSvgMethodOptions },
          { key: 'svg_diffvg_iters', label: 'DiffVG 迭代', type: 'number', defaultValue: '150' },
          { key: 'svg_diffvg_strokes', label: 'DiffVG 线条数', type: 'number', defaultValue: '64' },
          { key: 'jpeg_backend', label: 'JPEG 后端', type: 'select', defaultValue: 'auto', options: imgJpegBackendOptions },
          { key: 'png_lossy', label: 'PNG 有损量化', type: 'checkbox', defaultValue: true },
          { key: 'png_dither_level', label: 'PNG 抖动级别', type: 'number', defaultValue: '0.0' },
          { key: 'webp_lossy', label: 'WebP 有损模式', type: 'checkbox', defaultValue: true },
          { key: 'mw', label: '最大宽度', type: 'number', placeholder: '可选' },
          { key: 'mh', label: '最大高度', type: 'number', placeholder: '可选' },
          { key: 'threads', label: '工作线程', type: 'number', placeholder: '可选' },
          { key: 'avif_threads', label: 'AVIF 内部线程', type: 'number', placeholder: '可选' },
          { key: 'overwrite', label: '覆盖输出', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'input'),
        buildRunArgs: (values) => {
          const args = [
            'img',
            '-i',
            readText(values, 'input'),
            '-o',
            readText(values, 'output'),
            '-f',
            readText(values, 'format') || 'webp',
            '-q',
            readText(values, 'quality') || '80',
            '--svg-method',
            readText(values, 'svg_method') || 'bezier',
            '--svg-diffvg-iters',
            readText(values, 'svg_diffvg_iters') || '150',
            '--svg-diffvg-strokes',
            readText(values, 'svg_diffvg_strokes') || '64',
            '--jpeg-backend',
            readText(values, 'jpeg_backend') || 'auto',
            '--png-lossy',
            readBool(values, 'png_lossy') ? 'true' : 'false',
            '--png-dither-level',
            readText(values, 'png_dither_level') || '0.0',
            '--webp-lossy',
            readBool(values, 'webp_lossy') ? 'true' : 'false',
          ]
          pushOption(args, '--mw', readText(values, 'mw'))
          pushOption(args, '--mh', readText(values, 'mh'))
          pushOption(args, '-t', readText(values, 'threads'))
          pushOption(args, '--avif-threads', readText(values, 'avif_threads'))
          if (readBool(values, 'overwrite')) args.push('--overwrite')
          return args
        },
      }),
    ],
  },
  {
    id: 'video-tools',
    title: '视频处理',
    description: 'probe / compress / remux 全部在本地控制台内完成。',
    tasks: [
      runTask({
        id: 'video-probe',
        workspace: 'media-conversion',
        title: '视频探测',
        description: '读取媒体元数据。',
        action: 'video:probe',
        fields: [
          { key: 'input', label: '输入文件', type: 'text', required: true },
          { key: 'ffprobe', label: 'ffprobe 路径', type: 'text', placeholder: '可选' },
        ],
        target: (values) => readText(values, 'input'),
        buildRunArgs: (values) => {
          const args = ['video', 'probe', '-i', readText(values, 'input')]
          pushOption(args, '--ffprobe', readText(values, 'ffprobe'))
          return args
        },
      }),
      runTask({
        id: 'video-compress',
        workspace: 'media-conversion',
        title: '视频压缩',
        description: '按 mode / engine 做转码压缩。',
        action: 'video:compress',
        fields: [
          { key: 'input', label: '输入文件', type: 'text', required: true },
          { key: 'output', label: '输出文件', type: 'text', required: true },
          { key: 'mode', label: '模式', type: 'select', defaultValue: 'balanced', options: videoModeOptions },
          { key: 'engine', label: '引擎', type: 'select', defaultValue: 'auto', options: videoEngineOptions },
          { key: 'ffmpeg', label: 'ffmpeg 路径', type: 'text', placeholder: '可选' },
          { key: 'overwrite', label: '覆盖输出', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'input'),
        buildRunArgs: (values) => {
          const args = ['video', 'compress', '-i', readText(values, 'input'), '-o', readText(values, 'output'), '--mode', readText(values, 'mode') || 'balanced', '--engine', readText(values, 'engine') || 'auto']
          pushOption(args, '--ffmpeg', readText(values, 'ffmpeg'))
          if (readBool(values, 'overwrite')) args.push('--overwrite')
          return args
        },
      }),
      runTask({
        id: 'video-remux',
        workspace: 'media-conversion',
        title: '无损封装转换',
        description: '做 remux 容器迁移。',
        action: 'video:remux',
        fields: [
          { key: 'input', label: '输入文件', type: 'text', required: true },
          { key: 'output', label: '输出文件', type: 'text', required: true },
          { key: 'strict', label: '严格模式', type: 'checkbox', defaultValue: true },
          { key: 'ffmpeg', label: 'ffmpeg 路径', type: 'text', placeholder: '可选' },
          { key: 'ffprobe', label: 'ffprobe 路径', type: 'text', placeholder: '可选' },
          { key: 'overwrite', label: '覆盖输出', type: 'checkbox', defaultValue: false },
        ],
        target: (values) => readText(values, 'input'),
        buildRunArgs: (values) => {
          const args = ['video', 'remux', '-i', readText(values, 'input'), '-o', readText(values, 'output'), '--strict', readBool(values, 'strict') ? 'true' : 'false']
          pushOption(args, '--ffmpeg', readText(values, 'ffmpeg'))
          pushOption(args, '--ffprobe', readText(values, 'ffprobe'))
          if (readBool(values, 'overwrite')) args.push('--overwrite')
          return args
        },
      }),
    ],
  },
]
