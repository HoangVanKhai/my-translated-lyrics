#! /usr/bin/env node
// @ts-check

const fs = require('fs')
const path = require('path')

/**
 * @typedef {'LTY' | 'lty' | 'YZL' | 'yzl' | 'Y+L' | 'txt' | 'ttl' | 'cre' | 'clr' | 'eov'} Marker
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
  const minutes = Math.floor(ms / 60000)
  const seconds = Math.floor((ms % 60000) / 1000)
  const milliseconds = ms % 1000
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}.${String(milliseconds).padStart(3, '0')}`
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
 * Format a single line of credit text into <c.creditRole> and <c.creditName>.
 * @param {string} line - A single line from the credit block
 * @param {LanguageCode} languageCode - Language code
 * @returns {string} Formatted line
 */
function formatCreditLine(line, languageCode) {
  const separator = languageCode === 'zh' ? /\u3000+/ : / {2,}/
  const parts = line.split(separator).filter(p => p.trim() !== '')

  const formattedParts = parts.map(part => {
    const colonMatch = part.match(/^([^:：]+)[:：](.+)$/)
    if (colonMatch) {
      const role = colonMatch[1].trim()
      const name = colonMatch[2].trim()
      return `<c.creditRole>${role}</c> <c.creditName>${name}</c>`
    }
    return part
  })

  return formattedParts.join(' ')
}

/**
 * Format text for VTT output with special handling for credits and titles.
 * Preserves newlines by processing each line individually.
 * @param {Marker | undefined} marker - The marker
 * @param {string} text - Raw text (may contain newlines)
 * @param {LanguageCode} languageCode - Language code
 * @returns {string} Formatted text with <c> tags if applicable
 */
function formatVttText(marker, text, languageCode) {
  if (marker === 'ttl') {
    return `<c.title>${text}</c.title>`
  }

  if (marker === 'cre') {
    const lines = text.split('\n')
    const formattedLines = lines.map(line => formatCreditLine(line, languageCode))
    return formattedLines.join('\n')
  }

  return text
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
 * @param {Partial<Record<Marker, string>>} speakerMap - Mapping from marker to speaker name
 * @param {LanguageCode} languageCode - Language code
 * @param {Partial<Record<Marker, string>>} colorMap - Mapping from marker to CSS color
 * @returns {string} VTT formatted string
 */
function generateVtt(cues, speakerMap, languageCode, colorMap) {
  let vtt = ''

  vtt = 'WEBVTT\n'
  vtt += `Language: ${languageCode}\n`

  vtt += '\n'

  vtt += 'STYLE\n'

  vtt += '::cue {\n'
  vtt += '  background-color: transparent;\n'
  vtt += '  text-shadow: 2px 2px 2px black;\n'
  vtt += '}\n'

  for (const [marker, color] of Object.entries(colorMap)) {
    const speakerName = speakerMap[/** @type {Marker} */ (marker)]
    if (speakerName) {
      vtt += `::cue(v[voice="${speakerName}"]) {\n`
      vtt += `  color: ${color};\n`
      vtt += '}\n'
    }
  }

  vtt += '::cue(c.creditRole) {\n'
  vtt += '  color: #AAAA22;\n'
  vtt += '}\n'

  vtt += '::cue(c.creditName) {\n'
  vtt += '  color: #AAAAAA;\n'
  vtt += '}\n'

  vtt += '::cue(c.title) {\n'
  vtt += '  color: #FFD966;\n'
  vtt += '  font-weight: bold;\n'
  vtt += '}\n'

  vtt += '\n'

  for (const cue of cues) {
    vtt += `${msToVttTime(cue.startMs)} --> ${msToVttTime(cue.endMs)}\n`

    const formattedText = formatVttText(cue.marker, cue.text, languageCode)

    const speaker = cue.marker ? speakerMap[cue.marker] : undefined
    if (speaker) {
      vtt += `<v ${speaker}>${formattedText}</v>\n\n`
    } else {
      vtt += `${formattedText}\n\n`
    }
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

  /** @type {Partial<Record<Marker, string>>} */
  const zhSpeakerMap = {
    'LTY': '洛天依',
    'lty': '洛天依',
    'YZL': '乐正绫',
    'yzl': '乐正绫',
    'Y+L': '洛天依 & 乐正绫',
  }

  /** @type {Partial<Record<Marker, string>>} */
  const viSpeakerMap = {
    'LTY': 'Lạc Thiên Y',
    'lty': 'Lạc Thiên Y',
    'YZL': 'Nhạc Chính Lăng',
    'yzl': 'Nhạc Chính Lăng',
    'Y+L': 'Lạc Thiên Y & Nhạc Chính Lăng',
  }

  /** @type {Partial<Record<Marker, string>>} */
  const colorMap = {
    'LTY': '#66CCFF',
    'lty': '#66CCFF',
    'YZL': '#EE0000',
    'yzl': '#EE0000',
    'Y+L': '#9966CC',
  }

  fs.writeFileSync(path.join(baseDir, 'lyrics.zh.srt'), generateSrt(zhCues), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.zh.vtt'), generateVtt(zhCues, zhSpeakerMap, 'zh', colorMap), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.vi.mtl.srt'), generateSrt(viCues), 'utf8')
  fs.writeFileSync(path.join(baseDir, 'lyrics.vi.mtl.vtt'), generateVtt(viCues, viSpeakerMap, 'vi', colorMap), 'utf8')

  console.log('Subtitle files generated successfully.')
}

main()
