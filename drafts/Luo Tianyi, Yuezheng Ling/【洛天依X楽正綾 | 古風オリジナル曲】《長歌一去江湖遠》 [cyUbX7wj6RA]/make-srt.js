#! /usr/bin/env node
const fs = require('fs')
const path = require('path')
const timeInfo = require('./time-info.json')

const scriptDir = __dirname

const text = fs.readFileSync(path.join(scriptDir, 'texts.txt'), 'utf8')
const segments = text.split(/\n{2,}/).filter(Boolean)

const { starts, durations } = timeInfo

if (segments.length !== starts.length || segments.length !== durations.length) {
  console.error(`Mismatch: segments=${segments.length}, starts=${starts.length}, durations=${durations.length}`)
  process.exit(1)
}

/**
 * Convert time tuple `[hour, minute, second, millisecond]` to milliseconds.
 * @param {string} h - hours
 * @param {string} m - minutes
 * @param {string} s - seconds
 * @param {string} ms - milliseconds
 * @returns {number} total milliseconds
 */
function toMs(h, m, s, ms) {
  return (parseInt(h) * 3600 + parseInt(m) * 60 + parseInt(s)) * 1000 + parseInt(ms)
}

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
  return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${
    seconds.toString().padStart(2, '0')
  },${millis.toString().padStart(3, '0')}`
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

let srt = ''
for (let i = 0; i < segments.length; i++) {
  const startMs = toMs(...starts[i])
  const endMs = startMs + durations[i]
  const content = cleanSegment(segments[i])
  if (content === '') continue // skip empty segments after cleaning

  srt += `${i + 1}\n`
  srt += `${formatTime(startMs)} --> ${formatTime(endMs)}\n`
  srt += `${content}\n\n`
}
srt = srt.trimEnd()

fs.writeFileSync(path.join(scriptDir, 'output.srt'), srt)
console.log('SRT file generated: output.srt')
