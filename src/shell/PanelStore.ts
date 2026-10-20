export type LeftPanel = 'tools' | 'edit'
export type RightPanel = 'comments' | 'pages' | 'bookmarks'

export interface PanelState {
  left: LeftPanel
  right: RightPanel | null
}

type Listener = () => void

export class PanelStore {
  readonly #listeners = new Set<Listener>()
  #state: PanelState = { left: 'tools', right: null }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): PanelState => this.#state

  showLeft(left: LeftPanel) {
    if (left !== this.#state.left) this.#update({ ...this.#state, left })
  }

  showRight(right: RightPanel | null) {
    if (right !== this.#state.right) this.#update({ ...this.#state, right })
  }

  toggleRight(right: RightPanel) {
    this.showRight(this.#state.right === right ? null : right)
  }

  #update(state: PanelState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}
