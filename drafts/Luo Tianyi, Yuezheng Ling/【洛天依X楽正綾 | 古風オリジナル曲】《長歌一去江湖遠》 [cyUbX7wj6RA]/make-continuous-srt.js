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

// ----- Build cleaned lines (with their timestamps) from lyric segments only -----
const cleanedLines = [] // each element: { text: string, startMs: number, endMs: number }

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
      })
    }
  }
}

// ----- Align cleaned lines to actual lines by greedy concatenation -----
let cleanedIndex = 0
const actualWithTimes = [] // { text: string, startMs: number, endMs: number }

for (const actualRaw of actualLines) {
  const actualNorm = normalize(actualRaw)
  if (actualNorm === '') continue

  let accumulated = ''
  let blockStart = null
  let blockEnd = null
  let matched = false

  while (cleanedIndex < cleanedLines.length) {
    const lineObj = cleanedLines[cleanedIndex]
    const candidate = accumulated === '' ? lineObj.text : `${accumulated} ${lineObj.text}`
    const candidateNorm = normalize(candidate)

    if (candidateNorm === actualNorm) {
      blockStart = blockStart === null ? lineObj.startMs : blockStart
      blockEnd = lineObj.endMs
      actualWithTimes.push({
        text: actualRaw,
        startMs: blockStart,
        endMs: blockEnd,
      })
      cleanedIndex++
      matched = true
      break
    }
    // Try without spaces (remove all spaces from candidateNorm and actualNorm)
    const candidateNoSpace = candidateNorm.replace(/\s/g, '')
    const actualNoSpace = actualNorm.replace(/\s/g, '')
    if (candidateNoSpace === actualNoSpace && candidateNoSpace !== '') {
      // We'll consider it a match, but we need to keep the original candidate spacing?
      // However, the actual line may have spaces that are not in the candidate, but we still treat as match.
      // For now, just match.
      blockStart = blockStart === null ? lineObj.startMs : blockStart
      blockEnd = lineObj.endMs
      actualWithTimes.push({
        text: actualRaw,
        startMs: blockStart,
        endMs: blockEnd,
      })
      cleanedIndex++
      matched = true
      break
    }

    // Not a match yet, continue accumulating
    if (blockStart === null) blockStart = lineObj.startMs
    accumulated = candidate
    blockEnd = lineObj.endMs
    cleanedIndex++
  }

  if (!matched) {
    console.error(`Failed to match actual line: "${actualNorm}"`)
    process.exit(1)
  }
}

// ----- Extract credits and title from texts.txt for insertion -----
function extractCreditsAndTitle() {
  const parts = textsContent.split(/\n{2,}/).filter(Boolean)
  let credits = ''
  let title = ''
  for (const seg of parts) {
    if (seg.startsWith('[Credits]')) {
      const creditLines = seg.split('\n').slice(1).filter(l => l.trim() !== '')
      credits += creditLines.join('\n') + '\n'
    } else if (seg.startsWith('[Title]')) {
      const titleLines = seg.split('\n').slice(1).filter(l => l.trim() !== '')
      title = titleLines.join('\n')
    }
  }
  return { credits: credits.trim(), title: title.trim() }
}

const { credits, title } = extractCreditsAndTitle()

// ----- Build SRT content -----
let srt = ''
let subtitleIndex = 1

// Add title as first subtitle if present
if (title) {
  const startMs = 0
  const endMs = actualWithTimes.length > 0 ? actualWithTimes[0].startMs : 3000
  srt += `${subtitleIndex++}\n`
  srt += `${formatTime(startMs)} --> ${formatTime(endMs)}\n`
  srt += `${title}\n\n`
}

// Add credits as second subtitle if present
if (credits) {
  const startMs = actualWithTimes.length > 0 ? actualWithTimes[0].startMs : 0
  const endMs = actualWithTimes.length > 0 ? actualWithTimes[0].startMs : 3000
  srt += `${subtitleIndex++}\n`
  srt += `${formatTime(startMs)} --> ${formatTime(endMs)}\n`
  srt += `${credits}\n\n`
}

// Add actual lyrics subtitles
for (const item of actualWithTimes) {
  srt += `${subtitleIndex++}\n`
  srt += `${formatTime(item.startMs)} --> ${formatTime(item.endMs)}\n`
  srt += `${item.text}\n\n`
}

srt = srt.trimEnd()

// ----- Write output -----
fs.writeFileSync(path.join(scriptDir, 'continuous.srt'), srt)
console.log('SRT file generated: continuous.srt')
