import type { ParsedUrlQuery } from 'querystring'
import { findSimilarStrings } from 'src/helpers/string'

import { Dataset } from 'src/types'
import {
  fetchDatasetsIndex,
  filterDatasets,
  findDataset,
  getLatestCompatibleEnabledDatasets,
} from 'src/io/fetchDatasetsIndex'
import { getQueryParamMaybe } from 'src/io/getQueryParamMaybe'
import { useRecoilValue, useSetRecoilState } from 'recoil'
import { datasetCurrentAtom, datasetsAtom, datasetServerUrlAtom, datasetUpdatedAtom } from 'src/state/dataset.state'
import { useQuery } from 'react-query'
import { isNil } from 'lodash'

export async function getDatasetFromUrlParams(urlQuery: ParsedUrlQuery, datasets: Dataset[]) {
  // Retrieve dataset-related URL params and try to find a dataset based on these params
  const name = getQueryParamMaybe(urlQuery, 'dataset-name')

  if (!name) {
    return undefined
  }

  const tag = getQueryParamMaybe(urlQuery, 'dataset-tag')

  const dataset = findDataset(datasets, name, tag)

  if (!dataset) {
    const names = datasets.map((dataset) => dataset.path)
    const suggestions = findSimilarStrings(names, name)
      .slice(0, 10)
      .map((s) => `'${s}'`)
      .join(', ')
    const tagMsg = tag ? ` and tag '${tag}` : ''
    throw new Error(
      `Incorrect URL parameters: unable to find the dataset with name='${name}'${tagMsg}. Did you mean one of: ${suggestions}`,
    )
  }

  return dataset
}

export async function initializeDatasets(urlQuery: ParsedUrlQuery, datasetServerUrlDefault: string) {
  const datasetServerUrl = getQueryParamMaybe(urlQuery, 'dataset-server') ?? datasetServerUrlDefault

  const datasetsIndexJson = await fetchDatasetsIndex(datasetServerUrl)

  const { datasets } = getLatestCompatibleEnabledDatasets(datasetServerUrl, datasetsIndexJson)

  // Check if URL params specify dataset params and try to find the corresponding dataset
  const currentDataset = await getDatasetFromUrlParams(urlQuery, datasets)

  return { datasets, currentDataset }
}

/** Refetch dataset index periodically and update the local copy of if */
export function useUpdatedDatasetIndex() {
  const setDatasetsState = useSetRecoilState(datasetsAtom)
  const datasetServerUrl = useRecoilValue(datasetServerUrlAtom)
  useQuery(
    'refetchDatasetIndex',
    async () => {
      const { currentDataset: _, ...datasetsState } = await initializeDatasets({}, datasetServerUrl)
      setDatasetsState(datasetsState)
    },
    {
      suspense: false,
      staleTime: 0,
      refetchInterval: 2 * 60 * 60 * 1000, // 2 hours
      refetchIntervalInBackground: true,
      refetchOnMount: true,
      refetchOnReconnect: true,
      refetchOnWindowFocus: true,
    },
  )
}

/**
 * Check currently selected dataset against **local** dataset index periodically and store updated dataset locally.
 * If an updated dataset is stored, user will receive a notification.
 */
export function useUpdatedDataset() {
  const { datasets } = useRecoilValue(datasetsAtom)
  const datasetCurrent = useRecoilValue(datasetCurrentAtom)
  const setDatasetUpdated = useSetRecoilState(datasetUpdatedAtom)

  useQuery(
    'currentDatasetState',
    async () => {
      const path = datasetCurrent?.path
      const refAccession = datasetCurrent?.attributes.reference.value
      const updatedAt = datasetCurrent?.version?.updatedAt
      if (!isNil(refAccession) && !isNil(updatedAt)) {
        const candidateDatasets = filterDatasets(datasets, path, refAccession)
        const updatedDataset = candidateDatasets.find((candidate) => {
          const candidateTag = candidate.version?.updatedAt
          return candidateTag && candidateTag > updatedAt
        })
        setDatasetUpdated(updatedDataset)
      }
      return undefined
    },
    {
      suspense: false,
      staleTime: 0,
      refetchInterval: 60 * 60 * 1000, // 1 hour
      refetchIntervalInBackground: false,
      refetchOnMount: true,
      refetchOnReconnect: true,
      refetchOnWindowFocus: true,
    },
  )
}
