import type { AuspiceRefNodesDesc, AuspiceRefNodesOrder, CladeNodeAttrKeyDesc } from '_SchemaRoot'
import { REF_NODE_ATTR_FOUNDERS_GROUP, REF_NODE_CLADE_FOUNDER, REF_NODE_PARENT, REF_NODE_ROOT } from 'src/constants'
import { getCladeNodeAttrFounderSearchId } from 'src/helpers/relativeMuts'

/**
 * Single owner of the "Relative to" dropdown entry set and its ordering/hiding.
 *
 * Every entry is identified by a stable id: the built-ins `__root__`, `__parent__`, `__clade_founder__`; one
 * `__founder_of_<attr>__` per clade-like attribute (unless `skipAsReference`); and one per custom `search` entry
 * (its `name`). `resolveRefNodeIds` produces the ordered, filtered list of visible ids from the dataset's
 * `ref_nodes` config. Both the dropdown (rendering) and the preselection (`useRunAnalysis`) consume it, so the
 * rendered set and the preselected id never diverge.
 */

/** Attribute-founder ids (`__founder_of_<attr>__`) for attributes that participate as reference nodes. */
export function getAttrFounderIds(cladeNodeAttrDescs: CladeNodeAttrKeyDesc[] | undefined): string[] {
  return (cladeNodeAttrDescs ?? [])
    .filter((desc) => !desc.skipAsReference)
    .map((desc) => getCladeNodeAttrFounderSearchId(desc.name))
}

/** Default dropdown order, reproduced when `order` is absent: builtins, then attr founders, then custom entries. */
export function assembleRefNodeIdsDefault(
  refNodes: AuspiceRefNodesDesc | undefined,
  cladeNodeAttrDescs: CladeNodeAttrKeyDesc[] | undefined,
): string[] {
  const attrFounderIds = getAttrFounderIds(cladeNodeAttrDescs)
  const searchIds = (refNodes?.search ?? []).map((search) => search.name)
  return [REF_NODE_ROOT, REF_NODE_PARENT, REF_NODE_CLADE_FOUNDER, ...attrFounderIds, ...searchIds]
}

/**
 * Reorder and filter `defaultIds` according to `order`.
 *
 * Walks `order.entries` top to bottom: a built-in or custom-`search` id present in `defaultIds` places that one
 * entry; the `__attr_founders__` token places every not-yet-placed id in `attrFounderIds`. Any other token is
 * ignored, including a bare `__founder_of_<attr>__` literal (attribute founders are group-only) and an id that
 * names no entry. Each id appears at most once. Entries not named are then kept (appended in default order) or
 * hidden, per `order.others` (default `keep`). Absent `order` returns `defaultIds` unchanged.
 */
export function applyRefNodeOrder(
  defaultIds: string[],
  attrFounderIds: string[],
  order: AuspiceRefNodesOrder | undefined,
): string[] {
  const defaultSet = new Set(defaultIds)
  const attrFounderSet = new Set(attrFounderIds)

  const placed: string[] = []
  const placedSet = new Set<string>()
  const place = (id: string) => {
    if (!placedSet.has(id)) {
      placed.push(id)
      placedSet.add(id)
    }
  }

  for (const id of order?.entries ?? []) {
    if (id === REF_NODE_ATTR_FOUNDERS_GROUP) {
      attrFounderIds.forEach(place)
    } else if (defaultSet.has(id) && !attrFounderSet.has(id)) {
      // Built-in or custom `search` id. Attribute-founder ids are excluded here so they can be placed only
      // through the `__attr_founders__` token (group-only rule).
      place(id)
    }
    // Otherwise: unknown id, bare attribute-founder id, or duplicate -> ignored.
  }

  if (order?.others === 'hide') {
    return placed
  }
  const remainder = defaultIds.filter((id) => !placedSet.has(id))
  return [...placed, ...remainder]
}

/** Ordered, filtered list of visible dropdown entry ids for a dataset's `ref_nodes` config. */
export function resolveRefNodeIds(
  refNodes: AuspiceRefNodesDesc | undefined,
  cladeNodeAttrDescs: CladeNodeAttrKeyDesc[] | undefined,
): string[] {
  const defaultIds = assembleRefNodeIdsDefault(refNodes, cladeNodeAttrDescs)
  const attrFounderIds = getAttrFounderIds(cladeNodeAttrDescs)
  return applyRefNodeOrder(defaultIds, attrFounderIds, refNodes?.order)
}
