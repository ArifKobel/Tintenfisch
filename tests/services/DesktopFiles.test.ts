import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DesktopFiles } from '@/services/DesktopFiles.ts'

const desktop = vi.hoisted(() => ({
  running: true,
  opened: [] as string[],
  contents: new Map<string, number[]>(),
  handOver: () => {},
}))

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => desktop.running,
  invoke: async (command: string, args?: { path: string }) => {
    if (command === 'opened_files') return [...desktop.opened]
    const index = desktop.opened.indexOf(args?.path ?? '')
    if (index < 0) throw new Error('file was not opened')
    desktop.opened.splice(index, 1)
    return new Uint8Array(desktop.contents.get(args?.path ?? '') ?? []).buffer
  },
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: async (_event: string, handler: () => void) => {
    desktop.handOver = handler
    return () => {}
  },
}))

function openWith(path: string, bytes: number[]) {
  desktop.opened.push(path)
  desktop.contents.set(path, bytes)
}

describe('DesktopFiles', () => {
  beforeEach(() => {
    desktop.running = true
    desktop.opened = []
    desktop.contents.clear()
  })

  it('opens the files the app was started with, named after their path', async () => {
    openWith('/Users/me/Rechnung.pdf', [1, 2])
    openWith('C:\\Docs\\Artikel.pdf', [3])
    const open = vi.fn(async () => {})
    DesktopFiles.watch(open)
    await vi.waitFor(() => expect(open).toHaveBeenCalledTimes(2))
    expect(open).toHaveBeenCalledWith('Rechnung.pdf', new Uint8Array([1, 2]))
    expect(open).toHaveBeenCalledWith('Artikel.pdf', new Uint8Array([3]))
  })

  it('opens files handed over while it runs, each only once', async () => {
    const open = vi.fn(async () => {})
    DesktopFiles.watch(open)
    DesktopFiles.watch(open)
    openWith('/Users/me/Vertrag.pdf', [4])
    desktop.handOver()
    desktop.handOver()
    await vi.waitFor(() => expect(open).toHaveBeenCalledWith('Vertrag.pdf', new Uint8Array([4])))
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(open).toHaveBeenCalledTimes(1)
  })

  it('stays idle in the browser', async () => {
    desktop.running = false
    openWith('/Users/me/Rechnung.pdf', [1])
    const open = vi.fn(async () => {})
    DesktopFiles.watch(open)
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(open).not.toHaveBeenCalled()
    expect(desktop.opened).toEqual(['/Users/me/Rechnung.pdf'])
  })
})
