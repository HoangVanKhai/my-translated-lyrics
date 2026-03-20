#! /usr/bin/env node
//@ts-check
const fs = require('fs')
const path = require('path')
const timeInfo = require('./time-info.json')

const scriptDir = __dirname

// ----- Read inputs -----
const actualText = fs.readFileSync(path.join(scriptDir, 'actual.txt'), 'utf8')
const actualLines = actualText.split('\n').filter(line => line.trim() !== '')

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

// ----- Style functions (copied from make-discontinuous-srt.js) -----
/**
 * Apply HTML style to a line of credits.
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
 * Apply HTML styling to credits text.
 * @param {string} text - plain text
 * @returns {string} styled HTML
 */
const styleCredits = text => text
  .split('\n')
  .map(styleCreditsLine)
  .join('\n')

/**
 * Apply HTML styling to title text.
 * @param {string} text - plain text
 * @returns {string} styled HTML
 */
const styleTitle = text => text
  .split('\n')
  .filter(line => line.trim() !== '')
  .map(line => `<b><font color="#FFD966">${line}</font></b>`)
  .join('\n')

// ----- Build list of cleaned lyric lines with their segment index and timing -----
const cleanedLines = [] // { text: string, startMs: number, endMs: number, segIdx: number }

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
      cleanedLines.push({
        text: line.trim(),
        startMs,
        endMs,
        segmentIndex: i,
      })
    }
  }
}

// ----- Align cleaned lines to actual lines (greedy concatenation) -----
let cleanedIndex = 0
const actualWithTimes = [] // { text: string, startMs: number, endMs: number, startSegIdx: number, endSegIdx: number }

for (const actualRaw of actualLines) {
  const actualNorm = normalize(actualRaw)
  if (actualNorm === '') continue

  let accumulated = ''
  let blockStart = null
  let blockEnd = null
  let startSegmentIndex = null
  let endSegmentIndex = null
  let matched = false

  while (cleanedIndex < cleanedLines.length) {
    const lineObj = cleanedLines[cleanedIndex]
    const candidate = accumulated === '' ? lineObj.text : `${accumulated} ${lineObj.text}`
    const candidateNorm = normalize(candidate)

    if (candidateNorm === actualNorm) {
      blockStart = blockStart === null ? lineObj.startMs : blockStart
      blockEnd = lineObj.endMs
      startSegmentIndex = startSegmentIndex === null ? lineObj.segmentIndex : startSegmentIndex
      endSegmentIndex = lineObj.segmentIndex
      actualWithTimes.push({
        text: actualRaw,
        startMs: blockStart,
        endMs: blockEnd,
        startSegIdx: startSegmentIndex,
        endSegIdx: endSegmentIndex,
      })
      cleanedIndex++
      matched = true
      break
    }
    // Try without spaces (remove all spaces)
    const candidateNoSpace = candidateNorm.replace(/\s/g, '')
    const actualNoSpace = actualNorm.replace(/\s/g, '')
    if (candidateNoSpace === actualNoSpace && candidateNoSpace !== '') {
      blockStart = blockStart === null ? lineObj.startMs : blockStart
      blockEnd = lineObj.endMs
      startSegmentIndex = startSegmentIndex === null ? lineObj.segmentIndex : startSegmentIndex
      endSegmentIndex = lineObj.segmentIndex
      actualWithTimes.push({
        text: actualRaw,
        startMs: blockStart,
        endMs: blockEnd,
        startSegIdx: startSegmentIndex,
        endSegIdx: endSegmentIndex,
      })
      cleanedIndex++
      matched = true
      break
    }

    // Not a match yet, continue accumulating
    if (blockStart === null) blockStart = lineObj.startMs
    if (startSegmentIndex === null) startSegmentIndex = lineObj.segmentIndex
    accumulated = candidate
    blockEnd = lineObj.endMs
    endSegmentIndex = lineObj.segmentIndex
    cleanedIndex++
  }

  if (!matched) {
    console.error(`Failed to match actual line: "${actualNorm}"`)
    process.exit(1)
  }
}

// ----- Build output entries in segment order -----
const outputEntries = [] // { startMs, endMs, text, isStyled? }

let actualPtr = 0 // index into actualWithTimes

for (let segIdx = 0; segIdx < segments.length; segIdx++) {
  const rawSegment = segments[segIdx]
  const [hr, min, sec, ms] = starts[segIdx]
  const startMs = toMs(hr, min, sec, ms)
  const endMs = startMs + durations[segIdx]

  if (rawSegment.startsWith('[Credits]')) {
    // Clean and style credits
    const cleaned = cleanSegment(rawSegment.slice(10)) // remove "[Credits]" line
    if (cleaned) {
      const styled = styleCredits(cleaned)
      outputEntries.push({ startMs, endMs, text: styled, isStyled: true })
    }
  } else if (rawSegment.startsWith('[Title]')) {
    const cleaned = cleanSegment(rawSegment.slice(7)) // remove "[Title]" line
    if (cleaned) {
      const styled = styleTitle(cleaned)
      outputEntries.push({ startMs, endMs, text: styled, isStyled: true })
    }
  } else {
    // Lyric segment – we may need to output an actual line if this segment starts a new actual line group
    if (actualPtr < actualWithTimes.length && actualWithTimes[actualPtr].startSegIdx === segIdx) {
      const entry = actualWithTimes[actualPtr]
      // Use entry.startMs and entry.endMs (which may span multiple segments)
      outputEntries.push({
        startMs: entry.startMs,
        endMs: entry.endMs,
        text: entry.text,
        isStyled: false,
      })
      actualPtr++
    }
    // Otherwise, this segment is part of a previous actual line; do nothing.
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

srt = srt.trimEnd()

// ----- Write output -----
fs.writeFileSync(path.join(scriptDir, 'continuous.srt'), srt)
console.log('SRT file generated: continuous.srt')
