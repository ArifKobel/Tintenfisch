import type { Target } from '@/types/protocol.ts'
import type { ViewportStore } from '@/stores/ViewportStore.ts'

const OPENABLE = /^(https?:|mailto:)/i

export class TargetFollower {
  readonly #viewport: ViewportStore
  readonly #open: (uri: string) => void

  constructor(viewport: ViewportStore, open: (uri: string) => void = (uri) => window.open(uri, '_blank', 'noopener,noreferrer')) {
    this.#viewport = viewport
    this.#open = open
  }

  follow(target: Target | null) {
    if (target?.kind === 'page') this.#viewport.showAtTop(target.pageId, target.top ?? 0)
    else if (target?.kind === 'uri' && OPENABLE.test(target.uri.trim())) this.#open(target.uri.trim())
  }

  currentView(): Target | null {
    const top = this.#viewport.viewTop
    return top ? { kind: 'page', pageId: top.pageId, top: top.y } : null
  }
}
