import type { PageSummary } from '@engine/protocol.ts'

export const CSS_PIXELS_PER_POINT = 96 / 72
export const PAGE_GAP = 16

export interface PagePlacement {
  id: string
  index: number
  top: number
  width: number
  height: number
}

export interface LayoutAnchor {
  index: number
  fraction: number
}

export class PageLayout {
  readonly placements: PagePlacement[]
  readonly totalHeight: number
  readonly widestPage: number

  constructor(pages: PageSummary[], zoom: number, gap = PAGE_GAP) {
    const scale = zoom * CSS_PIXELS_PER_POINT
    let top = gap
    this.placements = pages.map((page, index) => {
      const placement = { id: page.id, index, top, width: page.width * scale, height: page.height * scale }
      top += placement.height + gap
      return placement
    })
    this.totalHeight = top
    this.widestPage = Math.max(0, ...this.placements.map((placement) => placement.width))
  }

  visible(top: number, bottom: number): PagePlacement[] {
    const first = this.#firstEndingAfter(top)
    const visible: PagePlacement[] = []
    for (let index = first; index < this.placements.length; index++) {
      const placement = this.placements[index]
      if (placement.top >= bottom) break
      visible.push(placement)
    }
    return visible
  }

  anchorAt(offset: number): LayoutAnchor {
    if (this.placements.length === 0) return { index: 0, fraction: 0 }
    const index = Math.min(this.#firstEndingAfter(offset), this.placements.length - 1)
    const placement = this.placements[index]
    return { index, fraction: (offset - placement.top) / Math.max(1, placement.height) }
  }

  offsetOf(anchor: LayoutAnchor): number {
    const placement = this.placements[Math.min(anchor.index, this.placements.length - 1)]
    return placement ? placement.top + anchor.fraction * placement.height : 0
  }

  #firstEndingAfter(offset: number): number {
    let low = 0
    let high = this.placements.length
    while (low < high) {
      const middle = (low + high) >> 1
      const placement = this.placements[middle]
      if (placement.top + placement.height <= offset) low = middle + 1
      else high = middle
    }
    return low
  }
}
