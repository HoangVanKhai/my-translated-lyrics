#! /usr/bin/env node
//@ts-check
const fs = require('fs')
const path = require('path')
const timeInfo = require('./time-info.json')

const scriptDir = __dirname

const text = fs.readFileSync(path.join(scriptDir, 'texts.txt'), 'utf8')
const segments = text.split(/\n{2,}/).filter(Boolean)

const { starts, durations } = timeInfo

if (segments.length !== starts.length || segments.length !== durations.length) {
  console.error(`Mismatch: segments=${segments.length}, starts=${starts.length}, durations=${durations.length}`)
  throw process.exit(1)
}

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
 * Remove the first line (which contains `[Credits]` or `[Title]`) and clean the rest.
 * @param {string} segmentText - raw segment text
 * @returns {string} cleaned text without the header
 */
const stripHeaderAndClean = segmentText => {
  const lines = segmentText.split('\n')
  // Remove the first line (the marker)
  const remaining = lines.slice(1).join('\n')
  return cleanSegment(remaining)
}

/**
 * Apply HTML styling to credits text.
 * @param {string} text - plain text
 * @returns {string} styled HTML
 */
const styleCredits = text => {
  // Use italic and a light gray color (common for credits)
  return `<i><font color="#AAAAAA">${text}</font></i>`
}

/**
 * Apply HTML styling to title text.
 * @param {string} text - plain text
 * @returns {string} styled HTML
 */
const styleTitle = text => {
  // Bold and slightly larger, with a gold-ish tone
  return `<b><font color="#FFD966">${text}</font></b>`
}

let srt = ''
for (let index = 0; index < segments.length; index++) {
  const rawSegment = segments[index]
  const [hr, min, sec, millis] = starts[index]
  const startMs = toMs(hr, min, sec, millis)
  const endMs = startMs + durations[index]

  let content = ''

  if (rawSegment.startsWith('[Credits]')) {
    const cleaned = stripHeaderAndClean(rawSegment)
    if (cleaned) {
      content = styleCredits(cleaned)
    }
  } else if (rawSegment.startsWith('[Title]')) {
    const cleaned = stripHeaderAndClean(rawSegment)
    if (cleaned) {
      content = styleTitle(cleaned)
    }
  } else {
    // Normal lyric segment
    content = cleanSegment(rawSegment)
  }

  if (!content.trim()) continue

  srt += `${index + 1}\n`
  srt += `${formatTime(startMs)} --> ${formatTime(endMs)}\n`
  srt += `${content}\n\n`
}
srt = srt.trimEnd()

fs.writeFileSync(path.join(scriptDir, 'discontinuous.srt'), srt)
console.log('SRT file generated: discontinuous.srt')
