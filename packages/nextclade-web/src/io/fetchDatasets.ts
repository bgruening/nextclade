/* eslint-disable prefer-destructuring */
import type { ParsedUrlQuery } from 'querystring'
import { findSimilarStrings } from 'src/helpers/string'
import { axiosHeadOrUndefined } from 'src/io/axiosFetch'
import {
  isGithubShortcut,
  isGithubRepoUrl,
  parseGitHubRepoShortcut,
  parseGithubRepoUrl,
} from 'src/io/fetchSingleDatasetFromGithub'

import { Dataset } from 'src/types'
import {
  fetchDatasetsIndex,
  findDataset,
  getCompatibleEnabledDatasets,
  getCompatibleMinimizerIndexVersion,
} from 'src/io/fetchDatasetsIndex'
import { getQueryParamMaybe } from 'src/io/getQueryParamMaybe'
import { useRecoilValue, useSetRecoilState } from 'recoil'
import { datasetsAtom, allDatasetsAtom, datasetServerUrlAtom, minimizerIndexVersionAtom } from 'src/state/dataset.state'
import { useQuery } from 'react-query'
import { isNil } from 'lodash'
import urljoin from 'url-join'
import { URL_GITHUB_DATA_RAW } from 'src/constants'

export async function getDatasetFromUrlParams(urlQuery: ParsedUrlQuery, datasets: Dataset[]) {
  // Retrieve dataset-related URL params and try to find a dataset based on these params
  const name = getQueryParamMaybe(urlQuery, 'dataset-name')

  if (!name) {
    return undefined
  }

  const tag = getQueryParamMaybe(urlQuery, 'dataset-tag')

  const dataset = findDataset(datasets, name, tag)

  if (!dataset) {
    // Check if the name exists but the tag is wrong
    if (tag) {
      const nameOnlyDataset = findDataset(datasets, name, undefined)
      if (nameOnlyDataset) {
        // Name exists but tag is wrong
        const availableTags = datasets
          .filter((d) => d.path === name || !!d.shortcuts?.includes(name))
          .map((d) => d.version?.tag)
          .filter((t) => t !== undefined)
          .map((t) => `'${t}'`)
          .join(', ')
        throw new Error(
          `Incorrect URL parameters: dataset with name='${name}' exists, but tag='${tag}' was not found. Available tags: ${availableTags}`,
        )
      }
    }

    // Name doesn't exist at all, suggest similar names
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

export async function getGithubDatasetServerUrl(): Promise<string | undefined> {
  const BRANCH_NAME = process.env.BRANCH_NAME
  if (!BRANCH_NAME) {
    return undefined
  }

  const githubDatasetServerUrl = urljoin(URL_GITHUB_DATA_RAW, BRANCH_NAME, 'data_output')
  const githubIndexJsonUrl = urljoin(githubDatasetServerUrl, 'index.json')

  const headRes = await axiosHeadOrUndefined(githubIndexJsonUrl)

  if (headRes) {
    return githubDatasetServerUrl
  }

  return undefined
}

export function toAbsoluteUrl(url: string): string {
  if (typeof window !== 'undefined' && url.slice(0) === '/') {
    return urljoin(window.location.origin, url)
  }
  return url
}

export async function getDatasetServerUrl(urlQuery: ParsedUrlQuery) {
  // Get dataset URL from query URL params.
  let datasetServerUrl = getQueryParamMaybe(urlQuery, 'dataset-server')

  // If the URL is formatted as a GitHub URL or as a GitHub URL shortcut, use it without any checking
  if (datasetServerUrl) {
    if (isGithubShortcut(datasetServerUrl)) {
      const { owner, repo, branch, path } = await parseGitHubRepoShortcut(datasetServerUrl)
      return urljoin('https://raw.githubusercontent.com', owner, repo, branch, path)
    }

    if (isGithubRepoUrl(datasetServerUrl)) {
      const { owner, repo, branch, path } = await parseGithubRepoUrl(datasetServerUrl)
      return urljoin('https://raw.githubusercontent.com', owner, repo, branch, path)
    }
  }

  // If requested to try GitHub-hosted datasets either using `DATA_TRY_GITHUB_BRANCH` env var (e.g. from
  // `.env` file), or using `&dataset-server=gh` or `&dataset-server=github` URL parameters, then check if the
  // corresponding branch in the default data repo on GitHub contains an `index.json` file. And if yes, use it.
  const datasetServerTryGithubBranch =
    (isNil(datasetServerUrl) && process.env.DATA_TRY_GITHUB_BRANCH === '1') ||
    (datasetServerUrl && ['gh', 'github'].includes(datasetServerUrl))
  if (datasetServerTryGithubBranch) {
    const githubDatasetServerUrl = await getGithubDatasetServerUrl()
    if (githubDatasetServerUrl) {
      datasetServerUrl = githubDatasetServerUrl
    }
  }

  // If none of the above, use hardcoded default URL (from `.env` file)
  datasetServerUrl = datasetServerUrl ?? process.env.DATA_FULL_DOMAIN ?? '/'

  // If the URL happens to be a relative path, then convert to absolute URL (on the app's current host)
  return toAbsoluteUrl(datasetServerUrl)
}

export async function initializeDatasets(datasetServerUrl: string, urlQuery: ParsedUrlQuery = {}) {
  const datasetsIndexJson = await fetchDatasetsIndex(datasetServerUrl)

  // Get all datasets for tag-based lookup
  const allDatasets = getCompatibleEnabledDatasets(datasetServerUrl, datasetsIndexJson, { latestOnly: false })

  // Get only latest datasets for UI display and autodetection
  const latestDatasets = getCompatibleEnabledDatasets(datasetServerUrl, datasetsIndexJson, { latestOnly: true })

  const minimizerIndexVersion = await getCompatibleMinimizerIndexVersion(datasetServerUrl, datasetsIndexJson)

  // Check if URL params specify dataset params and try to find from ALL datasets (including non-latest tags)
  const currentDataset = await getDatasetFromUrlParams(urlQuery, allDatasets)

  return {
    datasets: latestDatasets, // For UI display and autodetection
    allDatasets, // For tag-based lookup
    currentDataset,
    minimizerIndexVersion,
  }
}

/** Refetch dataset index periodically and update the local copy of if */
export function useUpdatedDatasetIndex() {
  const datasetServerUrl = useRecoilValue(datasetServerUrlAtom)
  const setDatasetsState = useSetRecoilState(datasetsAtom)
  const setAllDatasetsState = useSetRecoilState(allDatasetsAtom)
  const setMinimizerIndexVersion = useSetRecoilState(minimizerIndexVersionAtom)

  useQuery(
    ['refetchDatasetIndex'],
    async () => {
      if (isNil(datasetServerUrl)) {
        return
      }
      const { datasets, allDatasets, minimizerIndexVersion } = await initializeDatasets(datasetServerUrl)
      setDatasetsState(datasets)
      setAllDatasetsState(allDatasets)
      setMinimizerIndexVersion(minimizerIndexVersion)
    },
    {
      suspense: false,
      staleTime: 2 * 60 * 60 * 1000, // 2 hours
      refetchInterval: 2 * 60 * 60 * 1000, // 2 hours
      refetchIntervalInBackground: false,
      refetchOnMount: false,
      refetchOnReconnect: false,
      refetchOnWindowFocus: false,
      enabled: !isNil(datasetServerUrl),
    },
  )
}

/**
 * Check currently selected dataset against **local** dataset index periodically and store updated dataset locally.
 * If an updated dataset is stored, user will receive a notification.
 */
export function useUpdatedDataset() {
  // const { datasets } = useRecoilValue(datasetsAtom)
  // const datasetsCurrent = useRecoilValue(datasetsCurrentAtom)
  // const setDatasetUpdated = useSetRecoilState(datasetUpdatedAtom)
  //
  // useQuery(
  //   'currentDatasetState',
  //   async () => {
  //     const path = datasetCurrent?.path
  //     const updatedAt = datasetCurrent?.version?.updatedAt
  //     if (!isNil(updatedAt)) {
  //       const candidateDatasets = filterDatasets(datasets, path)
  //       const updatedDataset = candidateDatasets.find((candidate) => {
  //         const candidateTag = candidate.version?.updatedAt
  //         return candidateTag && candidateTag > updatedAt
  //       })
  //       setDatasetUpdated(updatedDataset)
  //     }
  //     return undefined
  //   },
  //   {
  //     suspense: false,
  //     staleTime: 0,
  //     refetchInterval: 60 * 60 * 1000, // 1 hour
  //     refetchIntervalInBackground: false,
  //     refetchOnMount: true,
  //     refetchOnReconnect: true,
  //     refetchOnWindowFocus: true,
  //   },
  // )
}
