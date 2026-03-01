# Neutrino Tau for TuneLab

> [!WARNING]
> ボカコレに間に合わせるために突貫工事で作ったものなのでだいぶ雑だし動くことは保証できません。あくまで参考程度にしてください。

Neutrino Tau for TuneLab は [TuneLab](https://github.com/LiuYunPlayer/TuneLab) で Neutrino を使用するためのプラグインです。

## インストール

1. [Neutrino](https://studio-neutrino.com/) の最新版をダウンロードしてインストールします。
2. [TuneLab](https://github.com/LiuYunPlayer/TuneLab) の最新版をダウンロードしてインストールします。
3. [Releases](https://github.com/sevenc-nanashi/tunelab-neutrino-tau/releases) から最新の `tunelab-neutrino-tau-x.x.x.tlx` をダウンロードします。
4. `Extensions`から`Install/Update...`からダウンロードした `.tlx` ファイルを選択してインストールします。

## パートのプロパティ

### `styleShift`

- 型: number (integer)
- 既定値: `0`
- 範囲: `-24` 〜 `24` (半音)
- 小数で指定された場合は四捨五入されます。

`styleShift` は内部の推論時にノートを半音単位でシフトして音色傾向を変えるためのパラメータです。  
ピッチ推論に適用され、最終波形合成の基準キーにも反映されます。

### `waveformStyleShift`

- 型: number (integer)
- 既定値: `0`
- 範囲: `-24` 〜 `24` (半音)
- 小数で指定された場合は四捨五入されます。

`waveformStyleShift` は最終波形合成にのみ追加で適用される半音シフトです。  
`styleShift` に重ねて波形段でだけシフト量を調整できます。
