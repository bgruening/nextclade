/* eslint-disable no-template-curly-in-string */

/**
 *
 * This dangerously and unreliably patches some of the node_modules. Mostly cosmetic stuff.
 * Do no use this to fix bugs or introduce features. Consider contributing to the upstream project instead.
 *
 */
import { concurrent } from 'fasy'
import fs from 'fs-extra'
import glob from 'glob'
import { promisify } from 'util'

export async function replace(filename: string, searchValue: string | RegExp, replaceValue = '') {
  const content = await fs.readFile(filename, 'utf8')
  const newContent = content.replace(searchValue, replaceValue)
  await fs.writeFile(filename, newContent, { encoding: 'utf8' })
}

/** Strips timerStart() and timerEnd() calls from Auspice */
export async function removeAuspiceTimers() {
  await fs.rm('node_modules/auspice/src/util/perf.js', { force: true })

  const files = await promisify(glob)('node_modules/auspice/src/**/*.js')

  await concurrent.forEach(async (file) => {
    await replace(file, /.*(timerStart|timerEnd)\(".+"\);.*\n/g, '')
    await replace(file, /.*import { timerStart, timerEnd }.*\n/g, '')
  }, files)
}

export async function main() {
  await Promise.all([
    // Removes warning "<title> should not be used in _document.js".
    // Reason: We want title and other SEO tags to be pre-rendered, so that crawlers could find them.
    replace(
      'node_modules/next/dist/pages/_document.js',
      `console.warn("Warning: <title> should not be used in _document.js's <Head>. https://nextjs.org/docs/messages/no-document-title");`,
    ),

    // Removes warning about babel codegen skipping optimizations. We only use babel in form of babel-node, to transpile
    // dev scripts on the fly, so this is not at all worth any attention.
    // Reason: too noisy
    replace(
      'node_modules/@babel/generator/lib/index.js',
      'console.error("[BABEL] Note: The code generator has deoptimised the styling of " + `${opts.filename} as it exceeds the max of ${"500KB"}.`);',
    ),

    // Removes reminder about upgrading caniuse database. Nice, but not that important. Will be handled along with
    // routine package updates.
    // Reason: too noisy
    replace(
      'node_modules/browserslist/node.js',
      `      console.warn(
        'Browserslist: caniuse-lite is outdated. Please run:\\n' +
        'npx browserslist@latest --update-db\\n' +
        '\\n' +
        'Why you should do it regularly:\\n' +
        'https://github.com/browserslist/browserslist#browsers-data-updating'
      )`,
    ),

    removeAuspiceTimers(),
  ])
}

main().catch(console.error)