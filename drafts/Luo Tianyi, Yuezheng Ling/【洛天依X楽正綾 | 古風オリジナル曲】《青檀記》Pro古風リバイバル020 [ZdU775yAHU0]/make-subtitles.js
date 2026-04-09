#! /usr/bin/env node
// @ts-check

const fs = require('fs')
const path = require('path')

/**
 * @typedef {Object} SubtitleCue
 * @property {number} startMs - Start time in milliseconds
 * @property {number} endMs - End time in milliseconds
 * @property {string} text - Subtitle text
 * @property {string} [marker] - Optional marker (e.g., 'LTY', 'txt')
 */

/**
 * @typedef {Object} CueEvent
 * @property {'cue'} type
 * @property {number} startMs
 * @property {string} text
 * @property {string} [marker]
 */

/**
 * @typedef {Object} ClearEvent
 * @property {'clr'} type
 * @property {number} startMs
 */

/**
 * @typedef {CueEvent | ClearEvent} ParsedEvent
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
 * Parse lyrics file content into an array of subtitle cues,
 * respecting `clr` markers to end the previous cue at the clear time.
 * Handles multi‑line cues where continuation lines lack timestamps.
 * @param {string} content - Raw file content
 * @returns {SubtitleCue[]} Array of parsed cues with correct start/end times
 */
function parseLyrics(content) {
  const lines = content.split('\n')
  /** @type {ParsedEvent[]} */
  const events = []

  /** @type {CueEvent | null} */
  let currentCue = null

  for (const line of lines) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue

    // Match timestamp at beginning: MM:SS.mmm
    const match = trimmed.match(/^(\d{2}:\d{2}\.\d{3})\s+(.+)$/)
    if (match) {
      const timestamp = match[1]
      const rest = match[2].trim()
      const startMs = timestampToMs(timestamp)

      // Ignore end-of-video markers (no subtitle content)
      if (rest.startsWith('eov')) {
        currentCue = null
        continue
      }

      // Clear screen event
      if (rest.startsWith('clr')) {
        events.push({ type: 'clr', startMs })
        currentCue = null
        continue
      }

      // Extract marker and text
      let marker = ''
      let text = ''
      const colonIndex = rest.indexOf(':')
      if (colonIndex !== -1) {
        marker = rest.substring(0, colonIndex).trim()
        text = rest.substring(colonIndex + 1).trim()
      } else {
        text = rest
      }

      // Skip if text is empty
      if (!text) {
        currentCue = null
        continue
      }

      /** @type {CueEvent} */
      const cueEvent = { type: 'cue', startMs, text, marker }
      events.push(cueEvent)
      currentCue = cueEvent
    } else {
      // No timestamp → continuation line for the current cue
      if (currentCue) {
        currentCue.text += '\n' + trimmed
      }
      // If no currentCue, ignore (e.g., stray text before any cue)
    }
  }

  // Sort events by start time (they should already be ordered, but ensure)
  events.sort((a, b) => a.startMs - b.startMs)

  /** @type {SubtitleCue[]} */
  const cues = []

  for (let i = 0; i < events.length; i++) {
    const event = events[i]
    if (event.type !== 'cue') continue

    const cue = {
      startMs: event.startMs,
      text: event.text,
      endMs: 0,
      marker: event.marker,
    }

    // Find the next event (cue or clr) to determine end time
    let nextEventIndex = i + 1
    while (nextEventIndex < events.length && events[nextEventIndex].type === 'clr') {
      // If the next event is a clr, that's where this cue should end
      cue.endMs = events[nextEventIndex].startMs
      nextEventIndex++
      break // stop at the first clr after this cue
    }

    if (cue.endMs === 0) {
      // No clr found before the next cue – use the next cue's start time
      if (nextEventIndex < events.length && events[nextEventIndex].type === 'cue') {
        cue.endMs = events[nextEventIndex].startMs
      } else {
        // This is the last cue and no trailing clr – default 3 seconds
        cue.endMs = cue.startMs + 3000
      }
    }

    cues.push(cue)
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
 * Generate VTT content with voice tags, styling, and language header.
 * @param {SubtitleCue[]} cues - Array of subtitle cues
 * @param {Record<string, string>} speakerMap - Mapping from marker to speaker name
 * @param {string} languageCode - e.g., 'zh', 'vi'
 * @param {Record<string, string>} colorMap - Mapping from marker to CSS color
 * @returns {string} VTT formatted string
 */
function generateVtt(cues, speakerMap, languageCode, colorMap) {
  let vtt = `WEBVTT\nLanguage: ${languageCode}\n\n`

  vtt += 'STYLE\n'

  vtt += '::cue {\n'
  vtt += '  background-color: transparent;\n'
  vtt += '  text-shadow: 2px 2px 2px black;\n'
  vtt += '}\n'

  // Generate rules for each marker that has both a speaker name and a color
  for (const [marker, color] of Object.entries(colorMap)) {
    const speakerName = speakerMap[marker]
    if (speakerName) {
      vtt += `::cue(v[voice="${speakerName}"]) {\n  color: ${color};\n}\n`
    }
  }
  vtt += '\n'

  // Write cues
  cues.forEach(cue => {
    vtt += `${msToVttTime(cue.startMs)} --> ${msToVttTime(cue.endMs)}\n`
    const speaker = cue.marker ? speakerMap[cue.marker] : undefined
    if (speaker) {
      vtt += `<v ${speaker}>${cue.text}</v>\n\n`
    } else {
      vtt += `${cue.text}\n\n`
    }
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

  const zhSpeakerMap = {
    'LTY': '洛天依',
    'lty': '洛天依',
    'YZL': '乐正绫',
    'yzl': '乐正绫',
    'Y+L': '洛天依 & 乐正绫',
  }

  const viSpeakerMap = {
    'LTY': 'Lạc Thiên Y',
    'lty': 'Lạc Thiên Y',
    'YZL': 'Nhạc Chính Lăng',
    'yzl': 'Nhạc Chính Lăng',
    'Y+L': 'Lạc Thiên Y & Nhạc Chính Lăng',
  }

  const colorMap = {
    'LTY': '#66CCFF',
    'lty': '#66CCFF',
    'YZL': '#EE0000',
    'yzl': '#EE0000',
    'Y+L': '#9966CC',
  }

  // Write Chinese SRT and VTT
  fs.writeFileSync(path.join(baseDir, 'lyrics.zh.srt'), generateSrt(zhCues), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.zh.vtt'), generateVtt(zhCues, zhSpeakerMap, 'zh', colorMap), 'utf8')

  // Write Vietnamese SRT and VTT
  fs.writeFileSync(path.join(baseDir, 'lyrics.vi.mtl.srt'), generateSrt(viCues), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.vi.mtl.vtt'), generateVtt(viCues, viSpeakerMap, 'vi', colorMap), 'utf8')

  console.log('Subtitle files generated successfully.')
}

main()
