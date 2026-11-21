import { useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from 'react'
import { Search, Download, Clipboard, Archive, Settings, Wrench, Puzzle, Plus, Folder, FileText, Link, Check, ChevronDown, ChevronRight, Copy, MoreHorizontal, Pin, BookOpen, Globe, Play, Pause, Image, Command, Bell, Info } from 'lucide-react'
import { Stage, Window, Text, Row, Button, Field, Segmented, Toggle, Badge, Kbd, type Act, type LookTokens } from '../kit'

export type Material = 'current' | 'acrylic'
export type Scheme = 'frappe' | 'graphite'
export type Scene = 'overview' | 'search' | 'clipboard' | 'theme'
export type Surface = 'downloads' | 'tools' | 'settings' | 'reader' | 'overlays'
export type Appearance = { material: Material; scheme: Scheme; light: boolean }
type State = { query: string; results: boolean; preview: boolean; draft: boolean; applied: boolean; done: boolean }
const initial: State = { query: '', results: false, preview: false, draft: false, applied: false, done: false }
const poster: State = { query: '设置', results: true, preview: true, draft: true, applied: true, done: true }

const palettes = {
  frappe: {
    dark: { rgb: '48 52 70', lower: '41 44 60', raised: '65 69 89', selected: '81 87 109', text: '#c6d0f5', secondary: '#a5adce', muted: '#949cbb', accent: '#c6d0f5', accentText: '#303446', green: '#a6d189' },
    light: { rgb: '239 241 245', lower: '230 233 239', raised: '255 255 255', selected: '204 208 218', text: '#4c4f69', secondary: '#5c5f77', muted: '#6c6f85', accent: '#4c4f69', accentText: '#eff1f5', green: '#40834b' },
  },
  graphite: {
    dark: { rgb: '31 31 35', lower: '24 24 27', raised: '42 42 47', selected: '63 63 70', text: '#f4f4f5', secondary: '#b4b4bc', muted: '#96969f', accent: '#d5d6da', accentText: '#202024', green: '#80ba92' },
    light: { rgb: '246 246 247', lower: '236 236 239', raised: '255 255 255', selected: '218 218 223', text: '#27272a', secondary: '#52525b', muted: '#71717a', accent: '#3f3f46', accentText: '#ffffff', green: '#3f7c50' },
  },
}

export function appearanceStyle({ scheme, light }: Appearance): CSSProperties {
  const p = palettes[scheme][light ? 'light' : 'dark']
  return {
    '--nx-rgb': p.rgb, '--nx-lower-rgb': p.lower, '--nx-raised-rgb': p.raised,
    '--nx-selected-rgb': p.selected, '--nx-text': p.text, '--nx-secondary': p.secondary,
    '--nx-muted': p.muted, '--nx-accent': p.accent, '--nx-accent-text': p.accentText, '--nx-green': p.green,
    '--ss-text': p.secondary, '--ss-text-strong': p.text, '--ss-text-muted': p.muted,
    '--ss-surface': `rgb(${p.rgb})`, '--ss-canvas': `rgb(${p.lower})`, '--ss-raised': `rgb(${p.raised})`,
    '--ss-line': light ? 'rgb(30 34 48 / .14)' : 'rgb(224 229 255 / .13)',
    '--ss-accent': p.accent, '--ss-accent-text': p.accentText, '--ss-accent-soft': `rgb(${p.selected} / .7)`,
    '--ss-success': p.green, '--ss-success-text': p.green,
    '--ss-font': '"Sarasa UI SC", "Microsoft YaHei UI", sans-serif',
    '--ss-radius-window': '10px', '--ss-radius-md': '6px', '--ss-radius-sm': '4px',
  } as CSSProperties
}

function Glyph({ icon: Icon, size = 16 }: { icon: typeof Search; size?: number }) {
  return <Icon width={size} height={size} strokeWidth={1.65} aria-hidden />
}
function Caption() {
  const assets = '../../../rust/apps/nxtools-desktop/assets/caption/'
  return <span className="nx-caption" aria-hidden>{['minimize', 'maximize', 'close'].map(name => <span key={name}><img src={`${assets}${name}.svg`} alt="" /></span>)}</span>
}
function AppWindow({ title = 'NxTools', className = '', children, ...rest }: { title?: string; className?: string; children: ReactNode } & Record<`data-${string}`, unknown>) {
  return <Window title={title} toolbar={<Caption />} className={`nx-window ${className}`} {...rest}>{children}</Window>
}
function Hint({ children }: { children: string }) { return <Text className="nx-muted">{children}</Text> }
function Section({ title, children }: { title: string; children: ReactNode }) {
  return <section className="nx-section"><Text className="nx-section-title">{title}</Text>{children}</section>
}
const modules = [
  { label: '下载', icon: Download, surface: 'downloads' }, { label: '快捷启动', icon: Search },
  { label: '剪切板', icon: Clipboard }, { label: '其他工具', icon: Wrench, surface: 'tools' },
  { label: '扩展', icon: Puzzle }, { label: '全局设置', icon: Settings, surface: 'settings' },
] as const
function Sidebar({ surface }: { surface: Surface }) {
  return <aside className="nx-sidebar">
    <div className="nx-side-nav">{modules.map(item => <Row key={item.label} pad={8} className="nx-nav-row" selected={'surface' in item && item.surface === surface} lead={<Glyph icon={item.icon} />} title={<Text>{item.label}</Text>} />)}</div>
    <div className="nx-side-bottom"><Row pad={8} lead={<Glyph icon={Info} />} title={<Text>关于 NxTools</Text>} /></div>
  </aside>
}
function Downloads() {
  return <div className="nx-work-content">
    <div className="nx-content-heading"><div><Text className="nx-page-title">下载</Text><Hint>管理任务与下载队列</Hint></div><Button size="large" icon={<Glyph icon={Plus} />}>新建下载</Button></div>
    <div className="nx-filter-bar"><Segmented items={['全部', '进行中', '已完成']} value={0} /><Button variant="ghost" icon={<Glyph icon={Folder} />}>打开目录</Button></div>
    <div className="nx-download-list">
      <div className="nx-download-item"><Row pad={12} lead={<Glyph icon={Archive} size={22} />} title={<Text>项目素材.zip</Text>} subtitle={<Text>128 MB / 256 MB</Text>} meta={<Text>8.2 MB/s</Text>}><Button size="small" variant="ghost" aria-label="暂停"><Glyph icon={Pause} /></Button></Row><div className="nx-progress"><span style={{ width: '50%' }} /></div></div>
      <div className="nx-download-item"><Row pad={12} lead={<Glyph icon={FileText} size={22} />} title={<Text>使用说明.pdf</Text>} subtitle={<Text>12.4 MB</Text>} meta={<Badge tone="success" icon={<Check size={12} />}>已完成</Badge>}><Button size="small" variant="ghost" aria-label="打开所在目录"><Glyph icon={Folder} /></Button></Row></div>
      <div className="nx-download-item"><Row pad={12} lead={<Glyph icon={Image} size={22} />} title={<Text>界面资源.zip</Text>} subtitle={<Text>等待队列</Text>} meta={<Hint>36.8 MB</Hint>}><Button size="small" variant="ghost" aria-label="开始"><Glyph icon={Play} /></Button></Row></div>
    </div>
    <div className="nx-list-summary"><Hint>3 个任务</Hint><span className="nx-status"><span />下载服务已连接</span></div>
  </div>
}
const tools = [
  [Clipboard, '剪切板', '检索与复用复制记录'], [BookOpen, 'Markdown', '阅读、编辑与大纲导航'],
  [Archive, '压缩包', '浏览内容与提取文件'], [Link, '链接检测', '识别链接并快速处理'],
] as const
function Tools() {
  return <div className="nx-work-content"><div className="nx-content-heading"><div><Text className="nx-page-title">其他工具</Text><Hint>常用能力，按需开启</Hint></div></div><div className="nx-tool-grid">{tools.map(([Icon, name, desc]) => <section key={name} className="nx-tool-tile"><Glyph icon={Icon} size={24} /><Text className="nx-tool-name">{name}</Text><Hint>{desc}</Hint><div className="nx-tool-actions"><Toggle on /><Button size="small">配置</Button></div></section>)}</div></div>
}
function SettingsContent({ s, scheme, live = false }: { s: State; scheme: Scheme; live?: boolean }) {
  return <div className="nx-work-content nx-settings-content">
    <div className="nx-content-heading"><div><Text className="nx-page-title">外观</Text><Hint>主题、配色与字体分别选择</Hint></div></div>
    <Section title="主题"><div className="nx-setting"><div><Text>界面主题</Text><Hint>应用到所有窗口与浮层</Hint></div><Segmented className="nx-theme-choices" items={['现有主题', '磨砂亚克力']} value={Number(s.draft)} targets={['choose-current', 'choose-acrylic']} /></div></Section>
    <Section title="配色与文字"><div className="nx-setting"><Text>配色方案</Text><Button trailingIcon={<Glyph icon={ChevronDown} />}>{scheme === 'frappe' ? 'Catppuccin' : 'Graphite'}</Button></div><div className="nx-setting"><Text>界面字体</Text><Button trailingIcon={<Glyph icon={ChevronDown} />}>Sarasa UI SC</Button></div></Section>
    <div className="nx-settings-footer"><Hint>{s.applied && live ? '主题已应用，配色保持不变' : '保留原有功能与操作习惯'}</Hint><Button data-target="apply-theme" variant="primary" size="large" icon={s.applied && live ? <Glyph icon={Check} /> : undefined}>应用</Button></div>
  </div>
}
function Reader() {
  return <div className="nx-reader"><div className="nx-document-tabs"><span><Glyph icon={FileText} />使用说明.md</span><MoreHorizontal size={16} /></div><div className="nx-reader-toolbar"><Button size="small" variant="tertiary">阅读</Button><Button size="small" variant="ghost">分栏</Button><Button size="small" variant="ghost">编辑</Button><span /><Button size="small" variant="ghost" icon={<Glyph icon={BookOpen} />}>大纲</Button></div><article className="nx-document"><Text className="nx-document-title">把常用操作放在手边</Text><Text wrap>NxTools 将下载、搜索和剪切板集中在一个轻量工作区。</Text><Text className="nx-document-heading">快速开始</Text><Text wrap>使用快捷键唤起搜索，输入名称即可定位应用、文件和命令。</Text><blockquote>同一套主题，贯穿主窗口与每一个辅助面板。</blockquote><Text className="nx-document-heading">剪切板</Text><Text wrap>保留选择位置，在需要时查看预览，再继续手中的工作。</Text></article></div>
}
function OverlayContent() {
  return <div className="nx-work-content"><div className="nx-content-heading"><div><Text className="nx-page-title">扩展</Text><Hint>已安装的工具与自动化能力</Hint></div><Button icon={<Glyph icon={Plus} />}>导入扩展</Button></div><div className="nx-detail-illustration"><Section title="扩展信息"><Row lead={<Glyph icon={Command} size={22} />} title={<Text>文本处理</Text>} subtitle={<Text>在当前选择上执行操作</Text>} /><div className="nx-detail-line"><Hint>运行状态</Hint><Badge tone="success">可用</Badge></div><div className="nx-detail-line"><Hint>触发方式</Hint><Text>手动执行</Text></div><Button size="large">打开设置</Button></Section><div className="nx-menu"><Row pad={8} title={<Text>复制文本</Text>} lead={<Glyph icon={Copy} />} selected meta={<Kbd plain keys={['Ctrl', 'C']} />} /><Row pad={8} title={<Text>固定记录</Text>} lead={<Glyph icon={Pin} />} /><div className="nx-menu-rule" /><Row pad={8} title={<Text>查看详情</Text>} lead={<Glyph icon={FileText} />} /></div></div></div>
}
function Workbench({ surface, s, scheme, className = '', live = false }: { surface: Surface; s: State; scheme: Scheme; className?: string; live?: boolean }) {
  return <AppWindow className={`nx-workbench ${className}`}><div className="nx-app-layout"><Sidebar surface={surface} /><main className="nx-main-pane">{surface === 'downloads' ? <Downloads /> : surface === 'tools' ? <Tools /> : surface === 'reader' ? <Reader /> : surface === 'overlays' ? <OverlayContent /> : <SettingsContent s={s} scheme={scheme} live={live} />}</main></div></AppWindow>
}
function Notice({ className = '' }: { className?: string }) {
  return <section className={`nx-notice ${className}`}><span className="nx-notice-icon"><Glyph icon={Check} size={18} /></span><div><Text className="nx-notice-title">下载已完成</Text><Text className="nx-notice-copy">使用说明.pdf</Text></div><span className="nx-notice-close" aria-hidden>×</span></section>
}
function SearchPanel({ s, overview = false }: { s: State; overview?: boolean }) {
  return <section className={`nx-search-panel ${overview ? 'nx-overview-search' : ''}`}>
    <Field className="nx-search-field" value={s.query} placeholder="搜索应用、文件和命令" focused={!s.results} leading={<Glyph icon={Search} size={20} />} trailing={<Kbd keys={['Esc']} />} box={{ 'data-target': 'search-input' }} />
    <div className="nx-search-results" data-visible={s.results} aria-hidden={!s.results}>
      <Hint>应用与命令</Hint>
      <Row className="nx-result" data-target="result-0" pad={12} selected lead={<Glyph icon={Settings} size={20} />} title={<Text>全局设置</Text>} subtitle={<Text>调整主题、配色和常用选项</Text>} meta={<Kbd plain keys={['Enter']} />} />
      <Row className="nx-result" pad={12} lead={<Glyph icon={Download} size={20} />} title={<Text>下载设置</Text>} subtitle={<Text>连接、目录与下载策略</Text>} />
      <Row className="nx-result" pad={12} lead={<Glyph icon={Clipboard} size={20} />} title={<Text>剪切板设置</Text>} subtitle={<Text>捕获、保留与快捷键</Text>} />
      <div className="nx-search-footer"><Hint>3 个结果</Hint><span><Kbd keys={['↑', '↓']} /><Hint>选择</Hint></span></div>
    </div>
  </section>
}
function ClipboardPanel() {
  return <AppWindow title="剪切板" className="nx-clipboard-panel"><section aria-label="剪切板历史" className="nx-clipboard-body"><Field value="" placeholder="搜索剪切板历史" leading={<Glyph icon={Search} />} box={{ 'data-target': 'clip-search' }} /><div className="nx-clip-filter"><Segmented items={['全部', '文本', '图片', '文件']} value={0} /><Hint>今天</Hint></div><div className="nx-clip-list"><Row data-target="clip-record" pad={12} selected lead={<Glyph icon={FileText} size={20} />} title={<Text>周五前确认界面方案</Text>} subtitle={<Text>文本记录</Text>} meta={<Glyph icon={Pin} size={14} />} /><Row pad={12} lead={<Glyph icon={Link} size={20} />} title={<Text>项目文档</Text>} subtitle={<Text>链接</Text>} /><Row pad={12} lead={<Glyph icon={Folder} size={20} />} title={<Text>界面素材</Text>} subtitle={<Text>文件夹</Text>} /></div><div className="nx-clip-footer"><Hint>3 条记录</Hint><span><Kbd keys={['Enter']} /><Hint>粘贴</Hint></span></div></section></AppWindow>
}
function ClipPreview({ show }: { show: boolean }) {
  return <AppWindow title="预览" className="nx-clip-preview" data-show={show}><div className="nx-preview-content"><Hint>文本</Hint><Text wrap className="nx-preview-text">周五前确认界面方案</Text><Text wrap>保持材质统一，让每个窗口都有清晰的阅读层次。</Text><div className="nx-preview-meta"><Hint>纯文本</Hint><Hint>示例记录</Hint></div></div></AppWindow>
}

async function searchScript(a: Act<State>) {
  await a.wait(850)
  a.mark('focus')
  await a.click('search-input')
  await a.type('query', '设置', { interval: 180 })
  await a.wait(450)
  a.mark('results')
  await a.set({ results: true })
  await a.wait(800)
  await a.move({ at: 'result-0', x: .8, y: .5 })
  a.mark('payoff')
  await a.wait(5600)
  await a.set({ done: true })
}
async function clipboardScript(a: Act<State>) {
  await a.wait(850)
  a.mark('hover')
  await a.move('clip-record')
  await a.wait(800)
  a.mark('preview')
  await a.set({ preview: true })
  a.mark('payoff')
  await a.wait(6500)
  await a.set({ done: true })
}
async function themeScript(a: Act<State>) {
  await a.wait(850)
  a.mark('choose')
  await a.click('choose-acrylic')
  await a.set({ draft: true })
  await a.wait(700)
  a.mark('apply')
  await a.click('apply-theme')
  await a.set({ applied: true })
  a.mark('payoff')
  await a.wait(5900)
  await a.set({ done: true })
}
const scripts = { search: searchScript, clipboard: clipboardScript, theme: themeScript }

export function ThemeScene({ scene, surface, appearance, started, reduced, onDone }: { scene: Scene; surface: Surface; appearance: Appearance; started: boolean; reduced: boolean; onDone: () => void }) {
  const root = useRef<HTMLDivElement>(null)
  const [medium, setMedium] = useState(false)
  useLayoutEffect(() => {
    const node = root.current
    if (!node) return
    const update = () => setMedium(node.clientWidth >= 560 && node.clientWidth < 900)
    update()
    const observer = new ResizeObserver(update)
    observer.observe(node)
    return () => observer.disconnect()
  }, [])
  const script = useMemo(() => scene === 'overview' ? undefined : async (a: Act<State>) => { await scripts[scene](a); onDone() }, [scene, onDone])
  const visual = appearanceStyle(appearance)
  const tokens = Object.fromEntries(Object.entries(visual).filter(([key]) => key.startsWith('--ss-'))) as LookTokens
  return <div ref={root} className="nx-theme" data-medium={medium} data-material={appearance.material} data-light={appearance.light} data-reduced={reduced} style={visual}>
    <Stage width={medium ? 720 : 1040} height={590} compact={{ below: 560, width: 358, height: scene === 'overview' ? 700 : scene === 'theme' ? 620 : scene === 'clipboard' ? 590 : 510 }} className={`nx-scene nx-scene-${scene}`} theme={appearance.light ? 'light' : 'dark'} tokens={tokens} fidelity="realistic" initial={initial} poster={poster} script={started && !reduced ? script : undefined} play="once" cursor={started && scene !== 'overview' && !reduced ? { at: { wide: { x: medium ? 660 : 950, y: 520 }, compact: { x: 330, y: 470 } }, style: 'arrow', idle: 1600 } : undefined}>
      {(s, _rect, layout) => {
        const material = scene === 'theme' && started && !reduced ? (s.applied ? 'acrylic' : 'current') : appearance.material
        return <div className="nx-desktop" data-layout={layout} data-material={material} data-story-done={s.done}>
          <div className="nx-wallpaper" aria-hidden />
          {scene === 'overview' && <><Workbench surface={surface} s={{ ...s, draft: material === 'acrylic' }} scheme={appearance.scheme} /><Notice /><section className="nx-overview-detail"><Text className="nx-small-title">剪切板</Text><Row pad={12} selected lead={<Glyph icon={FileText} />} title={<Text>周五前确认界面方案</Text>} subtitle={<Text>文本记录</Text>} /><div className="nx-small-footer"><Hint>主窗口与辅助面板，共用同一主题</Hint></div></section></>}
          {scene === 'search' && <><Workbench surface="downloads" s={s} scheme={appearance.scheme} className="nx-context-window" /><SearchPanel s={s} /></>}
          {scene === 'clipboard' && <><Workbench surface="downloads" s={s} scheme={appearance.scheme} className="nx-context-window" /><ClipboardPanel /><ClipPreview show={s.preview} /></>}
          {scene === 'theme' && <><Workbench surface="settings" s={started ? s : { ...s, draft: material === 'acrylic' }} scheme={appearance.scheme} live /><Notice /><section className="nx-theme-sample"><Text className="nx-small-title">快捷启动</Text><Field value="" placeholder="搜索应用、文件和命令" leading={<Glyph icon={Search} />} /><Hint>配色与字体保持不变</Hint></section></>}
        </div>
      }}
    </Stage>
  </div>
}
