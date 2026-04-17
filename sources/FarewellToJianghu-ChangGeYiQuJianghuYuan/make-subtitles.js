#! /usr/bin/env node
// @ts-check

const fs = require('fs')
const path = require('path')

/**
 * @typedef {'LRC' | 'ttl' | 'cre' | 'clr' | 'eov'} Marker
 */

/**
 * @typedef {'zh' | 'vi'} LanguageCode
 */

/**
 * @typedef {Object} SubtitleCue
 * @property {number} startMs - Start time in milliseconds
 * @property {number} endMs - End time in milliseconds
 * @property {string} text - Subtitle text
 * @property {Marker} [marker] - Optional marker
 */

/**
 * @typedef {Object} CueEvent
 * @property {'cue'} type
 * @property {number} startMs
 * @property {string} text
 * @property {Marker} [marker]
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
 * Format milliseconds as input timestamp "MM:SS.mmm"
 * @param {number} ms - Milliseconds
 * @returns {string} Formatted timestamp
 */
function msToInputTime(ms) {
  const minutes = Math.floor(ms / 60000).toString().padStart(2, '0')
  const seconds = Math.floor((ms % 60000) / 1000).toString().padStart(2, '0')
  const milliseconds = (ms % 1000).toString().padStart(3, '0')
  return `${minutes}:${seconds}.${milliseconds}`
}

/**
 * Format milliseconds as SRT timestamp "HH:MM:SS,mmm"
 * @param {number} ms - Milliseconds
 * @returns {string} SRT formatted timestamp
 */
function msToSrtTime(ms) {
  const hours = Math.floor(ms / 3600000).toString().padStart(2, '0')
  const minutes = Math.floor((ms % 3600000) / 60000).toString().padStart(2, '0')
  const seconds = Math.floor((ms % 60000) / 1000).toString().padStart(2, '0')
  const milliseconds = (ms % 1000).toString().padStart(3, '0')
  return `${hours}:${minutes}:${seconds},${milliseconds}`
}

/**
 * Format milliseconds as VTT timestamp "HH:MM:SS.mmm"
 * @param {number} ms - Milliseconds
 * @returns {string} VTT formatted timestamp
 */
function msToVttTime(ms) {
  const hours = Math.floor(ms / 3600000).toString().padStart(2, '0')
  const minutes = Math.floor((ms % 3600000) / 60000).toString().padStart(2, '0')
  const seconds = Math.floor((ms % 60000) / 1000).toString().padStart(2, '0')
  const milliseconds = (ms % 1000).toString().padStart(3, '0')
  return `${hours}:${minutes}:${seconds}.${milliseconds}`
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
      /** @type {Marker | undefined} */
      let marker
      let text = ''
      const colonIndex = rest.indexOf(':')
      if (colonIndex !== -1) {
        marker = /** @type {Marker} */ (rest.substring(0, colonIndex).trim())
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

  // Verify events are already ordered by start time
  for (let i = 1; i < events.length; i++) {
    if (events[i].startMs < events[i - 1].startMs) {
      throw new Error(`Events out of order at index ${i}: ${msToInputTime(events[i].startMs)} < ${msToInputTime(events[i - 1].startMs)}`)
    }
  }

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
        throw new Error(
          `Unable to determine end time for cue at ${msToInputTime(cue.startMs)}. ` +
          `Expected a following cue or 'clr' marker.`
        )
      }
    }

    cues.push(cue)
  }

  return cues
}

/**
 * Format a single line of credit text into VTT `<c>` tags.
 * The line uses the FarewellToJianghu convention: `role  value` where the
 * role is separated from the value by two spaces, and any `【...】` block
 * inside the value gets a special highlight class.
 * @param {string} line - A single line from the credit block
 * @returns {string} Formatted line
 */
function formatCreditLineVtt(line) {
  const [role, ...rest] = line.split('  ')
  const value = rest.join('  ')
  const styledValue = value.replaceAll(
    /【[^【】]*】/g,
    text => `<c.creditSpecial>${text}</c>`,
  )
  return `<c.creditRole>${role}</c>  <c.creditName>${styledValue}</c>`
}

/**
 * Format text for VTT output with special handling for credits and titles.
 * Preserves newlines by processing each line individually.
 * @param {Marker | undefined} marker - The marker
 * @param {string} text - Raw text (may contain newlines)
 * @returns {string} Formatted text with <c> tags if applicable
 */
