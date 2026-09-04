# Neutrino Tau for TuneLab

[Neutrino Tau](https://studio-neutrino.com/)をTuneLabで使えるようにするための拡張機能。

## インストール

**調声支援ツール同梱版のNeutrinoが必要です。**
Neutrinoのパッケージをダウンロードして展開し、モデルをインストールした後、
拡張機能の設定でNeutrinoがインストールされているパスを「NEUTRINO Path」に設定してください。
例えば、`neutrino.exe` が `E:/tools/neutrino/bin/neutrino.exe` にある場合、"NEUTRINO Path" は `E:/tools/neutrino` になります。

## パートのプロパティ

### `styleShift`

- 型: number (integer)
- 既定値: `0`
- 範囲: `-24` 〜 `24` (半音)

`styleShift` は内部の推論時にノートを半音単位でシフトして音色傾向を変えるためのパラメータです。  
例えば`12`に設定されていた場合、`C4`のノートは`C5`としてピッチが推論されます。
最後の波形合成は`C4`を基準に行われます。（単純なピッチシフトでの補正）

### `waveformStyleShift`

- 型: number (integer)
- 既定値: `0`
- 範囲: `-24` 〜 `24` (半音)

`waveformStyleShift` は最終波形合成にのみ追加で適用される半音シフトです。  
`styleShift` に重ねて波形段でだけシフト量を調整できます。

### `pitchShiftCents`

- 型: number
- 既定値: `0`
- 範囲: `-2400` 〜 `2400` (cent)

`pitchShiftCents` は最終的な F0 にセント単位で適用されるピッチシフトです。  
`styleShift` / `waveformStyleShift` とは独立して、微細なキー調整に使えます。
