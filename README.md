# NeutrinoTau Extension Scaffold

## Build

```
msbuild
```

`msbuild` 実行時に `rust/Cargo.toml` の `cdylib` がビルドされ、`csbindgen` により C# バインディングが生成されます。

## Package

1. `Extensions/NeutrinoTau` の中に `description.json` と `NeutrinoTau.dll` を配置
2. zip 化して拡張子を `.tlx` に変更

## Runtime Config

`config.json` を拡張フォルダに配置し、`neutrinoPath` を設定してください。

```json
{
  "neutrinoPath": "C:\\path\\to\\NEUTRINO"
}
```
