import type { AuspiceRefNodesDesc, AuspiceRefNodesOrder, CladeNodeAttrKeyDesc } from '_SchemaRoot'
import { assembleRefNodeIdsDefault, applyRefNodeOrder, resolveRefNodeIds } from 'src/helpers/refNodeDropdown'

// Entry ids are a stable contract shared with `default` and the output columns. Expected values are the
// documented id spellings (built-in constants and the `__founder_of_<attr>__` format), used here as the oracle.
const ROOT = '__root__'
const PARENT = '__parent__'
const CLADE = '__clade_founder__'
const FOUNDER_A = '__founder_of_a__'
const FOUNDER_B = '__founder_of_b__'

const attrs: CladeNodeAttrKeyDesc[] = [
  { name: 'a', displayName: 'A' },
  { name: 'b', displayName: 'B' },
]

function refNodes(order?: AuspiceRefNodesOrder): AuspiceRefNodesDesc {
  return { search: [{ name: 'Fermon' }, { name: 'Proto' }], order }
}

const DEFAULT_IDS = [ROOT, PARENT, CLADE, FOUNDER_A, FOUNDER_B, 'Fermon', 'Proto']

describe('refNodeDropdown', () => {
  describe('assembleRefNodeIdsDefault', () => {
    it('is builtins, then attribute founders, then custom search entries', () => {
      expect(assembleRefNodeIdsDefault(refNodes(), attrs)).toEqual(DEFAULT_IDS)
    })

    it('excludes attribute founders flagged skipAsReference', () => {
      const descs: CladeNodeAttrKeyDesc[] = [
        { name: 'a', displayName: 'A' },
        { name: 'b', displayName: 'B', skipAsReference: true },
      ]
      expect(assembleRefNodeIdsDefault(refNodes(), descs)).toEqual([ROOT, PARENT, CLADE, FOUNDER_A, 'Fermon', 'Proto'])
    })
  })

  describe('resolveRefNodeIds', () => {
    // AC1: absent `order` reproduces the default assembly exactly (back-compat no-op).
    it('AC1: absent order yields the default order', () => {
      expect(resolveRefNodeIds(refNodes(undefined), attrs)).toEqual(DEFAULT_IDS)
    })

    // AC2: whitelist ordering with hide shows exactly the listed entries, in order.
    it('AC2: order + hide shows exactly the whitelisted entries', () => {
      const order: AuspiceRefNodesOrder = { entries: ['Fermon', ROOT], others: 'hide' }
      expect(resolveRefNodeIds(refNodes(order), attrs)).toEqual(['Fermon', ROOT])
    })

    // AC3: partial order with default `keep` lifts the listed entry; the rest follow in default order.
    it('AC3: partial order keeps unlisted entries in default order', () => {
      const order: AuspiceRefNodesOrder = { entries: ['Fermon'] }
      expect(resolveRefNodeIds(refNodes(order), attrs)).toEqual([
        'Fermon',
        ROOT,
        PARENT,
        CLADE,
        FOUNDER_A,
        FOUNDER_B,
        'Proto',
      ])
    })

    // AC4: the group token expands to all attribute founders, in natural order, once each.
    it('AC4: __attr_founders__ expands to all attribute founders', () => {
      const order: AuspiceRefNodesOrder = { entries: [ROOT, '__attr_founders__'], others: 'hide' }
      expect(resolveRefNodeIds(refNodes(order), attrs)).toEqual([ROOT, FOUNDER_A, FOUNDER_B])
    })

    it('AC4: repeated group token does not duplicate founders', () => {
      const order: AuspiceRefNodesOrder = { entries: ['__attr_founders__', '__attr_founders__'], others: 'hide' }
      expect(resolveRefNodeIds(refNodes(order), attrs)).toEqual([FOUNDER_A, FOUNDER_B])
    })

    // AC4b: an individual founder id is not a placement (group-only); it is ignored like any unknown id.
    it('AC4b: individual __founder_of_<attr>__ in entries is ignored', () => {
      const order: AuspiceRefNodesOrder = { entries: [ROOT, FOUNDER_A], others: 'hide' }
      expect(resolveRefNodeIds(refNodes(order), attrs)).toEqual([ROOT])
    })

    // AC5: an id that names no entry is ignored, with no error.
    it('AC5: unknown id in entries is ignored', () => {
      const order: AuspiceRefNodesOrder = { entries: ['NoSuchName', ROOT], others: 'hide' }
      expect(resolveRefNodeIds(refNodes(order), attrs)).toEqual([ROOT])
    })
  })

  describe('applyRefNodeOrder', () => {
    it('empty entries with keep is a no-op', () => {
      expect(applyRefNodeOrder(DEFAULT_IDS, [FOUNDER_A, FOUNDER_B], { entries: [], others: 'keep' })).toEqual(
        DEFAULT_IDS,
      )
    })

    it('empty entries with hide drops everything', () => {
      expect(applyRefNodeOrder(DEFAULT_IDS, [FOUNDER_A, FOUNDER_B], { entries: [], others: 'hide' })).toEqual([])
    })
  })
})
