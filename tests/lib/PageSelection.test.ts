import { describe, expect, it } from 'vitest'
import { PageSelection } from '@/lib/PageSelection.ts'

const order = ['1-0', '2-0', '3-0', '4-0', '5-0']
const plain = { toggle: false, range: false }

describe('PageSelection', () => {
  it('selects one page on a plain click and toggles pages with the modifier', () => {
    const one = PageSelection.EMPTY.click('2-0', order, plain)
    expect(one.inOrder(order)).toEqual(['2-0'])
    const two = one.click('4-0', order, { toggle: true, range: false })
    expect(two.inOrder(order)).toEqual(['2-0', '4-0'])
    expect(two.click('2-0', order, { toggle: true, range: false }).inOrder(order)).toEqual(['4-0'])
  })

  it('extends from the anchor in either direction with shift', () => {
    const anchored = PageSelection.EMPTY.click('4-0', order, plain)
    expect(anchored.click('2-0', order, { toggle: false, range: true }).inOrder(order)).toEqual(['2-0', '3-0', '4-0'])
    expect(anchored.click('5-0', order, { toggle: false, range: true }).inOrder(order)).toEqual(['4-0', '5-0'])
  })

  it('forgets pages that no longer exist', () => {
    const selection = PageSelection.of(['2-0', '3-0'])
    expect(selection.within(['1-0', '3-0']).inOrder(['1-0', '3-0'])).toEqual(['3-0'])
    expect(selection.within(order)).toBe(selection)
  })
})
