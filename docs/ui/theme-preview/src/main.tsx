import { useCallback, useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { Play, RotateCcw, Square, Sun, Moon, Check, ArrowUpRight } from 'lucide-react'
import { ThemeScene, type Material, type Scheme, type Scene, type Surface } from './scenes'
import '../kit/stageshow.css'
import './styles.css'

const params = new URLSearchParams(location.search)
const scenes: { id: Scene; label: string; description: string }[] = [
  { id: 'overview', label: '主题总览', description: '从主窗口到辅助面板，统一材质与层级。' },
  { id: 'search', label: '快捷启动', description: '输入后呈现结果，不移动输入框，不重复播放列表。' },
  { id: 'clipboard', label: '剪切板预览', description: '保留悬停意图延迟；预览轻量出现，主面板保持稳定。' },
  { id: 'theme', label: '整套主题切换', description: '选择后应用到所有可见窗口，保留配色和字体。' },
]
const surfaces: { id: Surface; label: string }[] = [
  { id: 'downloads', label: '下载' }, { id: 'tools', label: '工具' },
  { id: 'settings', label: '设置' }, { id: 'reader', label: 'Markdown' }, { id: 'overlays', label: '菜单与浮层' },
]
const isScene = (value: string | null): value is Scene => scenes.some(scene => scene.id === value)
const requested = params.get('capture') ?? params.get('scene')
const firstScene: Scene = isScene(requested) ? requested : 'overview'
const capture = params.has('capture')

function Preview() {
  const [scene, setScene] = useState<Scene>(firstScene)
  const [surface, setSurface] = useState<Surface>('downloads')
  const [material, setMaterial] = useState<Material>(params.get('theme') === 'current' ? 'current' : 'acrylic')
  const [scheme, setScheme] = useState<Scheme>(params.get('palette') === 'graphite' ? 'graphite' : 'frappe')
  const [light, setLight] = useState(params.get('mode') === 'light')
  const [run, setRun] = useState(capture && firstScene !== 'overview' ? 1 : 0)
  const [running, setRunning] = useState(capture && firstScene !== 'overview')
  const [forcedReduced, setForcedReduced] = useState(params.has('reduced'))
  const [systemReduced, setSystemReduced] = useState(matchMedia('(prefers-reduced-motion: reduce)').matches)
  const reduced = forcedReduced || systemReduced
  const onDone = useCallback(() => setRunning(false), [])
  useEffect(() => {
    const media = matchMedia('(prefers-reduced-motion: reduce)')
    const update = () => setSystemReduced(media.matches)
    media.addEventListener('change', update)
    return () => media.removeEventListener('change', update)
  }, [])
  useEffect(() => { if (reduced) { setRunning(false); setRun(0) } }, [reduced])
  const chooseScene = (next: Scene) => { setScene(next); setRun(0); setRunning(false) }
  const play = () => { setRun(value => value + 1); setRunning(true) }
  const stop = () => { setRun(0); setRunning(false) }
  const currentScene = scenes.find(item => item.id === scene)!
  const sceneView = <div id="example" data-example={scene} data-preview-theme={material} data-preview-palette={scheme} data-preview-mode={light ? 'light' : 'dark'}>
    <ThemeScene key={`${scene}-${run}`} scene={scene} surface={surface} appearance={{ material, scheme, light }} started={run > 0} reduced={reduced} onDone={onDone} />
  </div>
  if (capture) return <div className="nx-capture">{sceneView}</div>
  return <main className="preview-page">
    <header className="preview-header"><div className="preview-brand"><img src="../../../rust/apps/nxtools-desktop/assets/brand/nxtools.ico" width="24" height="24" alt="" /><span>NxTools</span><span className="preview-divider" /><span>主题预览</span></div><a href="../../../.trellis/tasks/11-21-desktop-motion-acrylic-style/reference-acrylic.png" target="_blank" rel="noreferrer">查看材质参考<ArrowUpRight size={14} /></a></header>
    <section className="preview-intro"><div><h1>磨砂亚克力</h1><p>一套完整主题，贯穿每一个窗口。</p></div><span className="preview-tag">设计样稿</span></section>
    <section className="preview-controls" aria-label="外观预览选项">
      <div className="preview-control"><span id="theme-label">主题</span><div className="preview-segments" role="group" aria-labelledby="theme-label"><button type="button" aria-pressed={material === 'current'} onClick={() => setMaterial('current')}>现有主题</button><button type="button" aria-pressed={material === 'acrylic'} onClick={() => setMaterial('acrylic')}>磨砂亚克力</button></div></div>
      <label className="preview-control">配色<select value={scheme} onChange={event => setScheme(event.target.value as Scheme)}><option value="frappe">Catppuccin</option><option value="graphite">Graphite</option></select></label>
      <button type="button" className="preview-mode" aria-label={light ? '切换到深色' : '切换到浅色'} onClick={() => setLight(value => !value)}>{light ? <Sun size={16} /> : <Moon size={16} />}{light ? '浅色' : '深色'}</button>
    </section>
    <section className="preview-demo" aria-label="主题与动效预览">
      <nav className="preview-scenes" aria-label="演示场景">{scenes.map(item => <button type="button" key={item.id} aria-pressed={scene === item.id} onClick={() => chooseScene(item.id)}>{item.label}</button>)}</nav>
      <div className="preview-stage">{sceneView}</div>
      <div className="preview-stage-footer">{scene === 'overview' ? <nav className="preview-surfaces" aria-label="总览表面">{surfaces.map(item => <button type="button" key={item.id} aria-pressed={surface === item.id} onClick={() => setSurface(item.id)}>{item.label}</button>)}</nav> : <div className="preview-playback"><button type="button" className="preview-play" disabled={reduced} onClick={play}>{run ? <RotateCcw size={14} /> : <Play size={14} />}{run ? '重播演示' : '播放演示'}</button>{running && <button type="button" className="preview-stop" onClick={stop}><Square size={12} />停止</button>}<span className="preview-play-status" aria-live="polite">{reduced ? '显示最终画面' : running ? '正在播放' : run ? '演示结束' : '点击播放，一次结束'}</span></div>}
        <label className="preview-reduced"><input type="checkbox" checked={reduced} disabled={systemReduced} onChange={event => setForcedReduced(event.target.checked)} />减少动态效果{systemReduced && <span>（跟随系统）</span>}</label>
      </div>
    </section>
    <section className="preview-caption"><div><h2>{currentScene.label}</h2><p>{currentScene.description}</p></div><span><Check size={14} />主题与配色独立</span></section>
    <footer className="preview-footnote">浏览器视觉演示。原生亚克力、圆角合成及 CPU / 内存 / GPU 开销将在独立 GPUI 样例中验证。</footer>
  </main>
}

const fontPath = new URL('../../../rust/apps/nxtools-desktop/assets/fonts/SarasaUiSC-Regular.ttf', document.baseURI).href
const font = new FontFace('Sarasa UI SC', `url("${fontPath}")`, { weight: '400', display: 'swap' })
font.load().then(loaded => { document.fonts.add(loaded) }).catch(() => {})
createRoot(document.getElementById('root')!).render(<Preview />)
