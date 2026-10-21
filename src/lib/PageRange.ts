const PART = /^(\d*)\s*-\s*(\d*)$|^(\d+)$/

export class PageRange {
  static parse(text: string, count: number): number[] | null {
    const indices: number[] = []
    for (const part of text.split(',').map((piece) => piece.trim())) {
      if (part === '') continue
      const match = PART.exec(part)
      if (!match) return null
      const [, from, to, single] = match
      const first = single ? Number(single) : from ? Number(from) : 1
      const last = single ? Number(single) : to ? Number(to) : count
      if (first < 1 || last > count || first > last) return null
      for (let page = first; page <= last; page++) if (!indices.includes(page - 1)) indices.push(page - 1)
    }
    return indices.length > 0 ? indices : null
  }
}
