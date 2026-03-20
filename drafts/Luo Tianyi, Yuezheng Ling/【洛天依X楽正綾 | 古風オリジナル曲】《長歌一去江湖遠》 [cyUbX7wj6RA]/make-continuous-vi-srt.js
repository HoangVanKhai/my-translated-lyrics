#! /usr/bin/env node
//@ts-check
const fs = require('fs')
const path = require('path')
const timeInfo = require('./time-info.json')
const creditsDict = require('./credits-and-titles.json')

const scriptDir = __dirname

// ----- Read inputs -----
const actualChinese = fs.readFileSync(path.join(scriptDir, 'actual.txt'), 'utf8')
const actualChineseLines = actualChinese.split('\n').filter(line => line.trim() !== '')

const actualVietnamese = fs.readFileSync(path.join(scriptDir, 'actual.vi.mtl.txt'), 'utf8')
const actualVietnameseLines = actualVietnamese.split('\n').filter(line => line.trim() !== '')

// Verify that the number of Chinese and Vietnamese lines match
if (actualChineseLines.length !== actualVietnameseLines.length) {
  console.error(`Mismatch: Chinese lines=${actualChineseLines.length}, Vietnamese lines=${actualVietnameseLines.length}`)
  process.exit(1)
}

const textsContent = fs.readFileSync(path.join(scriptDir, 'texts.txt'), 'utf8')
const segments = textsContent.split(/\n{2,}/).filter(Boolean)

const { starts, durations } = timeInfo

if (segments.length !== starts.length || segments.length !== durations.length) {
  console.error(`Mismatch: segments=${segments.length}, starts=${starts.length}, durations=${durations.length}`)
  process.exit(1)
}

// ----- Helpers -----
/**
 * Convert time tuple `[hour, minute, second, millisecond]` to milliseconds.
 * @param {string} h - hours
 * @param {string} m - minutes
 * @param {string} s - seconds
 * @param {string} ms - milliseconds
 * @returns {number} total milliseconds
 */
const toMs = (h, m, s, ms) =>
  (parseInt(h) * 3600 + parseInt(m) * 60 + parseInt(s)) * 1000 + parseInt(ms)

/**
 * Format milliseconds into SRT timestamp (`HH:MM:SS,mmm`).
 * @param {number} ms - milliseconds
 * @returns {string} formatted timestamp
 */
function formatTime(ms) {
  const hours = Math.floor(ms / 3600000)
  const minutes = Math.floor((ms % 3600000) / 60000)
  const seconds = Math.floor((ms % 60000) / 1000)
  const millis = ms % 1000

  const hoursStr = hours.toString().padStart(2, '0')
  const minutesStr = minutes.toString().padStart(2, '0')
  const secondsStr = seconds.toString().padStart(2, '0')
  const millisStr = millis.toString().padStart(3, '0')

  return `${hoursStr}:${minutesStr}:${secondsStr},${millisStr}`
}

/**
 * Remove comment lines (starting with `#`) and struck‑through lines (containing `~~...~~`) from a segment.
 * @param {string} segmentText - raw segment text
 * @returns {string} cleaned text
 */
const cleanSegment = segmentText => segmentText
  .split('\n')
  .filter(line => !line.trimStart().startsWith('#'))
  .filter(line => !/^~~.*~~$/.test(line.trim()))
  .join('\n')

/**
 * Normalize a string by collapsing multiple spaces and trimming.
 * @param {string} s
 * @returns {string}
 */
function normalize(s) {
  return s.trim().replace(/\s+/g, ' ')
}

/**
 * Translate a text using the dictionary.
 * @param {string} text The untranslated text.
 * @returns {string} The translated text.
 */
function translate (text) {
  const entries = Object.entries(creditsDict)
    .sort(([_ak, av], [_bk, bv]) => bv.length - av.length) // sort from longest to shortest

  for (const [key, value] of entries) {
    text = text.replaceAll(key, value)
  }

  return text
}

/**
 * Apply HTML styling to a line of credits (with role in a different color).
 * @param {string} unstyledLine - the unstyled line of credits.
 * @returns {string} styled credits line.
 */
function styleCreditsLine(unstyledLine) {
  const [unstyledRole, ...rest] = unstyledLine.split('  ')
  const unstyledRest = rest.join('  ')
  const styledRole = `<font color="#AAAA22">${unstyledRole}</font>`
  const styledRestInner = unstyledRest.replaceAll(
    /【[^【】]*】/g,
    text => `<font color="#55ABCD">${text}</font>`,
  )
  const styledRest = `<font color="#AAAAAA">${styledRestInner}</font>`
  const styledLine = `${styledRole}  ${styledRest}`
  return styledLine
}

/**
 * Apply HTML styling to credits text (multiple lines).
 * @param {string} text - plain text (already translated)
 * @returns {string} styled HTML
 */
const styleCredits = text => text
  .split('\n')
  .map(styleCreditsLine)
  .join('\n')

/**
 * Apply HTML styling to title text.
 * @param {string} text - plain text (already translated)
 * @returns {string} styled HTML
 */
const styleTitle = text => text
  .split('\n')
  .filter(line => line.trim() !== '')
  .map(line => `<b><font color="#FFD966">${line}</font></b>`)
  .join('\n')

// ----- Build list of cleaned Chinese lyric lines with their segment index and timing -----
const cleanedChineseLines = [] // { text: string, startMs: number, endMs: number, segIdx: number }

