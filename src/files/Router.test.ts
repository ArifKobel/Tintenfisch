import { describe, expect, it } from 'vitest'
import { Router, type BrowserWindow } from '@files/Router.ts'

function fakeWindow(path: string) {
  const location = { pathname: path }
  const pushed: string[] = []
  let popstate: () => void = () => {}
  const window = {
    location,
    history: {
      pushState: (_state: unknown, _title: string, url: string) => {
        pushed.push(url)
        location.pathname = url
      },
      replaceState: (_state: unknown, _title: string, url: string) => {
        location.pathname = url
      },
    },
    addEventListener: (_type: string, listener: () => void) => {
      popstate = listener
    },
  } as unknown as BrowserWindow
  const back = (to: string) => {
    location.pathname = to
    popstate()
  }
  return { window, pushed, back }
}

describe('Router', () => {
  it('reads documents and the home page from the path', () => {
    expect(Router.parse('/documents/0b1c')).toEqual({ kind: 'document', id: '0b1c' })
    expect(Router.parse('/documents/0b1c/')).toEqual({ kind: 'document', id: '0b1c' })
    expect(Router.parse('/')).toEqual({ kind: 'home' })
    expect(Router.parse('/anything/else')).toEqual({ kind: 'home' })
  })

  it('pushes new routes once and follows the back button', () => {
    const { window, pushed, back } = fakeWindow('/')
    const router = new Router(window)
    const seen: string[] = []
    router.subscribe(() => seen.push(Router.pathOf(router.getSnapshot())))
    router.go({ kind: 'document', id: 'a1' })
    router.go({ kind: 'document', id: 'a1' })
    expect(pushed).toEqual(['/documents/a1'])
    back('/')
    expect(seen).toEqual(['/documents/a1', '/'])
    expect(router.getSnapshot()).toEqual({ kind: 'home' })
  })
})
