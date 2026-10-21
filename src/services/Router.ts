export type Route = { kind: 'home' } | { kind: 'document'; id: string }

export type BrowserWindow = Pick<Window, 'location' | 'history' | 'addEventListener'>

type Listener = () => void

const DOCUMENT_PATH = /^\/documents\/([^/]+)\/?$/

export class Router {
  readonly #window: BrowserWindow
  readonly #listeners = new Set<Listener>()
  #route: Route

  constructor(window: BrowserWindow) {
    this.#window = window
    this.#route = Router.parse(window.location.pathname)
    window.addEventListener('popstate', () => this.#update(Router.parse(this.#window.location.pathname)))
  }

  static parse(path: string): Route {
    const id = DOCUMENT_PATH.exec(path)?.[1]
    return id ? { kind: 'document', id: decodeURIComponent(id) } : { kind: 'home' }
  }

  static pathOf(route: Route): string {
    return route.kind === 'document' ? `/documents/${encodeURIComponent(route.id)}` : '/'
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): Route => this.#route

  go(route: Route) {
    if (Router.pathOf(route) === Router.pathOf(this.#route)) return
    this.#window.history.pushState(null, '', Router.pathOf(route))
    this.#update(route)
  }

  replace(route: Route) {
    this.#window.history.replaceState(null, '', Router.pathOf(route))
    this.#update(route)
  }

  #update(route: Route) {
    this.#route = route
    this.#listeners.forEach((listener) => listener())
  }
}
