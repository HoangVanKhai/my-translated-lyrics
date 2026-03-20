#! /usr/bin/env node
const fs = require('fs')

// Read files
const text = fs.readFileSync('texts.txt', 'utf8')
const timeInfo = JSON.parse(fs.readFileSync('time-info.json', 'utf8'))

// Split text into segments (same method used in the REPL)
const segments = text.split(/\n{2,}/).filter(Boolean)

const starts = timeInfo.starts
const durations = timeInfo.durations

if (segments.length !== starts.length || segments.length !== durations.length) {
  console.error(`Mismatch: segments=${segments.length}, starts=${starts.length}, durations=${durations.length}`)
  process.exit(1)
}

// Convert [h,m,s,ms] to milliseconds
function toMs(h, m, s, ms) {
  return (parseInt(h) * 3600 + parseInt(m) * 60 + parseInt(s)) * 1000 + parseInt(ms)
}

// Format time as HH:MM:SS,mmm
function formatTime(ms) {
  const hours = Math.floor(ms / 3600000)
  const minutes = Math.floor((ms % 3600000) / 60000)
  const seconds = Math.floor((ms % 60000) / 1000)
  const millis = ms % 1000
  return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${
    seconds.toString().padStart(2, '0')
  },${millis.toString().padStart(3, '0')}`
}

// Clean a segment: remove comment lines and struck‑through lines
function cleanSegment(segmentText) {
  const lines = segmentText.split('\n')
  const cleaned = lines.filter(line => {
    const trimmed = line.trim()
    // Remove comment lines (starting with #)
    if (trimmed.startsWith('#')) return false
    // Remove struck‑through lines (contain ~~...~~)
    if (trimmed.includes('~~')) return false
    return true
  })
  // Join with newline and trim extra whitespace
  return cleaned.join('\n').trim()
}

// Build SRT content
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

// Write output
fs.writeFileSync('output.srt', srt)
console.log('SRT file generated: output.srt')
