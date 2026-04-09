#! /usr/bin/env node
// @ts-check

const fs = require('fs')
const path = require('path')

/**
 * @typedef {Object} SubtitleCue
 * @property {number} startMs - Start time in milliseconds
 * @property {number} endMs - End time in milliseconds
 * @property {string} text - Subtitle text
 */

/**
 * Convert a timestamp string "MM:SS.mmm" to milliseconds
 * @param {string} timestamp - Timestamp in format MM:SS.mmm
 * @returns {number} Milliseconds
 */
function timestampToMs(timestamp) {
  const [minutes, seconds] = timestamp.split(':')
  const [sec, ms] = seconds.split('.')
  return parseInt(minutes, 10) * 60000 + parseInt(sec, 10) * 1000 + parseInt(ms.padEnd(3, '0'), 10)
}

/**
 * Format milliseconds as SRT timestamp "HH:MM:SS,mmm"
 * @param {number} ms - Milliseconds
 * @returns {string} SRT formatted timestamp
 */
function msToSrtTime(ms) {
  const hours = Math.floor(ms / 3600000)
  const minutes = Math.floor((ms % 3600000) / 60000)
  const seconds = Math.floor((ms % 60000) / 1000)
  const milliseconds = ms % 1000
  return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')},${
    String(milliseconds).padStart(3, '0')
  }`
}

/**
 * Format milliseconds as VTT timestamp "HH:MM:SS.mmm"
 * @param {number} ms - Milliseconds
 * @returns {string} VTT formatted timestamp
 */
function msToVttTime(ms) {
  const hours = Math.floor(ms / 3600000)
  const minutes = Math.floor((ms % 3600000) / 60000)
  const seconds = Math.floor((ms % 60000) / 1000)
  const milliseconds = ms % 1000
  return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}.${
    String(milliseconds).padStart(3, '0')
  }`
}

/**
 * Parse lyrics file content into an array of subtitle cues
 * @param {string} content - Raw file content
 * @returns {SubtitleCue[]} Array of parsed cues with start/end times and text
 */
function parseLyrics(content) {
  const lines = content.split('\n')
  /** @type {SubtitleCue[]} */
  const cues = []

  for (const line of lines) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue

    // Match timestamp at beginning: MM:SS.mmm
    const match = trimmed.match(/^(\d{2}:\d{2}\.\d{3})\s+(.+)$/)
    if (!match) continue

    const timestamp = match[1]
    const rest = match[2].trim()

    // Ignore clear-screen and end-of-video markers (no subtitle content)
    if (rest.startsWith('clr') || rest.startsWith('eov')) continue

    // Extract text after the marker colon, if any
    let text = ''
    const colonIndex = rest.indexOf(':')
    if (colonIndex !== -1) {
      text = rest.substring(colonIndex + 1).trim()
    } else {
      // Some lines might just be a marker without colon? Not present in sample.
      text = rest
    }

    // Skip if text is empty (e.g., "clr" alone)
    if (!text) continue

    cues.push({
      startMs: timestampToMs(timestamp),
      text: text,
      endMs: 0, // placeholder, will be set later
    })
  }

  // Sort cues by start time (they should be in order already, but ensure)
  cues.sort((a, b) => a.startMs - b.startMs)

  // Assign end times: use next cue's start time, or +3 seconds for the last one
  for (let i = 0; i < cues.length; i++) {
    if (i < cues.length - 1) {
      cues[i].endMs = cues[i + 1].startMs
    } else {
      cues[i].endMs = cues[i].startMs + 3000 // default 3 seconds
    }
  }

  return cues
}

/**
 * Generate SRT content from cues
 * @param {SubtitleCue[]} cues - Array of subtitle cues
 * @returns {string} SRT formatted string
 */
function generateSrt(cues) {
  let srt = ''
  cues.forEach((cue, index) => {
    srt += `${index + 1}\n`
    srt += `${msToSrtTime(cue.startMs)} --> ${msToSrtTime(cue.endMs)}\n`
    srt += `${cue.text}\n\n`
  })
  return srt.trim() + '\n'
}

/**
 * Generate VTT content from cues
 * @param {SubtitleCue[]} cues - Array of subtitle cues
 * @returns {string} VTT formatted string
 */
function generateVtt(cues) {
  let vtt = 'WEBVTT\n\n'
  cues.forEach(cue => {
    vtt += `${msToVttTime(cue.startMs)} --> ${msToVttTime(cue.endMs)}\n`
    vtt += `${cue.text}\n\n`
  })
  return vtt.trim() + '\n'
}

/**
 * Main entry point
 */
function main() {
  const baseDir = __dirname

  const zhInputPath = path.join(baseDir, 'lyrics.zh.txt')
  const viInputPath = path.join(baseDir, 'lyrics.vi.mtl.txt')

  if (!fs.existsSync(zhInputPath) || !fs.existsSync(viInputPath)) {
    console.error('Input files not found. Ensure lyrics.zh.txt and lyrics.vi.mtl.txt are in the script directory.')
    process.exit(1)
  }

  const zhContent = fs.readFileSync(zhInputPath, 'utf8')
  const viContent = fs.readFileSync(viInputPath, 'utf8')

  const zhCues = parseLyrics(zhContent)
  const viCues = parseLyrics(viContent)

  // Write Chinese SRT and VTT
  fs.writeFileSync(path.join(baseDir, 'lyrics.zh.srt'), generateSrt(zhCues), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.zh.vtt'), generateVtt(zhCues), 'utf8')

  // Write Vietnamese SRT and VTT
  fs.writeFileSync(path.join(baseDir, 'lyrics.vi.mtl.srt'), generateSrt(viCues), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.vi.mtl.vtt'), generateVtt(viCues), 'utf8')

  console.log('Subtitle files generated successfully.')
}

main()
