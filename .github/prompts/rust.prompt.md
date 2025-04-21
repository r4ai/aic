# Rust

これは Rust 言語でのみ意識すべきマニュアルです。

## 作業方針

- ユーザーからの作業の許可は必要ない。勝手にどんどん実行してよい。
- 作業は最後まであきらめずに続ける。

## 品質保証

コードに変更を加えた後は、必ず次のコマンドをすべて実行して、エラーが無いことを確認してください。

- `cargo fmt --all`
- `cargo clippy --fix --allow-dirty --allow-staged && cargo clippy -- -D warnings`