function formatVttText(marker, text) {
  switch (marker) {
    case 'ttl':
      return `<c.title>${text}</c.title>`
    case 'cre': {
      const lines = text.split('\n')
      const formattedLines = lines.map(formatCreditLineVtt)
      return formattedLines.join('\n')
    }
    case 'LRC':
    case undefined:
      return text
    case 'clr':
    case 'eov':
      if (text.trim()) {
        throw new Error(`Unexpected text: ${marker} ${text}`)
      }
      return text
    default:
      /** @type {never} */
      const _unreachable = marker
      throw new Error(`Marker ${JSON.stringify(_unreachable)} is unaccounted for`)
  }
}

/**
 * Format a single line of credit text into SRT-compatible HTML tags.
 * @param {string} line - A single line from the credit block
 * @returns {string} Formatted line with <font> tags
 */
function formatCreditLineSrt(line) {
  const [role, ...rest] = line.split('  ')
  const value = rest.join('  ')
  const styledValue = value.replaceAll(
    /【[^【】]*】/g,
    text => `<font color="#55ABCD">${text}</font>`,
  )
  return `<font color="#AAAA22">${role}</font>  <font color="#AAAAAA">${styledValue}</font>`
}

/**
 * Format text for SRT output using HTML tags for styling.
 * @param {Marker | undefined} marker - The marker
 * @param {string} text - Raw text (may contain newlines)
 * @returns {string} Formatted text with HTML tags
 */
function formatSrtText(marker, text) {
  switch (marker) {
    case 'ttl':
      return `<b><font color="#FFD966">${text}</font></b>`
    case 'cre': {
      const lines = text.split('\n')
      const formattedLines = lines.map(formatCreditLineSrt)
      return formattedLines.join('\n')
    }
    case 'LRC':
    case undefined:
      return text
    case 'clr':
    case 'eov':
      if (text.trim()) {
        throw new Error(`Unexpected text: ${marker} ${text}`)
      }
      return text
    default: {
      /** @type {never} */
      const _unreachable = marker
      throw new Error(`Marker ${JSON.stringify(_unreachable)} is unaccounted for`)
    }
  }
}

/**
 * Generate SRT content from cues with styling.
 * @param {SubtitleCue[]} cues - Array of subtitle cues
 * @returns {string} SRT formatted string
 */
function generateSrt(cues) {
  let srt = ''
  cues.forEach((cue, index) => {
    srt += `${index + 1}\n`
    srt += `${msToSrtTime(cue.startMs)} --> ${msToSrtTime(cue.endMs)}\n`
    const styledText = formatSrtText(cue.marker, cue.text)
    srt += `${styledText}\n\n`
  })
  return srt.trim() + '\n'
}

/**
 * Generate VTT content with styling and language header.
 * @param {SubtitleCue[]} cues - Array of subtitle cues
 * @param {LanguageCode} languageCode - Language code
 * @returns {string} VTT formatted string
 */
function generateVtt(cues, languageCode) {
  let vtt = ''

  vtt = 'WEBVTT\n'
  vtt += `Language: ${languageCode}\n`

  vtt += '\n'

  vtt += 'STYLE\n'

  vtt += '::cue {\n'
  vtt += '  background-color: transparent;\n'
  vtt += '  text-shadow: 2px 2px 2px black;\n'
  vtt += '}\n'

  vtt += '::cue(c.creditRole) {\n'
  vtt += '  color: #AAAA22;\n'
  vtt += '}\n'

  vtt += '::cue(c.creditName) {\n'
  vtt += '  color: #AAAAAA;\n'
  vtt += '}\n'

  vtt += '::cue(c.creditSpecial) {\n'
  vtt += '  color: #55ABCD;\n'
  vtt += '}\n'

  vtt += '::cue(c.title) {\n'
  vtt += '  color: #FFD966;\n'
  vtt += '  font-weight: bold;\n'
  vtt += '}\n'

  vtt += '\n'

  for (const cue of cues) {
    vtt += `${msToVttTime(cue.startMs)} --> ${msToVttTime(cue.endMs)}\n`
    vtt += `${formatVttText(cue.marker, cue.text)}\n\n`
  }

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

  fs.writeFileSync(path.join(baseDir, 'lyrics.zh.srt'), generateSrt(zhCues), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.zh.vtt'), generateVtt(zhCues, 'zh'), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.vi.mtl.srt'), generateSrt(viCues), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.vi.mtl.vtt'), generateVtt(viCues, 'vi'), 'utf8')

  console.log('Subtitle files generated successfully.')
}

main()
