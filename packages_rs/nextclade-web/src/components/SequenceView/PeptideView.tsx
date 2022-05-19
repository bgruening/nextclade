import React, { useCallback, useState } from 'react'
import { ReactResizeDetectorDimensions, withResizeDetector } from 'react-resize-detector'
import { Alert as ReactstrapAlert } from 'reactstrap'
import { useRecoilValue } from 'recoil'
import { geneMapAtom } from 'src/state/results.state'
import styled from 'styled-components'

import type { AnalysisResult, Gene, PeptideWarning } from 'src/algorithms/types'
import { useTranslationSafe } from 'src/helpers/useTranslationSafe'
import { getSafeId } from 'src/helpers/getSafeId'
import { WarningIcon } from 'src/components/Results/getStatusIconAndText'
import { Tooltip } from 'src/components/Results/Tooltip'
import { PeptideMarkerMutationGroup } from './PeptideMarkerMutationGroup'
import { SequenceViewWrapper, SequenceViewSVG } from './SequenceView'
import { groupAdjacentAminoacidChanges } from './groupAdjacentAminoacidChanges'
import { PeptideMarkerUnknown } from './PeptideMarkerUnknown'
import { PeptideMarkerFrameShift } from './PeptideMarkerFrameShift'

const MissingRow = styled.div`
  background-color: ${(props) => props.theme.gray650};
  color: ${(props) => props.theme.gray100};
  margin: auto;
  cursor: pointer;
  box-shadow: 2px 2px 5px #0005 inset;
`

const Alert = styled(ReactstrapAlert)`
  box-shadow: ${(props) => props.theme.shadows.slight};
  max-width: 400px;
`

export interface PeptideViewMissingProps {
  geneName: string
  reasons: PeptideWarning[]
}

export function PeptideViewMissing({ geneName, reasons }: PeptideViewMissingProps) {
  const { t } = useTranslationSafe()
  const [showTooltip, setShowTooltip] = useState(false)
  const onMouseEnter = useCallback(() => setShowTooltip(true), [])
  const onMouseLeave = useCallback(() => setShowTooltip(false), [])

  const id = getSafeId('sequence-label', { geneName })

  return (
    <MissingRow id={id} className="w-100 h-100 d-flex" onMouseEnter={onMouseEnter} onMouseLeave={onMouseLeave}>
      <span className="m-auto">{t('Gene "{{ geneName }}" is missing', { geneName })}</span>
      <Tooltip wide fullWidth target={id} isOpen={showTooltip} onMouseEnter={onMouseEnter} onMouseLeave={onMouseLeave}>
        <p>{t('This gene is missing due to the following errors during analysis: ')}</p>
        {reasons.map((warn) => (
          <Alert key={warn.geneName} color="warning" fade={false} className="px-2 py-1 my-1">
            <WarningIcon />
            {warn.message}
          </Alert>
        ))}
      </Tooltip>
    </MissingRow>
  )
}

export interface PeptideViewProps extends ReactResizeDetectorDimensions {
  sequence: AnalysisResult
  warnings?: PeptideWarning[]
  geneMap?: Gene[]
  viewedGene: string
}

export function PeptideViewUnsized({ width, sequence, warnings, viewedGene }: PeptideViewProps) {
  const { t } = useTranslationSafe()

  const geneMap = useRecoilValue(geneMapAtom)

  if (!width) {
    return (
      <SequenceViewWrapper>
        <SequenceViewSVG fill="transparent" viewBox={`0 0 10 10`} />
      </SequenceViewWrapper>
    )
  }

  const gene = geneMap.find((gene) => gene.geneName === viewedGene)
  if (!gene) {
    return (
      <SequenceViewWrapper>
        {t('Gene {{geneName}} is missing in gene map', { geneName: viewedGene })}
      </SequenceViewWrapper>
    )
  }

  const warningsForThisGene = (warnings ?? []).filter((warn) => warn.geneName === viewedGene)
  if (warningsForThisGene.length > 0) {
    return (
      <SequenceViewWrapper>
        <PeptideViewMissing geneName={gene.geneName} reasons={warningsForThisGene} />
      </SequenceViewWrapper>
    )
  }

  const { seqName, unknownAaRanges, frameShifts } = sequence
  const geneLength = gene.end - gene.start
  const pixelsPerAa = width / Math.round(geneLength / 3)
  const aaSubstitutions = sequence.aaSubstitutions.filter((aaSub) => aaSub.gene === viewedGene)
  const aaDeletions = sequence.aaDeletions.filter((aaSub) => aaSub.gene === viewedGene)
  const groups = groupAdjacentAminoacidChanges(aaSubstitutions, aaDeletions)

  const unknownAaRangesForGene = unknownAaRanges.find((range) => range.geneName === viewedGene)

  const frameShiftMarkers = frameShifts
    .filter((frameShift) => frameShift.geneName === gene.geneName)
    .map((frameShift) => (
      <PeptideMarkerFrameShift
        key={`${frameShift.geneName}_${frameShift.nucAbs.begin}`}
        seqName={seqName}
        frameShift={frameShift}
        pixelsPerAa={pixelsPerAa}
      />
    ))

  return (
    <SequenceViewWrapper>
      <SequenceViewSVG viewBox={`0 0 ${width} 10`}>
        <rect fill="transparent" x={0} y={-10} width={geneLength} height="30" />

        {unknownAaRangesForGene &&
          unknownAaRangesForGene.ranges.map((range) => (
            <PeptideMarkerUnknown key={range.begin} seqName={seqName} range={range} pixelsPerAa={pixelsPerAa} />
          ))}

        {groups.map((group) => {
          return (
            <PeptideMarkerMutationGroup
              key={group.codonAaRange.begin}
              seqName={seqName}
              group={group}
              pixelsPerAa={pixelsPerAa}
            />
          )
        })}
        {frameShiftMarkers}
      </SequenceViewSVG>
    </SequenceViewWrapper>
  )
}

export const PeptideViewUnmemoed = withResizeDetector(PeptideViewUnsized)

export const PeptideView = React.memo(PeptideViewUnmemoed)