for (let i = 0; i < segments.length; i++) {
  const rawSegment = segments[i]
  if (rawSegment.startsWith('[Credits]') || rawSegment.startsWith('[Title]')) continue

  const [hr, min, sec, ms] = starts[i]
  const startMs = toMs(hr, min, sec, ms)
  const endMs = startMs + durations[i]
  const cleaned = cleanSegment(rawSegment)
  if (cleaned === '') continue

  const lines = cleaned.split('\n')
  for (const line of lines) {
    if (line.trim() !== '') {
      cleanedChineseLines.push({
        text: line.trim(),
        startMs,
        endMs,
        segIdx: i,
      })
    }
  }
}

// ----- Align cleaned Chinese lines to actual Chinese lines (greedy concatenation) -----
let cleanedIdx = 0
const chineseWithTimes = [] // { text: string, startMs: number, endMs: number, startSegIdx: number, endSegIdx: number }

for (const actualChineseRaw of actualChineseLines) {
  const actualNorm = normalize(actualChineseRaw)
  if (actualNorm === '') continue

  let accumulated = ''
  let blockStart = null
  let blockEnd = null
  let startSegIdx = null
  let endSegIdx = null
  let matched = false

  while (cleanedIdx < cleanedChineseLines.length) {
    const lineObj = cleanedChineseLines[cleanedIdx]
    const candidate = accumulated === '' ? lineObj.text : `${accumulated} ${lineObj.text}`
    const candidateNorm = normalize(candidate)

    if (candidateNorm === actualNorm) {
      blockStart = blockStart === null ? lineObj.startMs : blockStart
      blockEnd = lineObj.endMs
      startSegIdx = startSegIdx === null ? lineObj.segIdx : startSegIdx
      endSegIdx = lineObj.segIdx
      chineseWithTimes.push({
        text: actualChineseRaw,
        startMs: blockStart,
        endMs: blockEnd,
        startSegIdx,
        endSegIdx,
      })
      cleanedIdx++
      matched = true
      break
    }
    // Try without spaces
    const candidateNoSpace = candidateNorm.replace(/\s/g, '')
    const actualNoSpace = actualNorm.replace(/\s/g, '')
    if (candidateNoSpace === actualNoSpace && candidateNoSpace !== '') {
      blockStart = blockStart === null ? lineObj.startMs : blockStart
      blockEnd = lineObj.endMs
      startSegIdx = startSegIdx === null ? lineObj.segIdx : startSegIdx
      endSegIdx = lineObj.segIdx
      chineseWithTimes.push({
        text: actualChineseRaw,
        startMs: blockStart,
        endMs: blockEnd,
        startSegIdx,
        endSegIdx,
      })
      cleanedIdx++
      matched = true
      break
    }

    // Not a match yet, continue accumulating
    if (blockStart === null) blockStart = lineObj.startMs
    if (startSegIdx === null) startSegIdx = lineObj.segIdx
    accumulated = candidate
    blockEnd = lineObj.endMs
    endSegIdx = lineObj.segIdx
    cleanedIdx++
  }

  if (!matched) {
    console.error(`Failed to match Chinese line: "${actualNorm}"`)
    process.exit(1)
  }
}

// ----- Build mapping from Chinese line index to its timing -----
// Since actualChineseLines and actualVietnameseLines correspond line by line,
// we can directly map the timing from Chinese line index to Vietnamese line.
const vietnameseWithTimes = chineseWithTimes.map((entry, idx) => ({
  text: actualVietnameseLines[idx] || '',
  startMs: entry.startMs,
  endMs: entry.endMs,
  startSegIdx: entry.startSegIdx,
  endSegIdx: entry.endSegIdx,
}))

// ----- Process credits and title segments -----
// We need to iterate segments in order and output either:
// - a styled credit/title subtitle (translated), or
// - a Vietnamese lyric subtitle when its startSegIdx matches the current segment.

// First, translate and style all credit/title segments.
const outputEntries = [] // { startMs, endMs, text, isStyled }

let vietIdx = 0 // pointer into vietnameseWithTimes

for (let segIdx = 0; segIdx < segments.length; segIdx++) {
  const rawSegment = segments[segIdx]
  const [hr, min, sec, ms] = starts[segIdx]
  const startMs = toMs(hr, min, sec, ms)
  const endMs = startMs + durations[segIdx]

  if (rawSegment.startsWith('[Credits]')) {
    // Clean the segment (remove marker, comments, struck-through lines)
    const cleaned = cleanSegment(rawSegment.slice(10)) // remove "[Credits]" line
    if (cleaned) {
      const styled = styleCredits(translate(cleaned))
      outputEntries.push({ startMs, endMs, text: styled, isStyled: true })
    }
  } else if (rawSegment.startsWith('[Title]')) {
    const cleaned = cleanSegment(rawSegment.slice(7)) // remove "[Title]" line
    if (cleaned) {
      const styled = styleTitle(translate(cleaned))
      outputEntries.push({ startMs, endMs, text: styled, isStyled: true })
    }
  } else {
    // Lyric segment – output Vietnamese line if this segment starts a new actual line
    if (vietIdx < vietnameseWithTimes.length && vietnameseWithTimes[vietIdx].startSegIdx === segIdx) {
      const entry = vietnameseWithTimes[vietIdx]
      outputEntries.push({
        startMs: entry.startMs,
        endMs: entry.endMs,
        text: entry.text,
        isStyled: false,
      })
      vietIdx++
    }
  }
}

// ----- Write SRT -----
let srt = ''
let subtitleIndex = 1

for (const entry of outputEntries) {
  srt += `${subtitleIndex++}\n`
  srt += `${formatTime(entry.startMs)} --> ${formatTime(entry.endMs)}\n`
  srt += `${entry.text}\n\n`
}

srt = srt.trimEnd() + '\n'

fs.writeFileSync(path.join(scriptDir, 'continuous.vi.srt'), srt)
console.log('SRT file generated: continuous.vi.srt')
