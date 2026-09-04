# Neutrino Tau for TuneLab

TuneLab Extension for [Neutrino Tau](https://studio-neutrino.com/).

## Installation

**You need the version of Neutrino that includes "調声支援ツール同梱版" ("with tuning support tool") to use this extension.**  
After downloading and extracting the Neutrino package, and installing models to use, set "NEUTRINO Path" in the extension settings to the path where Neutrino is installed.  
For example, if your `neutrino.exe` is located at `E:/tools/neutrino/bin/neutrino.exe`, set "NEUTRINO Path" to `E:/tools/neutrino`.

## Part Properties

### `styleShift`

- Type: number (integer)
- Default: `0`
- Range: `-24` to `24` (semitones)

`styleShift` is a parameter that shifts the notes in semitone units during internal inference to change the timbre tendency.
For example, if this value is set to `12` and the note is `C4`, the pitch inference will be performed as if the note is `C5`,
but the final waveform synthesis will still be based on `C4`. (with simply shifting the pitch)

### `waveformStyleShift`

- Type: number (integer)
- Default: `0`
- Range: `-24` to `24` (semitones)

`waveformStyleShift` is an additional semitone shift that is applied only to the score used for final waveform synthesis.

### `pitchShiftCents`

- Type: number
- Default: `0`
- Range: `-2400` to `2400` (cents)

`pitchShiftCents` is a pitch shift in cents that is applied to the final F0.
This value is independent of `styleShift` and `waveformStyleShift`, and is applied to the final F0 for fine-tuning the pitch.

