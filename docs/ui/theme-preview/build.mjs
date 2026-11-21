import { createRequire } from 'node:module'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { resolve, join } from 'node:path'
import { readFile, writeFile, rm } from 'node:fs/promises'

const root = fileURLToPath(new URL('.', import.meta.url))
const tools = process.env.NXTOOLS_PREVIEW_TOOLS
const require = createRequire(tools ? join(resolve(tools), 'package.json') : import.meta.url)
const { build } = await import(pathToFileURL(join(require.resolve('vite/package.json'), '../dist/node/index.js')).href)
const aliases = ['react', 'react-dom', 'lucide-react'].map(name => ({
  find: new RegExp(`^${name}(/.*)?$`),
  replacement: join(require.resolve(`${name}/package.json`), '..') + '$1',
}))
await build({
  configFile: false, root, base: './',
  resolve: { alias: aliases },
  esbuild: { jsx: 'automatic' },
  define: { 'process.env.NODE_ENV': JSON.stringify('production') },
  build: { outDir: 'dist', emptyOutDir: true, assetsInlineLimit: 0, minify: true },
})
const output = join(root, 'dist')
let html = await readFile(join(output, 'index.html'), 'utf8')
const script = html.match(/<script type="module"[^>]*src="\.\/([^\"]+)"[^>]*><\/script>/)
const style = html.match(/<link rel="stylesheet"[^>]*href="\.\/([^\"]+)"[^>]*>/)
if (!script || !style) throw new Error('Missing built script or style')
const js = await readFile(join(output, script[1]), 'utf8')
const css = await readFile(join(output, style[1]), 'utf8')
html = html.replace(script[0], () => `<script type="module">${js.replace(/<\/script/gi, '<\\/script')}</script>`)
html = html.replace(style[0], () => `<style>${css}</style>`)
await writeFile(join(root, 'preview.html'), html)
await rm(output, { recursive: true })
console.log('Built preview.html (local, bundled scripts and styles)')
