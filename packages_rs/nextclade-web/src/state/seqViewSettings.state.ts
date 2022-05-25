import { atom } from 'recoil'
import { ErrorInternal } from 'src/helpers/ErrorInternal'

import { persistAtom } from 'src/state/persist/localStorage'

export enum SeqMarkerHeightState {
  Off = 'Off',
  Top = 'Top',
  Bottom = 'Bottom',
  Full = 'Full',
}

export const SEQ_MARKER_HEIGHT_STATES = Object.keys(SeqMarkerHeightState)

export function seqMarkerHeightStateToString(val: SeqMarkerHeightState) {
  return val.toString()
}

export function seqMarkerHeightStateFromString(key: string) {
  // prettier-ignore
  switch (key) {
    case 'Top': return SeqMarkerHeightState.Top
    case 'Bottom': return SeqMarkerHeightState.Bottom
    case 'Full': return SeqMarkerHeightState.Full
    case 'Off': return SeqMarkerHeightState.Off
  }
  throw new ErrorInternal(`When converting string to 'SeqMarkerHeightState': Unknown variant'${key}'`)
}

export function getSeqMarkerDims(state: SeqMarkerHeightState) {
  switch (state) {
    case SeqMarkerHeightState.Top:
      return { y: -10, height: 10 }
    case SeqMarkerHeightState.Bottom:
      return { y: 10, height: 10 }
    case SeqMarkerHeightState.Full:
      return { y: -10, height: 30 }
    case SeqMarkerHeightState.Off:
      return { y: 0, height: 0 }
  }
  throw new ErrorInternal(`getSeqMarkerDims: Unknown 'SeqMarkerHeightState' variant: '${state}'`) // eslint-disable-line @typescript-eslint/restrict-template-expressions
}

export const seqMarkerMissingHeightStateAtom = atom<SeqMarkerHeightState>({
  key: 'seqMarkerMissingHeight',
  default: SeqMarkerHeightState.Top,
  effects: [persistAtom],
})

export const seqMarkerGapHeightStateAtom = atom<SeqMarkerHeightState>({
  key: 'seqMarkerGapHeight',
  default: SeqMarkerHeightState.Full,
  effects: [persistAtom],
})

export const seqMarkerMutationHeightStateAtom = atom<SeqMarkerHeightState>({
  key: 'seqMarkerMutationHeight',
  default: SeqMarkerHeightState.Full,
  effects: [persistAtom],
})

export const seqMarkerUnsequencedHeightStateAtom = atom<SeqMarkerHeightState>({
  key: 'seqMarkerUnsequencedHeight',
  default: SeqMarkerHeightState.Full,
  effects: [persistAtom],
})

export enum SeqMarkerFrameShiftState {
  Off = 'Off',
  On = 'On',
}

export const seqMarkerFrameShiftStateAtom = atom<SeqMarkerFrameShiftState>({
  key: 'seqMarkerFrameShiftState',
  default: SeqMarkerFrameShiftState.On,
  effects: [persistAtom],
})

export const maxNucMarkersAtom = atom<number>({
  key: 'maxNucMarkers',
  default: 300,
  effects: [persistAtom],
})
